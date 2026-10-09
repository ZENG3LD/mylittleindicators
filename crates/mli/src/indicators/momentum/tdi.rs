// Traders Dynamic Index (TDI) — RSI with Bollinger Band signal lines.
//
// Algorithm (Dean Malone / published specification):
//   1. rsi_value  = RSI(close, rsi_period)           — raw RSI in [0, 1]
//   2. signal     = MA(rsi_value, signal_period)      — fast signal line
//   3. bb_basis   = SMA(rsi_value, band_period)       — BB midline
//   4. bb_dev     = StdDev(rsi_value, band_period) * 1.6185  — BB width (published constant)
//   5. upper_band = bb_basis + bb_dev
//   6. lower_band = bb_basis - bb_dev
//
// Output: Triple(rsi_value, signal, bb_basis). Bands via upper_band() / lower_band().
// rsi_value is normalised to [0, 1] (Rsi::update_bar convention); all bands share that scale.

use crate::engine::contract_engine::{SmootherId, SmootherSlot};
use crate::indicators::momentum::rsi::Rsi;

#[derive(Debug, Clone)]
pub struct Tdi {
    rsi: Rsi,
    signal_ma: SmootherSlot,
    // Bollinger band on RSI: rolling SMA + stddev
    band_period: usize,
    band_buf: Vec<f64>,
    band_idx: usize,
    band_sum: f64,
    band_sum_sq: f64,
    rsi_value: f64,
    signal_value: f64,
    /// Bollinger band midline (SMA of RSI over band_period).
    band_basis: f64,
    /// Upper Bollinger band (basis + 1.6185 * stddev).
    band_upper: f64,
    /// Lower Bollinger band (basis - 1.6185 * stddev).
    band_lower: f64,
}

impl Tdi {
    /// Default ctor — signal line smoothed with EMA over the inner RSI.
    pub fn new(rsi_period: usize, signal_period: usize, band_period: usize) -> Self {
        Self::from_signal_smoother(rsi_period, signal_period, band_period, SmootherId::Ema)
    }

    /// Build from a narrow `SmootherId` for the SIGNAL smoother + the RSI/band periods.
    /// The inner RSI keeps its own default smoother. Legacy bridge; the contract path goes
    /// through `TdiConfig`.
    pub fn from_signal_smoother(
        rsi_period: usize,
        signal_period: usize,
        band_period: usize,
        signal_smoother: SmootherId,
    ) -> Self {
        let bp = band_period.max(2);
        Self {
            rsi: Rsi::new(rsi_period.max(1)),
            signal_ma: SmootherSlot::new(signal_smoother, signal_period.max(1)),
            band_period: bp,
            band_buf: Vec::with_capacity(bp),
            band_idx: 0,
            band_sum: 0.0,
            band_sum_sq: 0.0,
            rsi_value: 0.5,
            signal_value: 0.5,
            band_basis: 0.5,
            band_upper: 0.5,
            band_lower: 0.5,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.rsi.reset();
        self.signal_ma.reset();
        self.band_buf.clear();
        self.band_idx = 0;
        self.band_sum = 0.0;
        self.band_sum_sq = 0.0;
        self.rsi_value = 0.5;
        self.signal_value = 0.5;
        self.band_basis = 0.5;
        self.band_upper = 0.5;
        self.band_lower = 0.5;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rsi.is_ready()
    }

    #[inline]
    pub fn values(&self) -> (f64, f64, f64) {
        (self.rsi_value, self.signal_value, self.band_basis)
    }

    #[inline]
    pub fn upper_band(&self) -> f64 {
        self.band_upper
    }

    #[inline]
    pub fn lower_band(&self) -> f64 {
        self.band_lower
    }

    /// Brace-named getter: `rsi` output (RSI value, normalised to [0, 1]).
    #[inline]
    pub fn rsi(&self) -> f64 {
        self.rsi_value
    }

    /// Brace-named getter: `signal` output (fast signal line).
    #[inline]
    pub fn signal(&self) -> f64 {
        self.signal_value
    }

    /// Brace-named getter: `basis` output (Bollinger Band midline on RSI).
    #[inline]
    pub fn basis(&self) -> f64 {
        self.band_basis
    }


    /// Feed ONE pre-extracted scalar — the close (const SOURCE = Field{Close}). The inner RSI
    /// is still bar-driven (un-converted); feed it the scalar as the close slot (RSI's source
    /// is Close). Flips to `rsi.feed` when RSI is converted. Knows no transport.
    pub fn feed(&mut self, value: f64) -> (f64, f64, f64) {
        let r = self.rsi.feed(value);
        self.rsi_value = r;
        self.signal_value = self.signal_ma.feed(r);

        // Rolling Bollinger band on RSI
        if self.band_buf.len() < self.band_period {
            self.band_buf.push(r);
            self.band_sum += r;
            self.band_sum_sq += r * r;
        } else {
            let old = self.band_buf[self.band_idx];
            self.band_buf[self.band_idx] = r;
            self.band_sum += r - old;
            self.band_sum_sq += r * r - old * old;
            self.band_idx = (self.band_idx + 1) % self.band_period;
        }

        let n = self.band_buf.len() as f64;
        let mean = self.band_sum / n;
        let variance = (self.band_sum_sq / n - mean * mean).max(0.0);
        let std_dev = variance.sqrt();

        // Published TDI constant: 1.6185 (approximately sqrt(2*pi/2.4) ≈ Malone's choice)
        const TDI_BB_MULT: f64 = 1.6185;
        self.band_basis = mean;
        self.band_upper = mean + TDI_BB_MULT * std_dev;
        self.band_lower = mean - TDI_BB_MULT * std_dev;

        (self.rsi_value, self.signal_value, self.band_basis)
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};

/// Typed dual-mode config for [`Tdi`] — RSI + signal + Bollinger basis (all on the RSI scale).
///
/// `rsi_period` sizes the inner RSI (a fixed Port), `band_period` the BB window, `signal_period`
/// drives the signal-line smoother slot (default `follow(Ema)` at `signal_period`).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct TdiConfig {
    pub rsi_period: Param<usize>,
    pub band_period: Param<usize>,
    /// Signal-line smoothing period (the host period the slot follows by default).
    pub signal_period: Param<usize>,
    /// Signal-line smoother choice — default `follow(Ema)` at `signal_period`.
    #[slot]
    pub signal: Param<SmootherChoice>,
}

impl Indicator for Tdi {
    const ID: IndicatorId = IndicatorId::Tdi;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Operates on close through the inner RSI.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1) running BB moments over a `band_period`-deep heap ring; the inner RSI is a fixed
    /// `Port`, the signal smoother is the configurable `SLOTS` member.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi])],
    };
    const SLOTS: &'static [Slot] = TdiConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::TdiRsi),
        Output::percent(IndicatorOutputId::TdiSignal),
        Output::percent(IndicatorOutputId::TdiBasis),
    ];
    type Config = TdiConfig;
    type Runtime = Tdi;

    fn create(cfg: TdiConfig) -> Tdi {
        let rsi_period = cfg.rsi_period.resolved();
        let band_period = cfg.band_period.resolved();
        let signal_period = cfg.signal_period.resolved();
        let bp = band_period.max(2);
        let choice = cfg.signal.resolved();
        Tdi {
            rsi: Rsi::new(rsi_period.max(1)),
            signal_ma: choice.build(signal_period),
            band_period: bp,
            band_buf: Vec::with_capacity(bp),
            band_idx: 0,
            band_sum: 0.0,
            band_sum_sq: 0.0,
            rsi_value: 0.5,
            signal_value: 0.5,
            band_basis: 0.5,
            band_upper: 0.5,
            band_lower: 0.5,
        }
    }

    fn slot_members(cfg: &TdiConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for TdiConfig {
    fn defaults() -> Self {
        TdiConfig {
            rsi_period: Param::Solo(13),
            band_period: Param::Solo(34),
            signal_period: Param::Solo(2),
            signal: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // rsi_period/band_period/signal_period: Class A → auto range(2,4048,1).
        // signal: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for Tdi {
    fn rendering() -> RenderSpec {
        // RSI values are on the [0, 1] scale (Rsi convention), so the pane bounds match.
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::TdiRsi, "RSI", Color::hex(0x4CAF50), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::TdiSignal, "Signal", Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::TdiBasis, "Baseline", Color::hex(0x2196F3), 1.0))
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tdi_creation() {
        let tdi = Tdi::new(14, 9, 5);
        assert!(!tdi.is_ready());
        // rsi_value = 0.5 on construction, so values() returns (0.5, 0.5, 0.5)
        let (rsi, sig, basis) = tdi.values();
        assert!((rsi - 0.5).abs() < 1e-10);
        assert!((sig - 0.5).abs() < 1e-10);
        assert!((basis - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_tdi_with_signal_smoother() {
        let mut tdi = Tdi::from_signal_smoother(13, 2, 34, SmootherId::Sma);
        for i in 1..=50 {
            let p = 100.0 + i as f64 * 0.5;
            let (rsi, sig, basis) = tdi.feed(p);
            assert!(rsi.is_finite() && sig.is_finite() && basis.is_finite());
        }
        assert!(tdi.is_ready());
    }

    #[test]
    fn test_tdi_uptrend() {
        let mut tdi = Tdi::new(14, 9, 5);
        for i in 1..=40 {
            let price = 100.0 + i as f64 * 2.0;
            tdi.feed(price);
        }
        assert!(tdi.is_ready());
        let (rsi, _, _) = tdi.values();
        assert!(rsi > 0.5, "TDI RSI should be > 0.5 in uptrend, got {}", rsi);
    }

    #[test]
    fn test_tdi_downtrend() {
        let mut tdi = Tdi::new(14, 9, 5);
        for i in 1..=40 {
            let price = 200.0 - i as f64 * 2.0;
            tdi.feed(price);
        }
        assert!(tdi.is_ready());
        let (rsi, _, _) = tdi.values();
        assert!(rsi < 0.5, "TDI RSI should be < 0.5 in downtrend, got {}", rsi);
    }

    #[test]
    fn test_tdi_bands_ordered() {
        let mut tdi = Tdi::new(14, 9, 5);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            tdi.feed(price);
        }
        assert!(tdi.upper_band() >= tdi.lower_band(),
            "Upper band should be >= lower band");
    }

    #[test]
    fn test_tdi_reset() {
        let mut tdi = Tdi::new(14, 9, 5);
        for i in 1..=40 {
            let price = 100.0 + i as f64;
            tdi.feed(price);
        }
        assert!(tdi.is_ready());
        tdi.reset();
        assert!(!tdi.is_ready());
        let (rsi, sig, basis) = tdi.values();
        assert!((rsi - 0.5).abs() < 1e-10);
        assert!((sig - 0.5).abs() < 1e-10);
        assert!((basis - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_tdi_contract_create() {
        let cfg = <<Tdi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.signal_period.resolved(), 2);
        let mut tdi = <Tdi as Indicator>::create(cfg);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            tdi.feed(price);
        }
        assert!(tdi.is_ready());
    }

    /// The factory resolves the configured close field and feeds the scalar; TDI (with its
    /// inner RSI) runs end-to-end (uptrend -> RSI > 0.5).
    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Tdi(<<Tdi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=60 {
            let price = 100.0 + i as f64 * 2.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary() > 0.5, "factory TDI RSI uptrend should be > 0.5, got {}", f.primary());
    }
}
