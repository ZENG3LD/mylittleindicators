//! LongShortRatioMomentum — rolling slope of long_ratio over a window.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::long_short_ratio_consumer::LongShortRatioConsumer;
use crate::contract::render::Render;
use crate::contract::{Color, Cost, Family, Indicator, Output, Param, RenderSpec, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::core::types::LongShortRatio;
use crate::engine::stream_kind::StreamKind;

/// Computes the linear slope of `long_ratio` over the last `period` snapshots.
///
/// slope = (latest − oldest) / (n − 1)
///
/// Output: `slope`.
#[derive(Clone, Debug)]
pub struct LongShortRatioMomentum {
    period: usize,
    history: VecDeque<f64>,
    last_slope: f64,
}

impl LongShortRatioMomentum {
    /// Create a new indicator. `period` is clamped to at least 2.
    pub fn new(period: usize) -> Self {
        let period = period.max(2);
        Self {
            period,
            history: VecDeque::with_capacity(period),
            last_slope: 0.0,
        }
    }
}

/// Typed dual-mode config for [`LongShortRatioMomentum`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LongShortRatioMomentumConfig {
    pub period: Param<usize>,
}

impl Indicator for LongShortRatioMomentum {
    const ID: IndicatorId = IndicatorId::LongShortRatioMomentum;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::LongShortRatio];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::LongShortRatioMomentum)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    type Config = LongShortRatioMomentumConfig;
    type Runtime = LongShortRatioMomentum;

    fn create(cfg: LongShortRatioMomentumConfig) -> LongShortRatioMomentum {
        LongShortRatioMomentum::new(cfg.period.resolved())
    }
}

impl LongShortRatioConsumer for LongShortRatioMomentum {
    fn update_long_short_ratio(&mut self, lsr: &LongShortRatio) {
        self.history.push_back(lsr.long_ratio);
        while self.history.len() > self.period {
            self.history.pop_front();
        }
        if self.history.len() >= 2 {
            let oldest = self.history[0];
            let latest = self.history[self.history.len() - 1];
            self.last_slope = (latest - oldest) / (self.history.len() as f64 - 1.0);
        }
    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_slope = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= 2
    }
}

impl Default for LongShortRatioMomentum {
    /// Factory default: period=20.
    fn default() -> Self {
        Self::new(20)
    }
}

impl crate::contract::Config for LongShortRatioMomentumConfig {
    fn defaults() -> Self {
        LongShortRatioMomentumConfig { period: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // period is Class A (lookback period) — auto provides range(2,4048,1).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for LongShortRatioMomentum {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(
                IndicatorOutputId::LongShortRatioMomentum,
                "L/S Ratio Momentum",
                Color::hex(0xAB47BC),
            )
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;

    fn make_lsr(long_ratio: f64) -> LongShortRatio {
        LongShortRatio {
            symbol: String::new(),
            ratio_type: "global_account".to_string(),
            long_ratio,
            short_ratio: 1.0 - long_ratio,
            ratio: if (1.0 - long_ratio).abs() > 1e-12 {
                Some(long_ratio / (1.0 - long_ratio))
            } else {
                None
            },
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn rising_ratio_gives_positive_slope() {
        let mut ind = LongShortRatioMomentum::new(5);
        for v in [0.4, 0.5, 0.6, 0.7, 0.8] {
            ind.update_long_short_ratio(&make_lsr(v));
        }
        let slope = ind.value();
        assert!(slope > 0.0, "slope should be positive, got {slope}");
    }

    #[test]
    fn falling_ratio_gives_negative_slope() {
        let mut ind = LongShortRatioMomentum::new(5);
        for v in [0.8, 0.7, 0.6, 0.5, 0.4] {
            ind.update_long_short_ratio(&make_lsr(v));
        }
        let slope = ind.value();
        assert!(slope < 0.0, "slope should be negative, got {slope}");
    }

    #[test]
    fn not_ready_until_two_samples() {
        let mut ind = LongShortRatioMomentum::new(5);
        assert!(!ind.is_ready());
        ind.update_long_short_ratio(&make_lsr(0.5));
        assert!(!ind.is_ready());
        ind.update_long_short_ratio(&make_lsr(0.6));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = LongShortRatioMomentum::new(3);
        ind.update_long_short_ratio(&make_lsr(0.4));
        ind.update_long_short_ratio(&make_lsr(0.8));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_long_short_ratio_momentum() {
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::LongShortRatioMomentum(
            <<LongShortRatioMomentum as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // feed rising series — slope should go positive
        for v in [0.4, 0.5, 0.6] {
            f.feed(0, MarketSample::LongShortRatio(&make_lsr(v)));
        }
        let slope = f.primary();
        assert!(slope > 0.0, "slope should be positive, got {slope}");
    }
}

impl LongShortRatioMomentum {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_slope
    }
}
