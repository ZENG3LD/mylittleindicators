// RSI Percentile Bands: upper/lower percentiles of RSI distribution

use crate::indicators::momentum::rsi::Rsi;
use crate::indicators::utils::math::percentile::quickselect_nth;

/// RSI Percentile Bands — draws the 20th and 80th percentile of RSI values over a
/// rolling window alongside the current RSI (middle), providing dynamic
/// overbought/oversold thresholds that adapt to recent RSI behaviour.
#[derive(Debug, Clone)]
pub struct RsiPercentileBands {
    rsi: Rsi,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl RsiPercentileBands {
    pub fn new(rsi_period: usize, window: usize) -> Self {
        let w = window.clamp(10, 1024);
        Self {
            rsi: Rsi::new(rsi_period.max(1)),
            window: w,
            buf: Vec::with_capacity(w),
            idx: 0,
            filled: false,
            upper: 80.0,
            middle: 50.0,
            lower: 20.0,
        }
    }

    /// Alias exposing the RSI period parameter explicitly.
    ///
    /// # Arguments
    /// * `rsi_period` - RSI lookback period (minimum 1)
    /// * `window`     - Rolling window for percentile computation (clamped 10..1024)
    #[inline]
    pub fn with_rsi_period(rsi_period: usize, window: usize) -> Self {
        Self::new(rsi_period, window)
    }
    #[inline]
    pub fn reset(&mut self) {
        self.rsi.reset();
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.upper = 80.0;
        self.middle = 50.0;
        self.lower = 20.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.rsi.is_ready()
    }

    /// Feed a pre-resolved scalar — the contracted entry point.
    ///
    /// The inner RSI is driven via the close slot; math is byte-identical to the
    /// legacy `update_bar` which passed all OHLCV fields to RSI (which defaulted to close).
    pub fn feed(&mut self, value: f64) -> (f64, f64, f64) {
        let r = self.rsi.feed(value);
        if self.buf.len() < self.window {
            self.buf.push(r);
            if self.buf.len() == self.window {
                self.filled = true;
            }
        } else {
            self.buf[self.idx] = r;
        }
        self.idx = (self.idx + 1) % self.window;
        self.middle = r;
        if self.is_ready() {
            // O(n) quickselect instead of O(n log n) sorting
            let mut sorted: Vec<f64> = self.buf.iter().copied().collect();
            let len = sorted.len();
            self.lower = quickselect_nth(&mut sorted, (len * 20) / 100);
            self.upper = quickselect_nth(&mut sorted, (len * 80) / 100);
        }
        (self.upper, self.middle, self.lower)
    }

    pub fn window(&self) -> usize {
        self.window
    }

    /// Brace-named getter: `upper` output (80th-percentile RSI band).
    #[inline]
    pub fn upper(&self) -> f64 {
        self.upper
    }

    /// Brace-named getter: `middle` output (current RSI value).
    #[inline]
    pub fn middle(&self) -> f64 {
        self.middle
    }

    /// Brace-named getter: `lower` output (20th-percentile RSI band).
    #[inline]
    pub fn lower(&self) -> f64 {
        self.lower
    }
}

impl Default for RsiPercentileBands {
    fn default() -> Self {
        Self::with_rsi_period(14, 200)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice, SmootherId};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`RsiPercentileBands`] — RSI period + RSI smoother + rolling window + source.
///
/// Dual-mode: every field is a `Param`. RSI smoother follows `rsi_period` by default
/// (`follow(Rma)` = Wilder's original). Inner `Rsi` built via `Rsi::from_choice(choice, period)`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct RsiPercentileBandsConfig {
    /// RSI lookback period (minimum 1).
    pub rsi_period: Param<usize>,
    /// Rolling window for percentile computation (clamped 10..1024).
    pub window: Param<usize>,
    /// Configurable price source (default close).
    pub source: Param<OhlcvField>,
    /// RSI gain/loss smoother choice — default `follow(Rma)` (Wilder's RMA at `rsi_period`).
    #[slot]
    pub rsi_smoother: Param<SmootherChoice>,
}

impl Indicator for RsiPercentileBands {
    const ID: IndicatorId = IndicatorId::RsiPctBands;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// Rolling Vec for RSI value buffer; quickselect is O(n) per bar; inner RSI as a Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi])],
    };
    const SLOTS: &'static [crate::contract::Slot] = RsiPercentileBandsConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::RsiPctBandsUpper),
        Output::percent(IndicatorOutputId::RsiPctBandsMiddle),
        Output::percent(IndicatorOutputId::RsiPctBandsLower),
    ];

    type Config = RsiPercentileBandsConfig;
    type Runtime = RsiPercentileBands;

    fn create(cfg: RsiPercentileBandsConfig) -> RsiPercentileBands {
        let rsi_period = cfg.rsi_period.resolved().max(1);
        let window = cfg.window.resolved().clamp(10, 1024);
        let choice = cfg.rsi_smoother.resolved();
        RsiPercentileBands {
            rsi: Rsi::from_choice(choice, rsi_period),
            window,
            buf: Vec::with_capacity(window),
            idx: 0,
            filled: false,
            upper: 80.0,
            middle: 50.0,
            lower: 20.0,
        }
    }

    fn source_fields(cfg: &RsiPercentileBandsConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &RsiPercentileBandsConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for RsiPercentileBandsConfig {
    fn defaults() -> Self {
        RsiPercentileBandsConfig {
            rsi_period: Param::Solo(14),
            window: Param::Solo(200),
            source: Param::Solo(OhlcvField::Close),
            rsi_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // rsi_period/window: Class A → auto range(2,4048,1); source: Class O → auto all-8.
        // rsi_smoother: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for RsiPercentileBands {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::RsiPctBandsUpper, "Upper", Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::RsiPctBandsMiddle, "RSI", Color::hex(0x9C27B0), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::RsiPctBandsLower, "Lower", Color::hex(0x4CAF50), 1.0))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsi_percentile_bands_creation() {
        let rpb = RsiPercentileBands::new(14, 50);
        assert!(!rpb.is_ready());
        assert_eq!(rpb.upper(), 80.0);
        assert_eq!(rpb.middle(), 50.0);
        assert_eq!(rpb.lower(), 20.0);
        assert_eq!(rpb.window(), 50);
    }

    #[test]
    fn test_rsi_percentile_bands_with_rsi_period() {
        let mut rpb = RsiPercentileBands::with_rsi_period(9, 30);
        assert_eq!(rpb.window(), 30);
        for i in 1..=60 {
            let p = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let (upper, mid, lower) = rpb.feed(p);
            assert!(upper.is_finite() && mid.is_finite() && lower.is_finite());
        }
        assert!(rpb.is_ready());
    }

    #[test]
    fn test_rsi_percentile_bands_basic() {
        let mut rpb = RsiPercentileBands::new(14, 50);
        for i in 1..=80 {
            let price = 100.0 + i as f64 * 2.0;
            rpb.feed(price);
        }
        assert!(rpb.is_ready());
        let (upper, middle, lower) = (rpb.upper(), rpb.middle(), rpb.lower());
        assert!(upper.is_finite() && middle.is_finite() && lower.is_finite());
        assert!(upper >= lower, "Upper band should >= lower band");
    }

    #[test]
    fn test_rsi_percentile_bands_finite() {
        let mut rpb = RsiPercentileBands::new(14, 50);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let (upper, middle, lower) = rpb.feed(price);
            assert!(upper.is_finite() && middle.is_finite() && lower.is_finite());
        }
    }

    #[test]
    fn test_rsi_percentile_bands_reset() {
        let mut rpb = RsiPercentileBands::new(14, 50);
        for i in 1..=80 {
            let price = 100.0 + i as f64;
            rpb.feed(price);
        }
        assert!(rpb.is_ready());
        rpb.reset();
        assert!(!rpb.is_ready());
        assert_eq!(rpb.upper(), 80.0);
        assert_eq!(rpb.middle(), 50.0);
        assert_eq!(rpb.lower(), 20.0);
    }

    /// Factory resolves the close field (not the wild 9999 high) and feeds the scalar.
    /// Bands should be finite once window is filled.
    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut cfg = <<RsiPercentileBands as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        cfg.window = Param::Solo(20); // reduced so test fills faster
        let mut f = IndicatorOrder::RsiPctBands(cfg).build_solo().unwrap();
        for i in 1..=60 {
            let price = 100.0 + (i as f64 * 0.4).sin() * 15.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite(), "factory RsiPctBands primary output should be finite");
    }

    #[test]
    fn config_dual_mode() {
        use crate::contract::{Config};
        let d = <<RsiPercentileBands as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(d.cube_size(), 1);
        assert_eq!(d.iter().count(), 1);
    }
}
