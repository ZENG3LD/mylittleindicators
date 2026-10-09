//! MmrTracker — tracks the current maintenance margin ratio.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::risk_limit_consumer::RiskLimitConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::RiskLimit;

/// Tracks the current maintenance margin ratio (MMR) from exchange risk limit tiers.
///
/// Returns the latest MMR as received, or 0.0 before any update.
///
/// Output: `Single(mmr)`.
#[derive(Debug, Clone)]
pub struct MmrTracker {
    current_mmr: f64,
    has_data: bool,
}

impl MmrTracker {
    /// Create a new indicator with no prior state.
    pub fn new() -> Self {
        Self {
            current_mmr: 0.0,
            has_data: false,
        }
    }
}

impl Default for MmrTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl RiskLimitConsumer for MmrTracker {
    fn update_risk_limit(&mut self, r: &RiskLimit) {
        self.current_mmr = r.mmr;
        self.has_data = true;
    }


    fn reset(&mut self) {
        self.current_mmr = 0.0;
        self.has_data = false;
    }

    fn is_ready(&self) -> bool {
        self.has_data
    }
}

/// Unit config — MmrTracker has no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct MmrTrackerConfig;

impl Indicator for MmrTracker {
    const ID: IndicatorId = IndicatorId::MmrTracker;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::RiskLimit];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::MmrTracker)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = MmrTrackerConfig;
    type Runtime = MmrTracker;

    fn create(_cfg: MmrTrackerConfig) -> MmrTracker {
        MmrTracker::new()
    }
}

impl crate::contract::Config for MmrTrackerConfig {
    fn defaults() -> Self {
        MmrTrackerConfig
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


impl Render for MmrTracker {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::MmrTracker, "MMR", Color::hex(0xF44336))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    fn make_risk_limit(mmr: f64) -> RiskLimit {
        RiskLimit {
            tier: 1,
            max_leverage: 100.0,
            max_position_value: 100_000.0,
            mmr,
            imr: 0.01,
            timestamp: 0,
        }
    }

    #[test]
    fn tracks_mmr_correctly() {
        let mut ind = MmrTracker::new();
        assert!(!ind.is_ready());
        ind.update_risk_limit(&make_risk_limit(0.005));
        assert!(ind.is_ready());
        let v = ind.value();
        assert!((v - 0.005).abs() < 1e-12);
    }

    #[test]
    fn updates_on_new_risk_limit() {
        let mut ind = MmrTracker::new();
        ind.update_risk_limit(&make_risk_limit(0.005));
        ind.update_risk_limit(&make_risk_limit(0.01));
        let v = ind.value();
        assert!((v - 0.01).abs() < 1e-12);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = MmrTracker::new();
        ind.update_risk_limit(&make_risk_limit(0.005));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_mmr_tracker() {
        let mut f = IndicatorOrder::MmrTracker(MmrTrackerConfig).build_solo().unwrap();
        let rl = RiskLimit {
            tier: 1,
            max_leverage: 100.0,
            max_position_value: 100_000.0,
            mmr: 0.0075,
            imr: 0.015,
            timestamp: 1_000,
        };
        f.feed(0, MarketSample::RiskLimit(&rl));
        f.feed(0, MarketSample::RiskLimit(&rl));
        let v = f.primary();
        assert!((v - 0.0075).abs() < 1e-12, "mmr={v}");
    }
}

impl MmrTracker {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.current_mmr
    }
}
