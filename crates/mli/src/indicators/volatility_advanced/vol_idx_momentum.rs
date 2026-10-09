//! VolIdxMomentum — rolling linear slope of volatility index values.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::VolatilityIndexConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::core::types::VolatilityIndex;
use crate::engine::stream_kind::StreamKind;

/// Computes the linear slope of the volatility index over the last `period` snapshots.
///
/// slope = (latest − oldest) / (n − 1)
///
/// Output: `Single(slope)`. Returns 0.0 until at least two snapshots.
#[derive(Debug, Clone)]
pub struct VolIdxMomentum {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl VolIdxMomentum {
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

impl Default for VolIdxMomentum {
    fn default() -> Self {
        Self::new(14)
    }
}

impl VolatilityIndexConsumer for VolIdxMomentum {
    fn update_volatility_index(&mut self, vi: &VolatilityIndex) {
        self.history.push_back(vi.value);
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

/// Typed configuration for [`VolIdxMomentum`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VolIdxMomentumConfig {
    /// Rolling window of vol-index snapshots (clamped to ≥ 2).
    pub period: Param<usize>,
}

impl Indicator for VolIdxMomentum {
    const ID: IndicatorId = IndicatorId::VolIdxMomentum;
    /// Measures the direction of the volatility index regime — fits `Volatility` family.
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::VolatilityIndex];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::VolIdxMomentum)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    type Config = VolIdxMomentumConfig;
    type Runtime = VolIdxMomentum;

    fn create(cfg: VolIdxMomentumConfig) -> VolIdxMomentum {
        VolIdxMomentum::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for VolIdxMomentumConfig {
    fn defaults() -> Self {
        VolIdxMomentumConfig { period: Param::Solo(14) }
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


impl Render for VolIdxMomentum {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::VolIdxMomentum, "Vol Index Momentum", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(4)
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
    fn rising_vol_index_positive_slope() {
        let mut ind = VolIdxMomentum::new(5);
        for v in [10.0, 20.0, 30.0, 40.0, 50.0] {
            ind.update_volatility_index(&make_vi(v));
        }
        let s = ind.value();
        assert!(s > 0.0, "slope should be positive, got {s}");
    }

    #[test]
    fn falling_vol_index_negative_slope() {
        let mut ind = VolIdxMomentum::new(5);
        for v in [50.0, 40.0, 30.0, 20.0, 10.0] {
            ind.update_volatility_index(&make_vi(v));
        }
        let s = ind.value();
        assert!(s < 0.0, "slope should be negative, got {s}");
    }

    #[test]
    fn reset_clears() {
        let mut ind = VolIdxMomentum::new(3);
        ind.update_volatility_index(&make_vi(10.0));
        ind.update_volatility_index(&make_vi(20.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_vol_idx_momentum() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::VolIdxMomentum(VolIdxMomentumConfig { period: Param::Solo(3) })
            .build_solo()
            .unwrap();
        for v in [10.0, 20.0, 30.0] {
            f.feed(0, MarketSample::VolatilityIndex(&make_vi(v)));
        }
        let s = f.read(IndicatorOutputId::VolIdxMomentum);
        assert!(s > 0.0, "expected positive slope, got {s}");
    }
}

impl VolIdxMomentum {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_slope
    }
}
