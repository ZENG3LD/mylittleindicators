//! FundingOiPressure — cross-stream composite of funding rate × OI delta.
//!
//! Dual consumer: `FundingRateConsumer` + `OpenInterestConsumer`.
//!
//! Logic:
//! - `funding` = last funding rate
//! - `oi_delta` = current_oi - prev_oi
//! - `pressure` = funding × oi_delta
//!   - Both same sign → growing directional pressure (positive)
//!   - Opposite signs → declining pressure (negative)
//!
//! Output: `Triple(funding, oi_delta, pressure)`.

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::open_interest_consumer::OpenInterestConsumer;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::core::types::FundingRate;
use crate::core::types::OpenInterest;
use crate::engine::stream_kind::StreamKind;

/// Cross-stream pressure composite.
///
/// Implements both `FundingRateConsumer` and `OpenInterestConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Debug, Clone)]
pub struct FundingOiPressure {
    last_funding: f64,
    last_oi: f64,
    prev_oi: f64,
    oi_seen: usize,
    last_pressure: f64,
    last_oi_delta: f64,
}

impl FundingOiPressure {
    /// Create a new indicator.
    pub fn new() -> Self {
        Self {
            last_funding: 0.0,
            last_oi: 0.0,
            prev_oi: 0.0,
            oi_seen: 0,
            last_pressure: 0.0,
            last_oi_delta: 0.0,
        }
    }

    fn recompute(&mut self) {
        self.last_pressure = self.last_funding * self.last_oi_delta;
    }

    /// Named output: brace `funding`.
    pub fn funding(&self) -> f64 { self.last_funding }
    /// Named output: brace `oi_delta`.
    pub fn oi_delta(&self) -> f64 { self.last_oi_delta }
    /// Named output: brace `pressure`.
    pub fn pressure(&self) -> f64 { self.last_pressure }


    /// True when both streams have delivered at least one update.
    pub fn indicator_is_ready(&self) -> bool {
        self.last_funding != 0.0 && self.oi_seen >= 2
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.last_funding = 0.0;
        self.last_oi = 0.0;
        self.prev_oi = 0.0;
        self.oi_seen = 0;
        self.last_pressure = 0.0;
        self.last_oi_delta = 0.0;
    }
}

impl Default for FundingOiPressure {
    fn default() -> Self {
        Self::new()
    }
}

impl FundingRateConsumer for FundingOiPressure {
    fn update_funding(&mut self, fr: &FundingRate) {
        self.last_funding = fr.rate;
        self.recompute();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl OpenInterestConsumer for FundingOiPressure {
    fn update_oi(&mut self, oi: &OpenInterest) {
        self.prev_oi = self.last_oi;
        self.last_oi = oi.open_interest;
        self.oi_seen += 1;
        if self.oi_seen >= 2 {
            self.last_oi_delta = self.last_oi - self.prev_oi;
        }
        self.recompute();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

/// Typed configuration for [`FundingOiPressure`].
///
/// No parameters — the indicator is stateless with respect to configuration.
#[derive(Debug, Clone)]
pub struct FundingOiPressureConfig;

impl FundingOiPressureConfig {
    /// Config fingerprint: no params → stable constant.
    pub fn config_hash(&self) -> u64 { 0 }

    /// Paramless config — the sole cube point is `self` (mixed-radix dual of `iter().nth`).
    pub fn axes_decode(&self, _idx: u128) -> Self { self.clone() }
}

impl Indicator for FundingOiPressure {
    const ID: IndicatorId = IndicatorId::FundingOiPressure;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Funding, StreamKind::OpenInterest];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::FundingOiPressureFunding),
        Output::centered(IndicatorOutputId::FundingOiPressureOiDelta),
        Output::centered(IndicatorOutputId::FundingOiPressurePressure),
    ];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = FundingOiPressureConfig;
    type Runtime = FundingOiPressure;

    fn create(_cfg: FundingOiPressureConfig) -> FundingOiPressure {
        FundingOiPressure::new()
    }
}

impl crate::contract::Config for FundingOiPressureConfig {
    fn defaults() -> Self {
        FundingOiPressureConfig
    }
    fn machine_defaults() -> Self {
        // No Param fields — nothing to sweep. machine_defaults == defaults.
        FundingOiPressureConfig
    }
    fn cube_size(&self) -> u128 {
        1
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        Box::new(std::iter::once(self.clone()))
    }
}


impl Render for FundingOiPressure {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::FundingOiPressureFunding, "Funding Rate", Color::hex(0x2196F3), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::FundingOiPressureOiDelta, "OI Delta", Color::hex(0xFF9800), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::FundingOiPressurePressure, "Pressure", Color::hex(0xE91E63), 2.0))
            .zero_baseline()
            .precision(6)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate { rate, next_funding_time: None, timestamp: 1000, ..Default::default()}
    }

    fn make_oi(open_interest: f64, ts: i64) -> OpenInterest {
        OpenInterest { open_interest, open_interest_value: None, timestamp: ts, ..Default::default()}
    }

    #[test]
    fn positive_funding_growing_oi_gives_positive_pressure() {
        let mut ind = FundingOiPressure::new();
        ind.update_funding(&make_fr(0.001));
        ind.update_oi(&make_oi(1000.0, 1000));
        ind.update_oi(&make_oi(1100.0, 2000)); // delta = +100
        {
            let (funding, delta, pressure) = (ind.funding(), ind.oi_delta(), ind.pressure());
            assert!((funding - 0.001).abs() < 1e-9);
            assert!((delta - 100.0).abs() < 1e-9);
            assert!(pressure > 0.0);
        }
    }

    #[test]
    fn positive_funding_falling_oi_gives_negative_pressure() {
        let mut ind = FundingOiPressure::new();
        ind.update_funding(&make_fr(0.001));
        ind.update_oi(&make_oi(1100.0, 1000));
        ind.update_oi(&make_oi(1000.0, 2000)); // delta = -100
        assert!(ind.pressure() < 0.0);
    }

    #[test]
    fn not_ready_before_two_oi_updates() {
        let mut ind = FundingOiPressure::new();
        ind.update_funding(&make_fr(0.001));
        ind.update_oi(&make_oi(1000.0, 1000));
        assert!(!ind.indicator_is_ready());
        ind.update_oi(&make_oi(1100.0, 2000));
        // funding non-zero, oi_seen >= 2
        assert!(ind.indicator_is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = FundingOiPressure::new();
        ind.update_funding(&make_fr(0.001));
        ind.update_oi(&make_oi(1000.0, 1000));
        ind.update_oi(&make_oi(1100.0, 2000));
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        assert_eq!(ind.funding(), 0.0);
        assert_eq!(ind.oi_delta(), 0.0);
        assert_eq!(ind.pressure(), 0.0);
    }

    #[test]
    fn factory_feeds_funding_oi_pressure() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::FundingOiPressure(FundingOiPressureConfig).build_solo().unwrap();
        let fr = make_fr(0.001);
        f.feed(0, MarketSample::Funding(&fr));
        let oi1 = make_oi(1000.0, 1000);
        f.feed(0, MarketSample::OpenInterest(&oi1));
        let oi2 = make_oi(1100.0, 2000);
        f.feed(0, MarketSample::OpenInterest(&oi2));
        let funding = f.primary();
        assert!((funding - 0.001).abs() < 1e-9, "funding={funding}");
    }
}
