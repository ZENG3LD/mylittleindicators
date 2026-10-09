//! ATR-Normalized RSI — RSI scaled by the ratio of current ATR to its rolling average.
//!
//! Formula: ATR-RSI = RSI × sqrt(ATR / ATR_MA)
//!
//! Reuses the contracted `Atr` core for all H/L/C/V computation.
//! Configurable: RSI period, ATR period, and the MA used for ATR smoothing.

use crate::indicators::volatility::atr::Atr;
use crate::engine::contract_engine::SmootherSlot;

/// ATR-Normalized RSI result tuple.
#[derive(Debug, Clone, Copy)]
pub struct AtrRsiResult {
    /// ATR-normalized RSI (0-100 clamped).
    pub atr_rsi: f64,
    /// Plain Wilder RSI used as the base (0-100).
    pub raw_rsi: f64,
    /// Current ATR value.
    pub atr_value: f64,
    /// Ratio ATR / ATR_MA; used for scaling.
    pub atr_ratio: f64,
    /// Volatility regime: 1 = high, 0 = normal, -1 = low.
    pub volatility_regime: i8,
    /// Signal strength (0–1).
    pub signal_strength: f64,
}

impl AtrRsiResult {
    pub fn empty() -> Self {
        Self {
            atr_rsi: 50.0,
            raw_rsi: 50.0,
            atr_value: 0.0,
            atr_ratio: 1.0,
            volatility_regime: 0,
            signal_strength: 0.0,
        }
    }

    /// Volatility regime label.
    pub fn volatility_regime_name(&self) -> &'static str {
        match self.volatility_regime {
            1 => "High Volatility",
            -1 => "Low Volatility",
            _ => "Normal Volatility",
        }
    }

    /// Market condition label based on normalized RSI level.
    pub fn market_condition(&self) -> &'static str {
        let (low, high) = match self.volatility_regime {
            1 => (25.0, 75.0),
            -1 => (35.0, 65.0),
            _ => (30.0, 70.0),
        };
        if self.atr_rsi <= low {
            "Oversold"
        } else if self.atr_rsi >= high {
            "Overbought"
        } else {
            "Neutral"
        }
    }
}

/// ATR-Normalized RSI indicator.
///
/// Uses `Atr` (already contracted) for volatility, and its own inline RSI
/// computation (close-only) for the signal. The MA slot controls how the
/// ATR normalization baseline is smoothed.
#[derive(Debug, Clone)]
pub struct AtrRsi {
    /// Contracted ATR for H/L/C/V computation.
    atr: Atr,
    /// MA over ATR values — the normalization baseline.
    atr_ma: SmootherSlot,

    // Inline RSI (close-only Wilder)
    rsi_period: usize,
    gains: Vec<f64>,
    losses: Vec<f64>,
    avg_gain: f64,
    avg_loss: f64,
    prev_close: Option<f64>,

    // Volatility regime thresholds
    high_vol_threshold: f64,
    low_vol_threshold: f64,

    current_result: AtrRsiResult,
    is_ready: bool,
    update_count: usize,
}

impl AtrRsi {
    /// Default: RSI(14), ATR(14) with EMA(50) baseline.
    pub fn new() -> Self {
        Self::from_smoother(14, 14, SmootherId::Ema, 50)
    }

    /// Construct with explicit MA slot for the ATR baseline.
    pub fn from_smoother(
        rsi_period: usize,
        atr_period: usize,
        ma_id: SmootherId,
        ma_period: usize,
    ) -> Self {
        assert!(rsi_period > 0, "RSI period must be > 0");
        assert!(atr_period > 0, "ATR period must be > 0");
        assert!(ma_period > 0, "MA period must be > 0");
        Self {
            atr: Atr::new_wilder(atr_period),
            atr_ma: SmootherSlot::new(ma_id, ma_period),
            rsi_period,
            gains: Vec::with_capacity(64),
            losses: Vec::with_capacity(64),
            avg_gain: 0.0,
            avg_loss: 0.0,
            prev_close: None,
            high_vol_threshold: 1.5,
            low_vol_threshold: 0.7,
            current_result: AtrRsiResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }

    /// Feed the resolved `[high, low, close]` lanes (in `SOURCE` order). ATR needs H/L/C;
    /// the inline Wilder RSI uses close.
    pub fn feed(&mut self, lanes: &[f64]) -> AtrRsiResult {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let atr_value = self.atr.feed(&[high, low, close]);
        let atr_ma_value = self.atr_ma.feed(atr_value);

        let raw_rsi = self.calculate_rsi(close);

        let atr_ratio = if atr_ma_value > 0.0 {
            atr_value / atr_ma_value
        } else {
            1.0
        };

        let atr_rsi = (raw_rsi * atr_ratio.sqrt()).clamp(0.0, 100.0);

        let volatility_regime = if atr_ratio >= self.high_vol_threshold {
            1_i8
        } else if atr_ratio <= self.low_vol_threshold {
            -1_i8
        } else {
            0_i8
        };

        let signal_strength = {
            let extremity = if atr_rsi <= 30.0 {
                (30.0 - atr_rsi) / 30.0
            } else if atr_rsi >= 70.0 {
                (atr_rsi - 70.0) / 30.0
            } else {
                0.0
            };
            (extremity * atr_ratio.min(2.0)).min(1.0)
        };

        self.current_result = AtrRsiResult {
            atr_rsi,
            raw_rsi,
            atr_value,
            atr_ratio,
            volatility_regime,
            signal_strength,
        };

        if self.atr.is_ready() && self.atr_ma.is_ready() && self.gains.len() >= self.rsi_period {
            self.is_ready = true;
        }

        self.prev_close = Some(close);
        self.update_count += 1;
        self.current_result
    }

    fn calculate_rsi(&mut self, close: f64) -> f64 {
        if let Some(prev) = self.prev_close {
            let change = close - prev;
            let gain = if change > 0.0 { change } else { 0.0 };
            let loss = if change < 0.0 { -change } else { 0.0 };

            if self.gains.len() >= self.rsi_period {
                self.gains.remove(0);
            }
            self.gains.push(gain);

            if self.losses.len() >= self.rsi_period {
                self.losses.remove(0);
            }
            self.losses.push(loss);

            if self.gains.len() == self.rsi_period {
                if self.avg_gain == 0.0 && self.avg_loss == 0.0 {
                    self.avg_gain = self.gains.iter().sum::<f64>() / self.rsi_period as f64;
                    self.avg_loss = self.losses.iter().sum::<f64>() / self.rsi_period as f64;
                } else {
                    let alpha = 1.0 / self.rsi_period as f64;
                    self.avg_gain = alpha * gain + (1.0 - alpha) * self.avg_gain;
                    self.avg_loss = alpha * loss + (1.0 - alpha) * self.avg_loss;
                }
                if self.avg_loss == 0.0 {
                    return 100.0;
                }
                let rs = self.avg_gain / self.avg_loss;
                return 100.0 - (100.0 / (1.0 + rs));
            }
        }
        50.0
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.current_result.atr_rsi
    }

    pub fn result(&self) -> AtrRsiResult {
        self.current_result
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    pub fn reset(&mut self) {
        self.atr.reset();
        self.atr_ma.reset();
        self.gains.clear();
        self.losses.clear();
        self.avg_gain = 0.0;
        self.avg_loss = 0.0;
        self.prev_close = None;
        self.current_result = AtrRsiResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }

    #[inline]
    pub fn period(&self) -> usize {
        self.rsi_period
    }

    pub fn update_count(&self) -> usize {
        self.update_count
    }

    pub fn parameters(&self) -> (usize, usize, f64, f64) {
        (self.rsi_period, self.atr.period(), self.high_vol_threshold, self.low_vol_threshold)
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice, SmootherId};
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderSpec};

/// Typed contract config for [`AtrRsi`].
///
/// Dual-mode: every field is a `Param`. The ATR-MA smoother slot follows `atr_ma_period`
/// by default (`follow(Ema)` = EMA at the given period). The inline RSI period is a plain
/// `Param<usize>` field; there is no separate RSI smoother slot (the inline RSI runs
/// its own Wilder math and is not backed by a `SmootherSlot`).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct AtrRsiConfig {
    /// Inline RSI lookback period.
    pub rsi_period: Param<usize>,
    /// ATR lookback period.
    pub atr_period: Param<usize>,
    /// Period for the ATR normalization baseline MA.
    pub atr_ma_period: Param<usize>,
    /// MA smoother for the ATR baseline — default `follow(Ema)` (EMA at `atr_ma_period`).
    #[slot]
    pub atr_ma: Param<SmootherChoice>,
}

impl Indicator for AtrRsi {
    const ID: IndicatorId = IndicatorId::AtrRsi;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed multi-field consumer: needs H/L/C for ATR (volume unused).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High, OhlcvField::Low, OhlcvField::Close,
    ]));
    /// O(1) own update; ATR inner cost declared via Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const SLOTS: &'static [Slot] = AtrRsiConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::AtrRsi)];
    type Config = AtrRsiConfig;
    type Runtime = AtrRsi;

    fn create(cfg: AtrRsiConfig) -> AtrRsi {
        let rsi_period = cfg.rsi_period.resolved();
        let atr_period = cfg.atr_period.resolved();
        let atr_ma_period = cfg.atr_ma_period.resolved();
        let choice = cfg.atr_ma.resolved();
        AtrRsi {
            atr: Atr::new_wilder(atr_period),
            atr_ma: choice.build(atr_ma_period),
            rsi_period,
            gains: Vec::with_capacity(64),
            losses: Vec::with_capacity(64),
            avg_gain: 0.0,
            avg_loss: 0.0,
            prev_close: None,
            high_vol_threshold: 1.5,
            low_vol_threshold: 0.7,
            current_result: AtrRsiResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }

    fn slot_members(cfg: &AtrRsiConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for AtrRsiConfig {
    fn defaults() -> Self {
        AtrRsiConfig {
            rsi_period: Param::Solo(14),
            atr_period: Param::Solo(14),
            atr_ma_period: Param::Solo(50),
            atr_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // rsi_period/atr_period/atr_ma_period: Class A → auto range(2,4048,1).
        // atr_ma: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for AtrRsi {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::AtrRsi, "ATR RSI", Color::hex(0x9C27B0))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(70.0, Color::hex(0xFF5722)))
            .reference_line(ReferenceLine::new(30.0, Color::hex(0x4CAF50)))
            .precision(2)
            .build()
    }
}

impl Default for AtrRsi {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atr_rsi_creation() {
        let r = AtrRsi::new();
        assert!(!r.is_ready());
        assert_eq!(r.period(), 14);
        assert_eq!(r.parameters().0, 14);
    }

    #[test]
    fn test_atr_rsi_update() {
        let mut r = AtrRsi::new();
        for i in 0..100 {
            let price = 100.0 + i as f64 * 0.1;
            let result = r.feed(&[price + 2.0, price - 1.0, price]);
            if i > 20 {
                assert!(result.atr_rsi >= 0.0 && result.atr_rsi <= 100.0);
                assert!(result.raw_rsi >= 0.0 && result.raw_rsi <= 100.0);
                assert!(result.signal_strength >= 0.0 && result.signal_strength <= 1.0);
            }
        }
    }

    #[test]
    fn test_atr_rsi_reset() {
        let mut r = AtrRsi::new();
        for i in 0..100 {
            let price = 100.0 + i as f64 * 0.5;
            r.feed(&[price + 1.0, price - 1.0, price]);
        }
        r.reset();
        assert!(!r.is_ready());
        assert!((r.value() - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_atr_rsi_uptrend_rsi_high() {
        let mut r = AtrRsi::new();
        for i in 0..100 {
            let price = 100.0 + i as f64;
            r.feed(&[price + 2.0, price - 1.0, price + 1.0]);
        }
        if r.is_ready() {
            assert!(r.result().raw_rsi > 50.0);
        }
    }

    #[test]
    fn test_atr_rsi_update_count() {
        let mut r = AtrRsi::new();
        for i in 0..10 {
            let price = 100.0 + i as f64;
            r.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert_eq!(r.update_count(), 10);
    }

    #[test]
    fn factory_feeds_resolved_atr_rsi() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<AtrRsi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::AtrRsi(cfg).build_solo().unwrap();
        for i in 0..100 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: price,
                high: price + 2.0,
                low: price - 1.0,
                close: price,
                volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }

    #[test]
    fn config_dual_mode() {
        use crate::contract::{Config};
        let d = <<AtrRsi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(d.cube_size(), 1);
        assert_eq!(d.iter().count(), 1);

        let swept = AtrRsiConfig {
            rsi_period: Param::range(7, 21, 7),
            atr_period: Param::Solo(14),
            atr_ma_period: Param::range(20, 60, 20),
            atr_ma: Param::many(vec![
                SmootherChoice::follow(SmootherId::Ema),
                SmootherChoice::follow(SmootherId::Sma),
            ]),
        };
        // 3 rsi_period × 3 atr_ma_period × 2 smoothers = 18
        assert_eq!(swept.cube_size(), 18);
    }
}
