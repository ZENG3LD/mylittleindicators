//! OI Change Rate — rate of change of open interest per unit time.
//!
//! Measures how fast open interest is growing or shrinking.
//! Positive = OI increasing (new positions opening).
//! Negative = OI decreasing (positions closing / liquidations).
//!
//! Rate = (oi_now - oi_prev) / (dt_seconds)
//!
//! Output: `Double(oi_change_rate, oi_current)`.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::open_interest_consumer::OpenInterestConsumer;
use crate::contract::{Family, Indicator, Output, SourceAxis};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::OpenInterest;

/// Open interest change rate per second.
#[derive(Clone, Debug)]
pub struct OiChangeRate {
    prev_oi: f64,
    prev_ts: i64,
    last_rate: f64,
    last_oi: f64,
    has_prev: bool,
}

impl Default for OiChangeRate {
    fn default() -> Self {
        Self::new()
    }
}

impl OiChangeRate {
    pub fn new() -> Self {
        Self {
            prev_oi: 0.0,
            prev_ts: 0,
            last_rate: 0.0,
            last_oi: 0.0,
            has_prev: false,
        }
    }
}

impl OiChangeRate {
    /// Rate of change of open interest per second.
    pub fn rate(&self) -> f64 {
        self.last_rate
    }

    /// Most recent open interest value.
    pub fn current(&self) -> f64 {
        self.last_oi
    }
}

impl OpenInterestConsumer for OiChangeRate {
    fn update_oi(&mut self, oi: &OpenInterest) {
        self.last_oi = oi.open_interest;
        if self.has_prev {
            let dt_ms = (oi.timestamp - self.prev_ts).max(1);
            let dt_sec = dt_ms as f64 / 1000.0;
            let delta = oi.open_interest - self.prev_oi;
            self.last_rate = delta / dt_sec;
        }
        self.prev_oi = oi.open_interest;
        self.prev_ts = oi.timestamp;
        self.has_prev = true;
    }


    fn reset(&mut self) {
        self.prev_oi = 0.0;
        self.prev_ts = 0;
        self.last_rate = 0.0;
        self.last_oi = 0.0;
        self.has_prev = false;
    }

    fn is_ready(&self) -> bool {
        self.has_prev
    }
}

/// Typed configuration for [`OiChangeRate`]. No parameters — the rate is purely
/// derived from consecutive OI timestamps.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct OiChangeRateConfig;

impl Indicator for OiChangeRate {
    const ID: IndicatorId = IndicatorId::OiChangeRate;
    const FAMILY: &'static [Family] = &[Family::OpenInterest];
    const INPUT: &'static [StreamKind] = &[StreamKind::OpenInterest];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::OiChangeRateRate),
        Output::count(IndicatorOutputId::OiChangeRateCurrent),
    ];
    type Config = OiChangeRateConfig;
    type Runtime = OiChangeRate;

    fn create(_cfg: OiChangeRateConfig) -> OiChangeRate {
        OiChangeRate::new()
    }
}

impl crate::contract::Config for OiChangeRateConfig {
    fn defaults() -> Self {
        OiChangeRateConfig
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // Unit config — no Param fields. Auto is the full implementation.
        Self::machine_defaults_auto()
    }
}


impl Render for OiChangeRate {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::OiChangeRateRate, "OI Change Rate", Color::hex(0xFF9800))
            .line_output(IndicatorOutputId::OiChangeRateCurrent, "OI Current", Color::hex(0x9E9E9E))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    fn make_oi(oi: f64, ts: i64) -> OpenInterest {
        OpenInterest { open_interest: oi, open_interest_value: None, timestamp: ts, ..Default::default()}
    }

    #[test]
    fn factory_feeds_resolved_oi_change_rate() {
        let mut f = IndicatorOrder::OiChangeRate(OiChangeRateConfig).build_solo().unwrap();
        let oi1 = make_oi(1000.0, 0);
        let oi2 = make_oi(1200.0, 2000);
        f.feed(0, MarketSample::OpenInterest(&oi1));
        f.feed(0, MarketSample::OpenInterest(&oi2));
        // f.value() returns first output (rate)
        let rate = f.primary();
        assert!((rate - 100.0).abs() < 1e-6, "rate={rate}");
    }

    #[test]
    fn not_ready_initially() {
        let oicr = OiChangeRate::new();
        assert!(!oicr.is_ready());
    }

    #[test]
    fn ready_after_first_update() {
        let mut oicr = OiChangeRate::new();
        oicr.update_oi(&make_oi(1000.0, 0));
        assert!(oicr.is_ready());
    }

    #[test]
    fn rate_zero_on_first_update() {
        let mut oicr = OiChangeRate::new();
        oicr.update_oi(&make_oi(1000.0, 0));
        assert_eq!(oicr.rate(), 0.0);
    }

    #[test]
    fn positive_rate_on_growing_oi() {
        let mut oicr = OiChangeRate::new();
        oicr.update_oi(&make_oi(1000.0, 0));
        // 200 OI increase over 2 seconds = 100 per second
        oicr.update_oi(&make_oi(1200.0, 2000));
        let rate = oicr.rate();
        let oi = oicr.current();
        assert!((rate - 100.0).abs() < 1e-6, "expected 100/s, got {}", rate);
        assert!((oi - 1200.0).abs() < 1e-9);
    }

    #[test]
    fn negative_rate_on_shrinking_oi() {
        let mut oicr = OiChangeRate::new();
        oicr.update_oi(&make_oi(1000.0, 0));
        oicr.update_oi(&make_oi(800.0, 1000));
        let rate = oicr.rate();
        assert!(rate < 0.0, "rate should be negative when OI shrinks");
    }

    #[test]
    fn reset_clears_state() {
        let mut oicr = OiChangeRate::new();
        oicr.update_oi(&make_oi(1000.0, 0));
        oicr.reset();
        assert!(!oicr.is_ready());
    }
}
