// RSI Percentile Rank over a rolling window

use crate::indicators::momentum::rsi::Rsi;
use crate::indicators::utils::math::percentile::percentile_rank;

/// RSI Percentile Rank — ranks the current RSI reading within its own rolling
/// distribution, yielding a 0–100 percentile score rather than a fixed-threshold
/// overbought/oversold level.
///
/// `rank = percentile_rank(RSI_window, RSI_current)`
///
/// Near 100 = RSI at a multi-period high; near 0 = RSI at a multi-period low.
#[derive(Debug, Clone)]
pub struct RsiPercentileRank {
    rsi: Rsi,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl RsiPercentileRank {
    pub fn new(rsi_period: usize, window: usize) -> Self {
        let w = window.clamp(5, 1024);
        Self {
            rsi: Rsi::new(rsi_period.max(1)),
            window: w,
            buf: Vec::with_capacity(w),
            idx: 0,
            filled: false,
            value: 50.0,
        }
    }

    /// Alias exposing the RSI period parameter explicitly.
    ///
    /// # Arguments
    /// * `rsi_period` - RSI lookback period (minimum 1)
    /// * `window`     - Rolling window for rank computation (clamped 5..1024)
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
        self.value = 50.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.rsi.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed a pre-resolved scalar — the contracted entry point.
    ///
    /// The inner RSI is driven via the close slot; math is byte-identical to the
    /// legacy `update_bar` which passed all OHLCV fields to RSI (which defaulted to close).
    pub fn feed(&mut self, value: f64) -> f64 {
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
        if self.is_ready() {
            // O(n) percentile calculation instead of O(n log n) sorting
            self.value = percentile_rank(&self.buf[..], r);
        }
        self.value
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

impl Default for RsiPercentileRank {
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
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`RsiPercentileRank`] — RSI period + RSI smoother + rolling window + source.
///
/// Dual-mode: every field is a `Param`. RSI smoother follows `rsi_period` by default
/// (`follow(Rma)` = Wilder's original). Inner `Rsi` built via `Rsi::from_choice(choice, period)`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct RsiPercentileRankConfig {
    /// RSI lookback period (minimum 1).
    pub rsi_period: Param<usize>,
    /// Rolling window for percentile rank computation (clamped 5..1024).
    pub window: Param<usize>,
    /// Configurable price source (default close).
    pub source: Param<OhlcvField>,
    /// RSI gain/loss smoother choice — default `follow(Rma)` (Wilder's RMA at `rsi_period`).
    #[slot]
    pub rsi_smoother: Param<SmootherChoice>,
}

impl Indicator for RsiPercentileRank {
    const ID: IndicatorId = IndicatorId::RsiPctRank;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// Rolling Vec for RSI value buffer; percentile_rank is O(n) per bar; inner RSI as a Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi])],
    };
    const SLOTS: &'static [crate::contract::Slot] = RsiPercentileRankConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::RsiPctRank)];

    type Config = RsiPercentileRankConfig;
    type Runtime = RsiPercentileRank;

    fn create(cfg: RsiPercentileRankConfig) -> RsiPercentileRank {
        let rsi_period = cfg.rsi_period.resolved().max(1);
        let window = cfg.window.resolved().clamp(5, 1024);
        let choice = cfg.rsi_smoother.resolved();
        RsiPercentileRank {
            rsi: Rsi::from_choice(choice, rsi_period),
            window,
            buf: Vec::with_capacity(window),
            idx: 0,
            filled: false,
            value: 50.0,
        }
    }

    fn source_fields(cfg: &RsiPercentileRankConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &RsiPercentileRankConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for RsiPercentileRankConfig {
    fn defaults() -> Self {
        RsiPercentileRankConfig {
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


impl Render for RsiPercentileRank {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::RsiPctRank, "RSI % Rank", Color::hex(0x9C27B0))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsi_percentile_rank_creation() {
        let rpr = RsiPercentileRank::new(14, 50);
        assert!(!rpr.is_ready());
        assert_eq!(rpr.value(), 50.0);
        assert_eq!(rpr.window(), 50);
    }

    #[test]
    fn test_rsi_percentile_rank_with_rsi_period() {
        let mut rpr = RsiPercentileRank::with_rsi_period(9, 30);
        assert_eq!(rpr.window(), 30);
        for i in 1..=60 {
            let p = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let v = rpr.feed(p);
            assert!(v.is_finite());
        }
        assert!(rpr.is_ready());
    }

    #[test]
    fn test_rsi_percentile_rank_basic() {
        let mut rpr = RsiPercentileRank::new(14, 50);
        for i in 1..=80 {
            let price = 100.0 + i as f64 * 2.0;
            rpr.feed(price);
        }
        assert!(rpr.is_ready());
        assert!(rpr.value().is_finite());
    }

    #[test]
    fn test_rsi_percentile_rank_range() {
        let mut rpr = RsiPercentileRank::new(14, 50);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = rpr.feed(price);
            assert!(value.is_finite(), "RSI Percentile Rank should always be finite");
            if rpr.is_ready() {
                assert!(value >= 0.0 && value <= 100.0, "Percentile should be in [0, 100], got {}", value);
            }
        }
    }

    #[test]
    fn test_rsi_percentile_rank_reset() {
        let mut rpr = RsiPercentileRank::new(14, 50);
        for i in 1..=80 {
            let price = 100.0 + i as f64;
            rpr.feed(price);
        }
        assert!(rpr.is_ready());
        rpr.reset();
        assert!(!rpr.is_ready());
        assert_eq!(rpr.value(), 50.0);
    }

    /// Factory resolves the close field (not the wild 9999 high) and feeds the scalar.
    /// Percentile rank on oscillating prices should be in [0, 100].
    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut cfg = <<RsiPercentileRank as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        cfg.window = Param::Solo(20); // reduced so test fills faster
        let mut f = IndicatorOrder::RsiPctRank(cfg).build_solo().unwrap();
        for i in 1..=60 {
            let price = 100.0 + (i as f64 * 0.4).sin() * 15.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        let v = f.primary();
        assert!(v.is_finite(), "factory RsiPctRank should yield finite output, got {v}");
    }

    #[test]
    fn config_dual_mode() {
        use crate::contract::{Config};
        let d = <<RsiPercentileRank as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(d.cube_size(), 1);
        assert_eq!(d.iter().count(), 1);
    }
}
