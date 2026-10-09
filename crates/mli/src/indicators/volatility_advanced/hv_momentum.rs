//! HvMomentum — rolling linear slope of historical volatility values.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::HistoricalVolatilityConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::core::types::HistoricalVolatility;
use crate::engine::stream_kind::StreamKind;

/// Computes the linear slope of historical volatility over the last `period` snapshots.
///
/// slope = (latest − oldest) / (n − 1)
///
/// Output: `Single(slope)`. Returns 0.0 until at least two snapshots.
#[derive(Debug, Clone)]
pub struct HvMomentum {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl HvMomentum {
    /// Create a new indicator. `period` is clamped to at least 2.
    pub fn new(period: usize) -> Self {
        let period = period.max(2);
        Self {
            period,
            history: VecDeque::with_capacity(period),
            last_slope: 0.0,
        }
    }

    fn compute_slope(&self) -> f64 {
        let n = self.history.len();
        if n < 2 {
            return 0.0;
        }
        (self.history[n - 1] - self.history[0]) / (n as f64 - 1.0)
    }
}

impl Default for HvMomentum {
    fn default() -> Self {
        Self::new(14)
    }
}

impl HistoricalVolatilityConsumer for HvMomentum {
    fn update_historical_volatility(&mut self, hv: &HistoricalVolatility) {
        self.history.push_back(hv.volatility);
        while self.history.len() > self.period {
            self.history.pop_front();
        }
        self.last_slope = self.compute_slope();
    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_slope = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= 2
    }
}

/// Typed configuration for [`HvMomentum`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct HvMomentumConfig {
    /// Rolling window of HV snapshots (clamped to ≥ 2).
    pub period: Param<usize>,
}

impl Indicator for HvMomentum {
    const ID: IndicatorId = IndicatorId::HvMomentum;
    /// Measures the direction of the volatility regime — fits `Volatility` family.
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::HistoricalVolatility];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::HvMomentum)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    type Config = HvMomentumConfig;
    type Runtime = HvMomentum;

    fn create(cfg: HvMomentumConfig) -> HvMomentum {
        HvMomentum::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for HvMomentumConfig {
    fn defaults() -> Self {
        HvMomentumConfig { period: Param::Solo(14) }
    }
    fn machine_defaults() -> Self {
        // period: Class A (rolling slope lookback) → auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for HvMomentum {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::HvMomentum, "HV Momentum", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(4)
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
    fn rising_hv_positive_slope() {
        let mut ind = HvMomentum::new(5);
        for v in [0.1, 0.2, 0.3, 0.4, 0.5] {
            ind.update_historical_volatility(&make_hv(v));
        }
        let s = ind.value();
        assert!(s > 0.0, "slope should be positive, got {s}");
    }

    #[test]
    fn falling_hv_negative_slope() {
        let mut ind = HvMomentum::new(5);
        for v in [0.5, 0.4, 0.3, 0.2, 0.1] {
            ind.update_historical_volatility(&make_hv(v));
        }
        let s = ind.value();
        assert!(s < 0.0, "slope should be negative, got {s}");
    }

    #[test]
    fn reset_clears() {
        let mut ind = HvMomentum::new(3);
        ind.update_historical_volatility(&make_hv(0.1));
        ind.update_historical_volatility(&make_hv(0.2));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_hv_momentum() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::HvMomentum(HvMomentumConfig { period: Param::Solo(3) })
            .build_solo()
            .unwrap();
        for v in [0.10, 0.20, 0.30] {
            f.feed(0, MarketSample::HistoricalVolatility(&make_hv(v)));
        }
        let s = f.read(IndicatorOutputId::HvMomentum);
        assert!(s > 0.0, "expected positive slope, got {s}");
    }
}

impl HvMomentum {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_slope
    }
}
