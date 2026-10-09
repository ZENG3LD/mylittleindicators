//! LeverageReductionWarning — detects when exchange tightens leverage limits.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::risk_limit_consumer::RiskLimitConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::RiskLimit;

/// Detects exchange leverage tightening (bearish signal) or loosening (bullish).
///
/// - `+1` when new `max_leverage` < previous (exchange tightening)
/// - `-1` when new `max_leverage` > previous (exchange loosening)
/// - `0`  when unchanged or on first update
///
/// Output: `Signal(i8)`.
#[derive(Debug, Clone)]
pub struct LeverageReductionWarning {
    prev_max_leverage: f64,
    last_signal: i8,
}

impl LeverageReductionWarning {
    /// Create a new indicator with no prior state.
    pub fn new() -> Self {
        Self {
            prev_max_leverage: f64::NAN,
            last_signal: 0,
        }
    }
}

impl Default for LeverageReductionWarning {
    fn default() -> Self {
        Self::new()
    }
}

impl RiskLimitConsumer for LeverageReductionWarning {
    fn update_risk_limit(&mut self, r: &RiskLimit) {
        if self.prev_max_leverage.is_finite() {
            self.last_signal = if r.max_leverage < self.prev_max_leverage {
                1
            } else if r.max_leverage > self.prev_max_leverage {
                -1
            } else {
                0
            };
        } else {
            self.last_signal = 0;
        }
        self.prev_max_leverage = r.max_leverage;
    }


    fn reset(&mut self) {
        self.prev_max_leverage = f64::NAN;
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        self.prev_max_leverage.is_finite()
    }
}

/// Unit config — LeverageReductionWarning has no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct LeverageReductionWarningConfig;

impl Indicator for LeverageReductionWarning {
    const ID: IndicatorId = IndicatorId::LeverageReductionWarning;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::RiskLimit];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::LeverageReductionWarning)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = LeverageReductionWarningConfig;
    type Runtime = LeverageReductionWarning;

    fn create(_cfg: LeverageReductionWarningConfig) -> LeverageReductionWarning {
        LeverageReductionWarning::new()
    }
}

impl crate::contract::Config for LeverageReductionWarningConfig {
    fn defaults() -> Self {
        LeverageReductionWarningConfig
    }
    fn machine_defaults() -> Self {
        // Unit config — no Param fields. Auto is the full implementation.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for LeverageReductionWarning {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::LeverageReductionWarning, "Leverage Warning", Color::hex(0xE91E63))
            .zero_baseline()
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    fn make_risk_limit(max_leverage: f64) -> RiskLimit {
        RiskLimit {
            tier: 1,
            max_leverage,
            max_position_value: 100_000.0,
            mmr: 0.005,
            imr: 0.01,
            timestamp: 0,
        }
    }

    #[test]
    fn tightening_gives_positive_signal() {
        let mut ind = LeverageReductionWarning::new();
        ind.update_risk_limit(&make_risk_limit(100.0));
        ind.update_risk_limit(&make_risk_limit(50.0));
        assert_eq!(ind.value() as i8, 1);
    }

    #[test]
    fn loosening_gives_negative_signal() {
        let mut ind = LeverageReductionWarning::new();
        ind.update_risk_limit(&make_risk_limit(50.0));
        ind.update_risk_limit(&make_risk_limit(100.0));
        assert_eq!(ind.value() as i8, -1);
    }

    #[test]
    fn unchanged_gives_zero() {
        let mut ind = LeverageReductionWarning::new();
        ind.update_risk_limit(&make_risk_limit(100.0));
        ind.update_risk_limit(&make_risk_limit(100.0));
        assert_eq!(ind.value() as i8, 0);
    }

    #[test]
    fn first_update_gives_zero() {
        let mut ind = LeverageReductionWarning::new();
        ind.update_risk_limit(&make_risk_limit(100.0));
        assert_eq!(ind.value() as i8, 0);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = LeverageReductionWarning::new();
        ind.update_risk_limit(&make_risk_limit(100.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value() as i8, 0);
    }

    #[test]
    fn factory_feeds_resolved_leverage_reduction_warning() {
        let mut f = IndicatorOrder::LeverageReductionWarning(LeverageReductionWarningConfig).build_solo().unwrap();
        let rl1 = RiskLimit {
            tier: 1,
            max_leverage: 9999.0,
            max_position_value: 100_000.0,
            mmr: 0.005,
            imr: 0.01,
            timestamp: 1_000,
        };
        let rl2 = RiskLimit {
            tier: 1,
            max_leverage: 50.0,
            max_position_value: 100_000.0,
            mmr: 0.005,
            imr: 0.01,
            timestamp: 2_000,
        };
        f.feed(0, MarketSample::RiskLimit(&rl1));
        f.feed(0, MarketSample::RiskLimit(&rl2));
        assert_eq!(f.primary() as i8, 1, "tightening expected");
    }
}

impl LeverageReductionWarning {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }
}
