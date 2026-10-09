//! FundingDrift — spread between predicted and actual funding rate.

use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::predicted_funding_consumer::PredictedFundingConsumer;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::FundingRate;
use crate::core::types::PredictedFunding;

/// Drift between the exchange-predicted funding rate and the actual settled rate.
///
/// `drift = predicted_rate - actual_rate`
///
/// Implements both `PredictedFundingConsumer` and `FundingRateConsumer`.
/// Inherent methods (`indicator_value`, `indicator_is_ready`, `indicator_reset`)
/// are used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
///
/// Output: `Single(drift)`.
#[derive(Debug, Clone)]
pub struct FundingDrift {
    last_predicted: f64,
    last_actual: f64,
    last_drift: f64,
}

impl FundingDrift {
    /// Create a new indicator with zeroed state.
    pub fn new() -> Self {
        Self {
            last_predicted: 0.0,
            last_actual: 0.0,
            last_drift: 0.0,
        }
    }

    /// Current indicator value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> f64 {
        self.last_drift
    }

    /// True if at least one stream has delivered data.
    pub fn indicator_is_ready(&self) -> bool {
        self.last_predicted != 0.0 || self.last_actual != 0.0
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.last_predicted = 0.0;
        self.last_actual = 0.0;
        self.last_drift = 0.0;
    }
}

impl Default for FundingDrift {
    fn default() -> Self {
        Self::new()
    }
}

impl PredictedFundingConsumer for FundingDrift {
    fn update_predicted_funding(&mut self, pf: &PredictedFunding) {
        self.last_predicted = pf.predicted_rate;
        self.last_drift = self.last_predicted - self.last_actual;
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl FundingRateConsumer for FundingDrift {
    fn update_funding(&mut self, fr: &FundingRate) {
        self.last_actual = fr.rate;
        self.last_drift = self.last_predicted - self.last_actual;
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

/// Typed configuration for [`FundingDrift`] — no parameters needed.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FundingDriftConfig;

impl Indicator for FundingDrift {
    const ID: IndicatorId = IndicatorId::FundingDrift;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::PredictedFunding, StreamKind::Funding];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::FundingDrift)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = FundingDriftConfig;
    type Runtime = FundingDrift;

    fn create(_cfg: FundingDriftConfig) -> FundingDrift {
        FundingDrift::new()
    }
}

impl crate::contract::Config for FundingDriftConfig {
    fn defaults() -> Self {
        FundingDriftConfig
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // No Param fields — unit struct; nothing to sweep.
        Self::machine_defaults_auto()
    }
}


impl Render for FundingDrift {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::FundingDrift, "Funding Drift", Color::hex(0xFF9800))
            .precision(6)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_predicted(rate: f64) -> PredictedFunding {
        PredictedFunding {
            predicted_rate: rate,
            next_funding_time: 0,
            timestamp: 0,
        }
    }

    fn make_funding_rate(rate: f64) -> FundingRate {
        FundingRate {
            rate,
            next_funding_time: None,
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn drift_is_predicted_minus_actual() {
        let mut ind = FundingDrift::new();
        ind.update_predicted_funding(&make_predicted(0.001));
        ind.update_funding(&make_funding_rate(0.0003));
        let d = ind.indicator_value();
        assert!((d - 0.0007).abs() < 1e-12, "drift = {d}");
    }

    #[test]
    fn negative_drift_when_actual_exceeds_predicted() {
        let mut ind = FundingDrift::new();
        ind.update_predicted_funding(&make_predicted(0.0003));
        ind.update_funding(&make_funding_rate(0.001));
        let d = ind.indicator_value();
        assert!(d < 0.0, "drift should be negative, got {d}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = FundingDrift::new();
        ind.update_predicted_funding(&make_predicted(0.001));
        ind.update_funding(&make_funding_rate(0.0003));
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        let v = ind.indicator_value();
        assert_eq!(v, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_funding_drift() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::FundingDrift(FundingDriftConfig).build_solo().unwrap();
        let pf = make_predicted(0.001);
        let fr = make_funding_rate(0.0003);
        f.feed(0, MarketSample::PredictedFunding(&pf));
        f.feed(0, MarketSample::Funding(&fr));
        let d = f.primary();
        assert!((d - 0.0007).abs() < 1e-12, "drift={d}");
    }
}
