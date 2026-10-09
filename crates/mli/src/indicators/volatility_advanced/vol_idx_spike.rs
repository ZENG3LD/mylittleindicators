//! VolIdxSpike — detects when volatility index exceeds 95th percentile of rolling history.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::VolatilityIndexConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::{Color, HistogramStyle, Render, RenderSpec};
use crate::core::types::VolatilityIndex;
use crate::engine::stream_kind::StreamKind;

/// Detects spikes in a volatility index by comparing to the 95th percentile of a rolling window.
///
/// If `current > p95(history)` → signal +1, otherwise 0.
///
/// Output: `Signal(i8)`.
#[derive(Debug, Clone)]
pub struct VolIdxSpike {
    period: usize,
    history: VecDeque<f64>,
    last_signal: i8,
}

impl VolIdxSpike {
    /// Create a new indicator. `period` is clamped to at least 3.
    pub fn new(period: usize) -> Self {
        let period = period.max(3);
        Self {
            period,
            history: VecDeque::with_capacity(period),
            last_signal: 0,
        }
    }

    fn compute_signal(&self, current: f64) -> i8 {
        let n = self.history.len();
        if n < 2 {
            return 0;
        }
        let mut sorted: Vec<f64> = self.history.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p95_idx = ((n as f64 * 0.95) as usize).min(n.saturating_sub(1));
        let p95 = sorted[p95_idx];
        if current > p95 { 1 } else { 0 }
    }
}

impl Default for VolIdxSpike {
    fn default() -> Self {
        Self::new(20)
    }
}

impl VolatilityIndexConsumer for VolIdxSpike {
    fn update_volatility_index(&mut self, vi: &VolatilityIndex) {
        let current = vi.value;
        // Compute signal against existing history before inserting new value
        self.last_signal = self.compute_signal(current);
        self.history.push_back(current);
        while self.history.len() > self.period {
            self.history.pop_front();
        }
    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= 2
    }
}

/// Typed configuration for [`VolIdxSpike`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VolIdxSpikeConfig {
    /// Rolling window size (clamped to ≥ 3).
    pub period: Param<usize>,
}

impl Indicator for VolIdxSpike {
    const ID: IndicatorId = IndicatorId::VolIdxSpike;
    /// Spike detector — not a pluggable volatility measure, so family `&[]`.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::VolatilityIndex];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::VolIdxSpike)];
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);
    type Config = VolIdxSpikeConfig;
    type Runtime = VolIdxSpike;

    fn create(cfg: VolIdxSpikeConfig) -> VolIdxSpike {
        VolIdxSpike::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for VolIdxSpikeConfig {
    fn defaults() -> Self {
        VolIdxSpikeConfig { period: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // period: Class A (rolling p95 lookback) → auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VolIdxSpike {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(crate::contract::RenderOutput::histogram(
                IndicatorOutputId::VolIdxSpike,
                "Vol Index Spike",
                Color::hex(0xFF5722),
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

    fn make_vi(v: f64) -> VolatilityIndex {
        VolatilityIndex { value: v, timestamp: 0, ..Default::default()}
    }

    #[test]
    fn spike_above_p95() {
        let mut ind = VolIdxSpike::new(20);
        for i in 0..20 {
            ind.update_volatility_index(&make_vi(i as f64));
        }
        ind.update_volatility_index(&make_vi(1000.0));
        let s = ind.value() as i8;
        assert_eq!(s, 1, "should be +1 for spike above p95");
    }

    #[test]
    fn no_spike_below_p95() {
        let mut ind = VolIdxSpike::new(20);
        for i in 0..20 {
            ind.update_volatility_index(&make_vi(i as f64));
        }
        // Median value should not trigger
        ind.update_volatility_index(&make_vi(9.0));
        let s = ind.value() as i8;
        assert_eq!(s, 0, "should be 0 for median value");
    }

    #[test]
    fn reset_clears() {
        let mut ind = VolIdxSpike::new(5);
        for i in 0..5 {
            ind.update_volatility_index(&make_vi(i as f64));
        }
        ind.reset();
        assert!(!ind.is_ready());
        let s = ind.value() as i8;
        assert_eq!(s, 0);
    }

    #[test]
    fn factory_feeds_resolved_vol_idx_spike() {
        use crate::engine::contract_engine::{IndicatorOrder, IndicatorOutputId};
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::VolIdxSpike(VolIdxSpikeConfig { period: Param::Solo(20) })
            .build_solo()
            .unwrap();
        for i in 0..20 {
            f.feed(0, MarketSample::VolatilityIndex(&make_vi(i as f64)));
        }
        // Spike well above any prior value
        f.feed(0, MarketSample::VolatilityIndex(&make_vi(1000.0)));
        let s = f.read(IndicatorOutputId::VolIdxSpike) as i8;
        assert_eq!(s, 1, "expected spike signal");
    }
}

impl VolIdxSpike {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }
}
