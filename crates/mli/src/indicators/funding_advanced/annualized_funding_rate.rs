//! AnnualizedFundingRate — converts the raw funding rate to annualized percentage.
//!
//! Formula: `annualized = rate × periods_per_day × 365 × 100`
//!
//! For 8-hour funding intervals the default is `periods_per_day = 3.0`.
//!
//! Output: `Single(annualized_pct)`.

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::core::types::FundingRate;
use crate::engine::stream_kind::StreamKind;

/// Converts per-snapshot funding rate to annualized percentage.
///
/// `annualized_pct = rate × funding_periods_per_day × 365 × 100`
#[derive(Debug, Clone)]
pub struct AnnualizedFundingRate {
    funding_periods_per_day: f64,
    last_value: f64,
    ready: bool,
}

impl AnnualizedFundingRate {
    /// Create a new indicator.
    ///
    /// `funding_periods_per_day`: number of funding events per day.
    /// For 8-hour intervals use `3.0` (default).
    pub fn new(funding_periods_per_day: f64) -> Self {
        Self {
            funding_periods_per_day: funding_periods_per_day.max(0.0),
            last_value: 0.0,
            ready: false,
        }
    }

    fn compute(&self, rate: f64) -> f64 {
        rate * self.funding_periods_per_day * 365.0 * 100.0
    }
}

impl Default for AnnualizedFundingRate {
    fn default() -> Self {
        Self::new(3.0)
    }
}

/// Typed configuration for [`AnnualizedFundingRate`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AnnualizedFundingRateConfig {
    pub funding_periods_per_day: Param<f64>,
}

impl Indicator for AnnualizedFundingRate {
    const ID: IndicatorId = IndicatorId::AnnualizedFundingRate;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Funding];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::AnnualizedFundingRate)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::fixed(StoreKind::Scalar, 2)]);
    type Config = AnnualizedFundingRateConfig;
    type Runtime = Self;

    fn create(cfg: AnnualizedFundingRateConfig) -> Self {
        Self::new(cfg.funding_periods_per_day.resolved())
    }
}

impl crate::contract::Config for AnnualizedFundingRateConfig {
    fn defaults() -> Self {
        AnnualizedFundingRateConfig { funding_periods_per_day: Param::Solo(3.0) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // funding_periods_per_day: Class J PIN — exchange constant (3.0 = Binance/Bybit 8h cycle).
        // f64 auto leaves Solo; confirmed untouched.
        Self::machine_defaults_auto()
    }
}


impl Render for AnnualizedFundingRate {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::AnnualizedFundingRate, "Annualized Funding Rate", Color::hex(0xFF9800))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

impl FundingRateConsumer for AnnualizedFundingRate {
    fn update_funding(&mut self, fr: &FundingRate) {
        self.last_value = self.compute(fr.rate);
        self.ready = true;
    }


    fn reset(&mut self) {
        self.last_value = 0.0;
        self.ready = false;
    }

    fn is_ready(&self) -> bool {
        self.ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_annualized_funding_rate() {
        let mut f = IndicatorOrder::AnnualizedFundingRate(
            <<AnnualizedFundingRate as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let fr = FundingRate { rate: 0.0001, next_funding_time: None, timestamp: 1000, ..Default::default()};
        f.feed(0, MarketSample::Funding(&fr));
        assert!(f.primary() > 0.0);
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
    fn annualized_formula_default_periods() {
        // rate=0.0001, periods=3, days=365 → 0.0001 × 3 × 365 × 100 = 10.95
        let mut ind = AnnualizedFundingRate::default();
        ind.update_funding(&make_fr(0.0001));
        let val = ind.value();
        let expected = 0.0001_f64 * 3.0 * 365.0 * 100.0;
        assert!((val - expected).abs() < 1e-10, "expected {expected}, got {val}");
    }

    #[test]
    fn zero_rate_gives_zero() {
        let mut ind = AnnualizedFundingRate::default();
        ind.update_funding(&make_fr(0.0));
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn negative_rate_gives_negative_annualized() {
        let mut ind = AnnualizedFundingRate::default();
        ind.update_funding(&make_fr(-0.0001));
        assert!(ind.value() < 0.0, "negative rate should give negative annualized");
    }

    #[test]
    fn not_ready_before_first_update() {
        let ind = AnnualizedFundingRate::default();
        assert!(!ind.is_ready());
    }

    #[test]
    fn ready_after_first_update() {
        let mut ind = AnnualizedFundingRate::default();
        ind.update_funding(&make_fr(0.0001));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = AnnualizedFundingRate::default();
        ind.update_funding(&make_fr(0.0001));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }
}

impl AnnualizedFundingRate {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_value
    }
}
