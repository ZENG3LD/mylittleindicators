// QQE (Quantitative Qualitative Estimation) — RSI smoothed with ATR-derived bands.
//
// Algorithm:
//   1. Compute RSI(period)
//   2. Smooth RSI with EMA(smooth) → smoothed_rsi
//   3. Compute |smoothed_rsi[i] - smoothed_rsi[i-1]| → rsi_delta
//   4. Smooth rsi_delta with Wilder EMA(smooth*4.236) → atr_rsi
//   5. Upper/lower bands = smoothed_rsi ± threshold_mult * atr_rsi
//   6. QQE line follows bands (trailing stop logic):
//      - rises when smoothed_rsi crosses above upper, falls when below lower
//
// Output: Double(qqe_line, smoothed_rsi)

use crate::engine::contract_engine::{SmootherId, SmootherSlot};
use crate::indicators::momentum::rsi::Rsi;

#[derive(Debug, Clone)]
pub struct Qqe {
    rsi: Rsi,
    /// First EMA smoothing of RSI
    smoothing: SmootherSlot,
    /// Wilder-style smoothing of RSI delta magnitude → ATR-like band width
    atr_smooth: SmootherSlot,
    threshold_mult: f64,
    qqe_value: f64,
    smoothed_rsi: f64,
    prev_smoothed_rsi: f64,
    atr_rsi: f64,
    /// QQE long/short trailing level
    qqe_upper: f64,
    qqe_lower: f64,
    /// Long band is trailing below RSI, short band trailing above
    is_long: bool,
}

impl Qqe {
    pub fn new(period: usize, smooth: usize, threshold_mult: f64) -> Self {
        Self::from_smoothers(period, smooth, threshold_mult, SmootherId::Ema, SmootherId::Rma)
    }

    /// Create QQE with explicit smoother IDs for RSI-smoothing and ATR-band smoothing.
    pub fn from_smoothers(
        period: usize,
        smooth: usize,
        threshold_mult: f64,
        smooth_id: SmootherId,
        atr_id: SmootherId,
    ) -> Self {
        let p = period.max(1);
        let s = smooth.max(1);
        let tm = if threshold_mult > 0.0 { threshold_mult } else { 1.5 };
        // Wilder ATR period = smooth * 4.236 (classic QQE constant)
        let atr_period = ((s as f64 * 4.236).round() as usize).max(2);
        Self {
            rsi: Rsi::new(p),
            smoothing: SmootherSlot::new(smooth_id, s),
            atr_smooth: SmootherSlot::new(atr_id, atr_period),
            threshold_mult: tm,
            qqe_value: 0.0,
            smoothed_rsi: 0.0,
            prev_smoothed_rsi: 0.0,
            atr_rsi: 0.0,
            qqe_upper: 0.0,
            qqe_lower: 100.0,
            is_long: true,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.qqe_value = 0.0;
        self.smoothed_rsi = 0.0;
        self.prev_smoothed_rsi = 0.0;
        self.atr_rsi = 0.0;
        self.qqe_upper = 0.0;
        self.qqe_lower = 100.0;
        self.is_long = true;
        self.smoothing.reset();
        self.atr_smooth.reset();
        self.rsi.reset();
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.smoothing.is_ready() && self.atr_smooth.is_ready()
    }


    /// Named output: brace `line` (the QQE oscillator line).
    #[inline]
    pub fn line(&self) -> f64 {
        self.qqe_value
    }

    /// Feed ONE pre-extracted scalar (close).
    pub fn feed(&mut self, close: f64) -> f64 {
        // RSI still uses update_bar internally; feed close for all fields
        let _ = self.rsi.feed(close);
        let rsi_val = self.rsi.value();

        self.prev_smoothed_rsi = self.smoothed_rsi;
        self.smoothed_rsi = self.smoothing.feed(rsi_val);

        // RSI delta magnitude for ATR-like smoothing
        let rsi_delta = (self.smoothed_rsi - self.prev_smoothed_rsi).abs();
        self.atr_rsi = self.atr_smooth.feed(rsi_delta);

        let band_width = self.threshold_mult * self.atr_rsi;

        // Trailing QQE line (long/short band)
        if self.is_long {
            let new_lower = self.smoothed_rsi - band_width;
            if new_lower > self.qqe_lower {
                self.qqe_lower = new_lower;
            }
            if self.smoothed_rsi < self.qqe_lower {
                self.is_long = false;
                self.qqe_upper = self.smoothed_rsi + band_width;
                self.qqe_value = self.qqe_upper;
            } else {
                self.qqe_value = self.qqe_lower;
            }
        } else {
            let new_upper = self.smoothed_rsi + band_width;
            if new_upper < self.qqe_upper {
                self.qqe_upper = new_upper;
            }
            if self.smoothed_rsi > self.qqe_upper {
                self.is_long = true;
                self.qqe_lower = self.smoothed_rsi - band_width;
                self.qqe_value = self.qqe_lower;
            } else {
                self.qqe_value = self.qqe_upper;
            }
        }

        self.qqe_value
    }

    pub fn threshold_mult(&self) -> f64 {
        self.threshold_mult
    }

    /// Brace-named getter: `smoothed` output (smoothed RSI line).
    #[inline]
    pub fn smoothed(&self) -> f64 {
        self.smoothed_rsi
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, UpdateComplexity};
use crate::contract::axis::sweep_f64;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};

/// Typed dual-mode config for [`Qqe`].
///
/// `rsi_period` sizes the inner RSI; `smooth_period` drives the RSI-smoothing EMA;
/// `threshold_mult` is the ATR-band multiplier. `smooth` selects the RSI-smoothing member
/// (default `follow(Ema)` at `smooth_period`); `atr_smooth` selects the ATR-band member
/// (default `follow(Rma)` — the classic Wilder smoother). The ATR period is always derived
/// from `smooth_period × 4.236` (classic QQE constant).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct QqeConfig {
    pub rsi_period: Param<usize>,
    pub smooth_period: Param<usize>,
    pub threshold_mult: Param<f64>,
    /// RSI first-smoothing slot — default `follow(Ema)` at `smooth_period`.
    #[slot]
    pub smooth: Param<SmootherChoice>,
    /// ATR-band smoothing slot — default `follow(Rma)` (Wilder; period derived from smooth×4.236).
    #[slot]
    pub atr_smooth: Param<SmootherChoice>,
}

impl Indicator for Qqe {
    const ID: IndicatorId = IndicatorId::Qqe;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Single configurable price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1) own update; RSI cost via Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Rsi, &[IndicatorOutputId::Rsi])],
    };
    const SLOTS: &'static [Slot] = QqeConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::QqeLine),
        Output::percent(IndicatorOutputId::QqeSmoothed),
    ];
    type Config = QqeConfig;
    type Runtime = Qqe;

    fn create(cfg: QqeConfig) -> Qqe {
        let rsi_period = cfg.rsi_period.resolved();
        let smooth_period = cfg.smooth_period.resolved();
        let threshold_mult = cfg.threshold_mult.resolved();
        let smooth_choice = cfg.smooth.resolved();
        let atr_choice = cfg.atr_smooth.resolved();
        // Classic QQE: atr period is derived from smooth_period × 4.236 (overrides Own period).
        let atr_period = ((smooth_period as f64 * 4.236).round() as usize).max(2);
        let p = rsi_period.max(1);
        let s = smooth_period.max(1);
        let tm = if threshold_mult > 0.0 { threshold_mult } else { 1.5 };
        Qqe {
            rsi: Rsi::new(p),
            smoothing: smooth_choice.build(s),
            atr_smooth: SmootherSlot::new(atr_choice.kind, atr_period),
            threshold_mult: tm,
            qqe_value: 0.0,
            smoothed_rsi: 0.0,
            prev_smoothed_rsi: 0.0,
            atr_rsi: 0.0,
            qqe_upper: 0.0,
            qqe_lower: 100.0,
            is_long: true,
        }
    }

    fn source_fields(cfg: &QqeConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }

    fn slot_members(cfg: &QqeConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for QqeConfig {
    fn defaults() -> Self {
        // Classic QQE: RSI(14), smooth EMA(5), atr RMA(derived ~21), mult=4.236
        QqeConfig {
            rsi_period: Param::Solo(14),
            smooth_period: Param::Solo(5),
            threshold_mult: Param::Solo(4.236),
            smooth: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            atr_smooth: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // rsi_period/smooth_period: Class A → auto range(2,4048,1).
        // threshold_mult: Class C (multiplier, typical range 0.1..10.0) — sweep_f64(0.1,10.0,0.1).
        // smooth/atr_smooth: #[slot] SmootherChoice → left Solo (deferred wave).
        let mut s = Self::machine_defaults_auto();
        s.threshold_mult = Param::many(sweep_f64(0.1, 10.0, 0.1));
        s
    }
}


impl Render for Qqe {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::QqeLine, "QQE", Color::hex(0x2196F3), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::QqeSmoothed, "Smoothed RSI", Color::hex(0xFF9800), 1.0))
            .precision(2)
            .build()
    }
}

impl Default for Qqe {
    fn default() -> Self {
        Self::from_smoothers(14, 5, 1.5, SmootherId::Ema, SmootherId::Rma)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qqe_creation() {
        let qqe = Qqe::new(14, 5, 4.236);
        assert!(!qqe.is_ready());
        assert_eq!(qqe.line(), 0.0);
        assert!((qqe.threshold_mult() - 4.236).abs() < 1e-10);
    }

    #[test]
    fn test_qqe_default_threshold() {
        let qqe = Qqe::new(14, 5, 0.0);
        assert!((qqe.threshold_mult() - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_qqe_basic_calculation() {
        let mut qqe = Qqe::new(14, 5, 4.236);
        for i in 1..=100 {
            let price = 100.0 + i as f64;
            qqe.feed(price);
        }
        assert!(qqe.is_ready());
        let (qqe_line, smoothed_rsi) = (qqe.line(), qqe.smoothed());
        assert!(qqe_line.is_finite(), "QQE line should be finite");
        assert!(smoothed_rsi.is_finite(), "Smoothed RSI should be finite");
    }

    #[test]
    fn test_qqe_finite_values() {
        let mut qqe = Qqe::new(14, 5, 4.236);
        for i in 1..=200 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            let value = qqe.feed(price);
            assert!(value.is_finite(), "QQE should always be finite");
        }
    }

    #[test]
    fn test_qqe_reset() {
        let mut qqe = Qqe::new(14, 5, 4.236);
        for i in 1..=100 {
            let price = 100.0 + i as f64;
            qqe.feed(price);
        }
        assert!(qqe.is_ready());
        qqe.reset();
        assert!(!qqe.is_ready());
        assert_eq!(qqe.line(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_qqe() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<Qqe as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Qqe(cfg).build_solo().unwrap();
        for i in 1..=100 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
