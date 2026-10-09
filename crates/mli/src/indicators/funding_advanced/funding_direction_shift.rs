//! FundingDirectionShift — detects sign changes in the funding rate.
//!
//! Fires `Signal(+1)` when funding flips from negative → positive.
//! Fires `Signal(-1)` when funding flips from positive → negative.
//! Otherwise `Signal(0)`.
//!
//! Output: `Signal(i8)`.

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::core::types::FundingRate;
use crate::engine::stream_kind::StreamKind;

/// Detects sign flip in funding rate direction.
///
/// `+1` = flipped to positive (longs pay shorts).
/// `-1` = flipped to negative (shorts pay longs).
/// `0`  = no flip.
#[derive(Debug, Clone)]
pub struct FundingDirectionShift {
    prev_rate: Option<f64>,
    last_signal: i8,
}

impl FundingDirectionShift {
    /// Create a new indicator.
    pub fn new() -> Self {
        Self {
            prev_rate: None,
            last_signal: 0,
        }
    }
}

impl Default for FundingDirectionShift {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed configuration for [`FundingDirectionShift`] — no parameters.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FundingDirectionShiftConfig;

impl Indicator for FundingDirectionShift {
    const ID: IndicatorId = IndicatorId::FundingDirectionShift;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Funding];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::FundingDirectionShift)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = FundingDirectionShiftConfig;
    type Runtime = Self;

    fn create(_cfg: FundingDirectionShiftConfig) -> Self {
        Self::new()
    }
}

impl crate::contract::Config for FundingDirectionShiftConfig {
    fn defaults() -> Self {
        FundingDirectionShiftConfig
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


impl Render for FundingDirectionShift {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::FundingDirectionShift, "Funding Direction Shift", Color::hex(0xE91E63))
            .zero_baseline()
            .precision(0)
            .build()
    }
}

impl FundingRateConsumer for FundingDirectionShift {
    fn update_funding(&mut self, fr: &FundingRate) {
        let current = fr.rate;
        self.last_signal = match self.prev_rate {
            Some(prev) if prev > 0.0 && current < 0.0 => -1,
            Some(prev) if prev < 0.0 && current > 0.0 => 1,
            _ => 0,
        };
        self.prev_rate = Some(current);
    }


    fn reset(&mut self) {
        self.prev_rate = None;
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        self.prev_rate.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_funding_direction_shift() {
        let mut f = IndicatorOrder::FundingDirectionShift(
            <<FundingDirectionShift as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let fr = FundingRate { rate: 0.0001, next_funding_time: None, timestamp: 1000, ..Default::default()};
        f.feed(0, MarketSample::Funding(&fr));
        // value() returns Signal — main() coerces to f64
        let _ = f.primary();
    }

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate {
            rate,
            next_funding_time: None,
            timestamp: 1000,
            ..Default::default()
        }
    }

    #[test]
    fn pos_to_neg_gives_minus_one() {
        let mut ind = FundingDirectionShift::new();
        ind.update_funding(&make_fr(0.0001));
        ind.update_funding(&make_fr(-0.0001));
        assert_eq!(ind.value().round() as i8, -1);
    }

    #[test]
    fn neg_to_pos_gives_plus_one() {
        let mut ind = FundingDirectionShift::new();
        ind.update_funding(&make_fr(-0.0001));
        ind.update_funding(&make_fr(0.0001));
        assert_eq!(ind.value().round() as i8, 1);
    }

    #[test]
    fn same_sign_gives_zero() {
        let mut ind = FundingDirectionShift::new();
        ind.update_funding(&make_fr(0.0001));
        ind.update_funding(&make_fr(0.0002));
        assert_eq!(ind.value().round() as i8, 0);
    }

    #[test]
    fn not_ready_before_first_update() {
        let ind = FundingDirectionShift::new();
        assert!(!ind.is_ready());
    }

    #[test]
    fn ready_after_first_update() {
        let mut ind = FundingDirectionShift::new();
        ind.update_funding(&make_fr(0.0001));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears() {
        let mut ind = FundingDirectionShift::new();
        ind.update_funding(&make_fr(0.0001));
        ind.update_funding(&make_fr(-0.0001));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().round() as i8, 0);
    }
}

impl FundingDirectionShift {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }
}
