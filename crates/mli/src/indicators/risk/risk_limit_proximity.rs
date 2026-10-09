//! RiskLimitProximity — proxy score for how restrictive the exchange margin tier is.
//!
//! Cannot compute "proximity to limit" without current position data, which is
//! account-level and not available in indicator streams.
//!
//! SIMPLIFIED: uses (mmr + imr) / 2.0 as a proxy — higher values mean the exchange
//! tier is more restrictive (risk-off environment).

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::risk_limit_consumer::RiskLimitConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::RiskLimit;

/// Proxy score for exchange margin requirement restrictiveness.
///
/// score = (mmr + imr) / 2.0
///
/// Higher values → exchange demands more margin → tighter risk environment.
///
/// Note: this is NOT a proximity-to-limit in the position sense — that requires
/// current position data which is account-level. This indicator tracks how
/// restrictive the current tier is relative to its own margin requirements.
///
/// Output: `Single(score)`.
#[derive(Debug, Clone)]
pub struct RiskLimitProximity {
    last_score: f64,
    has_data: bool,
}

impl RiskLimitProximity {
    /// Create a new indicator.
    pub fn new() -> Self {
        Self {
            last_score: 0.0,
            has_data: false,
        }
    }
}

impl Default for RiskLimitProximity {
    fn default() -> Self {
        Self::new()
    }
}

impl RiskLimitConsumer for RiskLimitProximity {
    fn update_risk_limit(&mut self, r: &RiskLimit) {
        self.last_score = (r.mmr + r.imr) / 2.0;
        self.has_data = true;
    }


    fn reset(&mut self) {
        self.last_score = 0.0;
        self.has_data = false;
    }

    fn is_ready(&self) -> bool {
        self.has_data
    }
}

/// Unit config — RiskLimitProximity has no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct RiskLimitProximityConfig;

impl Indicator for RiskLimitProximity {
    const ID: IndicatorId = IndicatorId::RiskLimitProximity;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::RiskLimit];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::RiskLimitProximity)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = RiskLimitProximityConfig;
    type Runtime = RiskLimitProximity;

    fn create(_cfg: RiskLimitProximityConfig) -> RiskLimitProximity {
        RiskLimitProximity::new()
    }
}

impl crate::contract::Config for RiskLimitProximityConfig {
    fn defaults() -> Self {
        RiskLimitProximityConfig
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


impl Render for RiskLimitProximity {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::RiskLimitProximity, "Risk Limit Proximity", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    fn make_risk_limit(mmr: f64, imr: f64) -> RiskLimit {
        RiskLimit {
            tier: 1,
            max_leverage: 100.0,
            max_position_value: 100_000.0,
            mmr,
            imr,
            timestamp: 0,
        }
    }

    #[test]
    fn score_is_average_of_mmr_imr() {
        let mut ind = RiskLimitProximity::new();
        ind.update_risk_limit(&make_risk_limit(0.005, 0.01));
        let s = ind.value();
        let expected = (0.005 + 0.01) / 2.0;
        assert!((s - expected).abs() < 1e-12, "score = {s}, expected {expected}");
    }

    #[test]
    fn score_increases_with_tighter_requirements() {
        let mut ind = RiskLimitProximity::new();
        ind.update_risk_limit(&make_risk_limit(0.005, 0.01));
        let v1 = ind.value();
        ind.update_risk_limit(&make_risk_limit(0.01, 0.02));
        let v2 = ind.value();
        assert!(v2 > v1, "tighter requirements should give higher score");
    }

    #[test]
    fn not_ready_before_first_update() {
        let ind = RiskLimitProximity::new();
        assert!(!ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = RiskLimitProximity::new();
        ind.update_risk_limit(&make_risk_limit(0.005, 0.01));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_risk_limit_proximity() {
        let mut f = IndicatorOrder::RiskLimitProximity(RiskLimitProximityConfig).build_solo().unwrap();
        let rl = RiskLimit {
            tier: 2,
            max_leverage: 50.0,
            max_position_value: 50_000.0,
            mmr: 0.005,
            imr: 0.01,
            timestamp: 2_000,
        };
        f.feed(0, MarketSample::RiskLimit(&rl));
        let v = f.primary();
        let expected = (0.005 + 0.01) / 2.0;
        assert!((v - expected).abs() < 1e-12, "score={v}");
    }
}

impl RiskLimitProximity {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_score
    }
}
