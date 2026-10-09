// RSI Z-Score over rolling window

use crate::indicators::momentum::rsi::Rsi;

/// RSI Z-Score — standardizes the RSI value within a rolling window.
///
/// `z = (RSI - mean(RSI, window)) / std(RSI, window)`
///
/// Identifies when RSI is statistically extreme relative to its own recent
/// distribution, rather than against fixed overbought/oversold levels.
#[derive(Debug, Clone)]
pub struct RsiZscore {
    rsi: Rsi,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    sum: f64,
    sumsq: f64,
    z: f64,
}

impl RsiZscore {
    pub fn new(rsi_period: usize, window: usize) -> Self {
        let w = window.max(2);
        Self {
            rsi: Rsi::new(rsi_period),
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            sum: 0.0,
            sumsq: 0.0,
            z: 0.0,
        }
    }

    /// Alias exposing the RSI period parameter explicitly.
    ///
    /// # Arguments
    /// * `rsi_period` - RSI lookback period
    /// * `window`     - Rolling z-score window (minimum 2)
    #[inline]
    pub fn with_rsi_period(rsi_period: usize, window: usize) -> Self {
        Self::new(rsi_period, window)
    }

    #[inline]
    pub fn reset(&mut self) {
        self.rsi.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.sum = 0.0;
        self.sumsq = 0.0;
        self.z = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.rsi.is_ready()
    }

    /// Feed a pre-resolved scalar — the contracted entry point.
    ///
    /// The inner RSI is driven via the close slot; math is byte-identical to the
    /// legacy `update_bar` which passed all OHLCV fields to RSI (which defaulted to close).
    pub fn feed(&mut self, value: f64) -> f64 {
        let v = self.rsi.feed(value);
        let old = self.buf[self.idx];
        self.buf[self.idx] = v;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        self.sum += v - old;
        self.sumsq += v * v - old * old;
        let n = if self.filled {
            self.window as f64
        } else {
            self.idx as f64
        };
        if n >= 2.0 {
            let mean = self.sum / n;
            let var = (self.sumsq / n) - mean * mean;
            let std = if var > 0.0 { var.sqrt() } else { 0.0 };
            self.z = if std > 1e-12 { (v - mean) / std } else { 0.0 };
        } else {
            self.z = 0.0;
        }
        self.z
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.z
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

impl Default for RsiZscore {
    fn default() -> Self {
        Self::with_rsi_period(14, 100)
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

/// Typed config for [`RsiZscore`] — RSI period + RSI smoother + rolling z-score window + source.
///
/// Dual-mode: every field is a `Param`. The RSI smoother follows the `rsi_period` field by default
/// (`follow(Rma)` = Wilder's original). Inner `Rsi` is built via `Rsi::from_choice(choice, period)`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct RsiZscoreConfig {
    /// RSI lookback period.
    pub rsi_period: Param<usize>,
    /// Rolling window for z-score (minimum 2).
    pub window: Param<usize>,
    /// Configurable price source (default close).
    pub source: Param<OhlcvField>,
    /// RSI gain/loss smoother choice — default `follow(Rma)` (Wilder's RMA at `rsi_period`).
    #[slot]
    pub rsi_smoother: Param<SmootherChoice>,
}

impl Indicator for RsiZscore {
    const ID: IndicatorId = IndicatorId::RsiZscore;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// Rolling Vec for the RSI value buffer; inner RSI declared as a Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[Store::window(StoreKind::Vec)],  // buf (RSI values)
        inner: &[Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi])],
    };
    const SLOTS: &'static [crate::contract::Slot] = RsiZscoreConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::RsiZscore)];

    type Config = RsiZscoreConfig;
    type Runtime = RsiZscore;

    fn create(cfg: RsiZscoreConfig) -> RsiZscore {
        let rsi_period = cfg.rsi_period.resolved();
        let window = cfg.window.resolved().max(2);
        let choice = cfg.rsi_smoother.resolved();
        RsiZscore {
            rsi: Rsi::from_choice(choice, rsi_period),
            window,
            buf: vec![0.0; window],
            idx: 0,
            filled: false,
            sum: 0.0,
            sumsq: 0.0,
            z: 0.0,
        }
    }

    fn source_fields(cfg: &RsiZscoreConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &RsiZscoreConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for RsiZscoreConfig {
    fn defaults() -> Self {
        RsiZscoreConfig {
            rsi_period: Param::Solo(14),
            window: Param::Solo(100),
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


impl Render for RsiZscore {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::RsiZscore, "RSI Z-Score", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsi_zscore_creation() {
        let rz = RsiZscore::new(14, 20);
        assert!(!rz.is_ready());
        assert_eq!(rz.value(), 0.0);
        assert_eq!(rz.window(), 20);
    }

    #[test]
    fn test_rsi_zscore_with_rsi_period() {
        let mut rz = RsiZscore::with_rsi_period(9, 15);
        assert_eq!(rz.window(), 15);
        for i in 1..=40 {
            let p = 100.0 + i as f64 * 0.5;
            let v = rz.feed(p);
            assert!(v.is_finite());
        }
        assert!(rz.is_ready());
    }

    #[test]
    fn test_rsi_zscore_basic() {
        let mut rz = RsiZscore::new(14, 20);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            rz.feed(price);
        }
        assert!(rz.is_ready());
        assert!(rz.value().is_finite());
    }

    #[test]
    fn test_rsi_zscore_reset() {
        let mut rz = RsiZscore::new(14, 20);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            rz.feed(price);
        }
        assert!(rz.is_ready());
        rz.reset();
        assert!(!rz.is_ready());
        assert_eq!(rz.value(), 0.0);
    }

    #[test]
    fn test_rsi_zscore_finite_values() {
        let mut rz = RsiZscore::new(14, 20);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = rz.feed(price);
            assert!(value.is_finite(), "RSI Zscore should always be finite");
        }
    }

    /// Factory resolves the close field (not the wild 9999 high) and feeds the scalar.
    /// RSI z-score on a sinusoidal close should be finite throughout.
    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<RsiZscore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::RsiZscore(cfg).build_solo().unwrap();
        for i in 1..=150 {
            let price = 100.0 + (i as f64 * 0.4).sin() * 20.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(
            f.primary().is_finite(),
            "factory RsiZscore should produce finite output, got {}",
            f.primary()
        );
    }

    #[test]
    fn config_dual_mode() {
        use crate::contract::{Config};
        let d = <<RsiZscore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(d.cube_size(), 1);

        let swept = RsiZscoreConfig {
            rsi_period: Param::range(7, 21, 7),
            window: Param::Solo(100),
            source: Param::Solo(OhlcvField::Close),
            rsi_smoother: Param::many(vec![
                SmootherChoice::follow(SmootherId::Rma),
                SmootherChoice::follow(SmootherId::Ema),
            ]),
        };
        assert_eq!(swept.cube_size(), 6); // 3 periods × 2 smoothers
    }
}
