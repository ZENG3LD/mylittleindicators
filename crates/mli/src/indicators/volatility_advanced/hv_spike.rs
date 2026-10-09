//! HvSpike — detects when HV exceeds N times its rolling mean.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::HistoricalVolatilityConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::axis::sweep_f64;
use crate::contract::{Color, HistogramStyle, Render, RenderSpec};
use crate::core::types::HistoricalVolatility;
use crate::engine::stream_kind::StreamKind;

/// Detects spikes in historical volatility by comparing current value to a rolling mean.
///
/// If `current_hv > multiplier * rolling_mean` → signal +1, otherwise 0.
///
/// Output: `Signal(i8)`.
#[derive(Debug, Clone)]
pub struct HvSpike {
    period: usize,
    multiplier: f64,
    history: VecDeque<f64>,
    last_signal: i8,
}

impl HvSpike {
    /// Create a new indicator.
    /// - `period`: rolling window for mean computation (clamped to at least 2)
    /// - `multiplier`: spike threshold factor (default 2.0)
    pub fn new(period: usize, multiplier: f64) -> Self {
        let period = period.max(2);
        Self {
            period,
            multiplier,
            history: VecDeque::with_capacity(period),
            last_signal: 0,
        }
    }

    fn compute_signal(&self, current: f64) -> i8 {
        let n = self.history.len();
        if n < 2 {
            return 0;
        }
        let mean = self.history.iter().sum::<f64>() / n as f64;
        if mean > 1e-12 && current > self.multiplier * mean {
            1
        } else {
            0
        }
    }
}

impl Default for HvSpike {
    fn default() -> Self {
        Self::new(20, 2.0)
    }
}

impl HistoricalVolatilityConsumer for HvSpike {
    fn update_historical_volatility(&mut self, hv: &HistoricalVolatility) {
        let current = hv.volatility;
        self.history.push_back(current);
        while self.history.len() > self.period {
            self.history.pop_front();
        }
        self.last_signal = self.compute_signal(current);
    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= 2
    }
}

/// Typed configuration for [`HvSpike`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct HvSpikeConfig {
    /// Rolling window size (clamped to ≥ 2).
    pub period: Param<usize>,
    /// Threshold: signal when current > multiplier × rolling_mean.
    pub multiplier: Param<f64>,
}

impl Indicator for HvSpike {
    const ID: IndicatorId = IndicatorId::HvSpike;
    /// Spike detector — not a pluggable volatility measure, so family `&[]`.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::HistoricalVolatility];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::HvSpike)];
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);
    type Config = HvSpikeConfig;
    type Runtime = HvSpike;

    fn create(cfg: HvSpikeConfig) -> HvSpike {
        HvSpike::new(cfg.period.resolved(), cfg.multiplier.resolved())
    }
}

impl crate::contract::Config for HvSpikeConfig {
    fn defaults() -> Self {
        HvSpikeConfig {
            period: Param::Solo(20),
            multiplier: Param::Solo(2.0),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // period: Class A (rolling mean lookback) → auto range(2,4048,1) is correct.
        // multiplier: Class C (spike threshold factor, scales the rolling mean) →
        //   sweep_f64(0.1,10.0,0.1).
        s.multiplier = Param::many(sweep_f64(0.1, 10.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for HvSpike {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(crate::contract::RenderOutput::histogram(
                IndicatorOutputId::HvSpike,
                "HV Spike",
                Color::hex(0xF44336),
            ))
            .bounds(0.0, 1.0)
            .histogram_style(HistogramStyle::FromBottom)
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hv(v: f64) -> HistoricalVolatility {
        HistoricalVolatility { volatility: v, timestamp: 0 }
    }

    #[test]
    fn spike_detected_above_threshold() {
        let mut ind = HvSpike::new(5, 2.0);
        // Fill with baseline around 0.1
        for _ in 0..5 {
            ind.update_historical_volatility(&make_hv(0.1));
        }
        // Push spike: 0.1 * 2.0 = 0.2 threshold; current = 0.5 > 0.2
        ind.update_historical_volatility(&make_hv(0.5));
        let s = ind.value() as i8;
        assert_eq!(s, 1, "should detect spike");
    }

    #[test]
    fn no_spike_below_threshold() {
        let mut ind = HvSpike::new(5, 2.0);
        for _ in 0..5 {
            ind.update_historical_volatility(&make_hv(0.1));
        }
        ind.update_historical_volatility(&make_hv(0.15));
        let s = ind.value() as i8;
        assert_eq!(s, 0, "should not detect spike");
    }

    #[test]
    fn reset_clears() {
        let mut ind = HvSpike::new(5, 2.0);
        for _ in 0..5 {
            ind.update_historical_volatility(&make_hv(0.1));
        }
        ind.reset();
        assert!(!ind.is_ready());
        let s = ind.value() as i8;
        assert_eq!(s, 0);
    }

    #[test]
    fn factory_feeds_resolved_hv_spike() {
        use crate::engine::contract_engine::{IndicatorOrder, IndicatorOutputId};
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::HvSpike(HvSpikeConfig { period: Param::Solo(5), multiplier: Param::Solo(2.0) })
            .build_solo()
            .unwrap();
        // Baseline
        for _ in 0..5 {
            f.feed(0, MarketSample::HistoricalVolatility(&make_hv(0.1)));
        }
        // Spike value well above threshold
        f.feed(0, MarketSample::HistoricalVolatility(&make_hv(0.5)));
        let s = f.read(IndicatorOutputId::HvSpike) as i8;
        assert_eq!(s, 1, "expected spike signal");
    }
}

impl HvSpike {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }
}
