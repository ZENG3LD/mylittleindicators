//! PredictedFundingExtreme — detects extreme predicted funding rate.

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::predicted_funding_consumer::PredictedFundingConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::contract::axis::sweep_f64;
use crate::contract::{Color, Render, RenderSpec};
use crate::core::types::PredictedFunding;
use crate::engine::stream_kind::StreamKind;

/// Detects when the absolute predicted funding rate exceeds a threshold.
///
/// - `+1` when predicted_rate > threshold (extreme positive funding)
/// - `-1` when predicted_rate < -threshold (extreme negative funding)
/// - `0`  otherwise
///
/// Output: `Signal(i8)`.
///
/// Default threshold: `0.001` (= 0.1%).
#[derive(Debug, Clone)]
pub struct PredictedFundingExtreme {
    threshold: f64,
    last_signal: i8,
    has_data: bool,
}

impl PredictedFundingExtreme {
    /// Create a new indicator with a custom threshold.
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            last_signal: 0,
            has_data: false,
        }
    }

    /// Create with the default threshold of 0.001.
    pub fn with_default_threshold() -> Self {
        Self::new(0.001)
    }
}

impl Default for PredictedFundingExtreme {
    fn default() -> Self {
        Self::with_default_threshold()
    }
}

/// Typed configuration for [`PredictedFundingExtreme`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PredictedFundingExtremeConfig {
    pub threshold: Param<f64>,
}

impl Indicator for PredictedFundingExtreme {
    const ID: IndicatorId = IndicatorId::PredictedFundingExtreme;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::PredictedFunding];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::PredictedFundingExtreme)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = PredictedFundingExtremeConfig;
    type Runtime = Self;

    fn create(cfg: PredictedFundingExtremeConfig) -> Self {
        Self::new(cfg.threshold.resolved())
    }
}

impl crate::contract::Config for PredictedFundingExtremeConfig {
    fn defaults() -> Self {
        PredictedFundingExtremeConfig { threshold: Param::Solo(0.001) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // threshold: named "threshold" (Class F pattern) but operates in raw funding rate space,
        // not sigma space. Default 0.001 = 0.1%; practical range 0.01%–1.0% (0.0001–0.01).
        // Sweep: 0.0001..=0.01 step 0.0001 (100 values covering the full perpetual funding range).
        s.threshold = Param::many(sweep_f64(0.0001, 0.01, 0.0001));
        s
    }
}


impl Render for PredictedFundingExtreme {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::PredictedFundingExtreme, "Predicted Funding Extreme", Color::hex(0xE91E63))
            .zero_baseline()
            .precision(0)
            .build()
    }
}

impl PredictedFundingConsumer for PredictedFundingExtreme {
    fn update_predicted_funding(&mut self, pf: &PredictedFunding) {
        self.has_data = true;
        self.last_signal = if pf.predicted_rate > self.threshold {
            1
        } else if pf.predicted_rate < -self.threshold {
            -1
        } else {
            0
        };
    }


    fn reset(&mut self) {
        self.last_signal = 0;
        self.has_data = false;
    }

    fn is_ready(&self) -> bool {
        self.has_data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_predicted_funding_extreme() {
        let mut f = IndicatorOrder::PredictedFundingExtreme(
            <<PredictedFundingExtreme as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let pf = PredictedFunding { predicted_rate: 0.005, next_funding_time: 0, timestamp: 0 };
        f.feed(0, MarketSample::PredictedFunding(&pf));
        // Signal(1) → main() = 1.0
        assert_eq!(f.primary(), 1.0);
    }

    fn make_predicted(rate: f64) -> PredictedFunding {
        PredictedFunding {
            predicted_rate: rate,
            next_funding_time: 0,
            timestamp: 0,
        }
    }

    #[test]
    fn positive_extreme_gives_one() {
        let mut ind = PredictedFundingExtreme::new(0.001);
        ind.update_predicted_funding(&make_predicted(0.005));
        assert_eq!(ind.value().round() as i8, 1);
    }

    #[test]
    fn negative_extreme_gives_minus_one() {
        let mut ind = PredictedFundingExtreme::new(0.001);
        ind.update_predicted_funding(&make_predicted(-0.005));
        assert_eq!(ind.value().round() as i8, -1);
    }

    #[test]
    fn normal_range_gives_zero() {
        let mut ind = PredictedFundingExtreme::new(0.001);
        ind.update_predicted_funding(&make_predicted(0.0003));
        assert_eq!(ind.value().round() as i8, 0);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = PredictedFundingExtreme::new(0.001);
        ind.update_predicted_funding(&make_predicted(0.005));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().round() as i8, 0);
    }
}

impl PredictedFundingExtreme {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }
}
