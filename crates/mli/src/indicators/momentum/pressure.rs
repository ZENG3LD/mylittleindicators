// High-performance Pressure indicator
// (c) 2024

use crate::indicators::volatility::atr::Atr;
use crate::engine::contract_engine::{SmootherId, SmootherSlot};

/// Buy/sell pressure oscillator.
///
/// Computes per-bar normalized pressure:
/// ```text
/// rel_volume    = volume / avg_volume
/// buy_pressure  = ((close - low)  / atr) * rel_volume
/// sell_pressure = ((high - close) / atr) * rel_volume
/// value         = buy_pressure - sell_pressure
/// ```
///
/// ATR smoothing shape is configurable via `atr_smoother`; volume-average shape via
/// `volume_smoother`. Both are plain [`SmootherSlot`]s fed pre-extracted scalars.
#[derive(Debug, Clone)]
pub struct Pressure {
    atr: Atr,
    avg_volume_ma: SmootherSlot,
    value: f64,
    value_cumulative: f64,
}

impl Pressure {
    /// Build from two explicit [`SmootherId`]s and a common period.
    pub fn from_smoothers(
        period: usize,
        volume_smoother: SmootherId,
        atr_smoother: SmootherId,
    ) -> Self {
        Self {
            atr: Atr::from_smoother(period, atr_smoother),
            avg_volume_ma: SmootherSlot::new(volume_smoother, period),
            value: 0.0,
            value_cumulative: 0.0,
        }
    }

    /// Feed the resolved `[high, low, close, volume]` lanes (in `SOURCE` order). ATR needs
    /// H/L/C; the pressure ratio and volume-average need H/L/C/V.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let volume = lanes[3];
        self.atr.feed(&[high, low, close]);
        // Volume field extraction is a one-liner — no wrapper type needed.
        let vol = volume;
        self.avg_volume_ma.feed(vol);

        if !self.atr.is_ready() || !self.avg_volume_ma.is_ready() {
            self.value = 0.0;
        } else {
            let atr = self.atr.value();
            let avg_volume = self.avg_volume_ma.value();

            if avg_volume.abs() < 1e-12 || atr.abs() < 1e-12 {
                self.value = 0.0;
            } else {
                let rel_volume = vol / avg_volume;
                let buy_pressure = ((close - low) / atr) * rel_volume;
                let sell_pressure = ((high - close) / atr) * rel_volume;
                self.value = buy_pressure - sell_pressure;
            }
        }
        self.value_cumulative += self.value;
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    #[inline]
    pub fn value_cumulative(&self) -> f64 {
        self.value_cumulative
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.atr.is_ready() && self.avg_volume_ma.is_ready()
    }

    pub fn reset(&mut self) {
        self.atr.reset();
        self.avg_volume_ma.reset();
        self.value = 0.0;
        self.value_cumulative = 0.0;
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed contract config for [`Pressure`].
///
/// Two independent smoother slots:
/// - `atr_smoother`: shape of ATR's internal MA (default Follow(RMA) / Wilder).
/// - `volume_smoother`: shape of the average-volume MA (default Follow(SMA)).
///
/// A shared `period` drives both slots via `Follow` semantics.
///
/// Dual-mode: every field is a `Param`. Each `#[slot]` smoother is a `Param<SmootherChoice>`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct PressureConfig {
    /// Shared base lookback period for ATR and volume averaging.
    pub period: Param<usize>,
    /// ATR smoother shape — Wilder's RMA (follow host period) by default.
    #[slot]
    pub atr_smoother: Param<SmootherChoice>,
    /// Volume-average smoother shape — SMA (follow host period) by default.
    #[slot]
    pub volume_smoother: Param<SmootherChoice>,
}

impl Indicator for Pressure {
    const ID: IndicatorId = IndicatorId::Pressure;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Pressure consumes H/L/C/V: ATR needs H/L/C, volume-average needs V, and the
    /// pressure ratio needs H/L/C/V. Bound to the H/L/C/V slices (`Fields` flavor; the
    /// factory feeds these resolved lanes in order).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        crate::engine::ohlcv_field::OhlcvField::High,
        crate::engine::ohlcv_field::OhlcvField::Low,
        crate::engine::ohlcv_field::OhlcvField::Close,
        crate::engine::ohlcv_field::OhlcvField::Volume,
    ]));
    /// O(1): two smoother feeds + a handful of arithmetic ops. Smoother buffer cost
    /// lands recursively through `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = PressureConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Pressure)];
    type Config = PressureConfig;
    type Runtime = Pressure;

    fn create(cfg: PressureConfig) -> Pressure {
        let period = cfg.period.resolved();
        let atr_choice = cfg.atr_smoother.resolved();
        Pressure {
            atr: Atr::from_smoother(atr_choice.period.resolve(period), atr_choice.id()),
            avg_volume_ma: cfg.volume_smoother.resolved().build(period),
            value: 0.0,
            value_cumulative: 0.0,
        }
    }

    fn slot_members(cfg: &PressureConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for PressureConfig {
    fn defaults() -> Self {
        PressureConfig {
            period: Param::Solo(14),
            atr_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
            volume_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1).
        // atr_smoother/volume_smoother: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for Pressure {
    fn rendering() -> RenderSpec {
        use crate::contract::RenderOutput;
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(IndicatorOutputId::Pressure, "Pressure", Color::hex(0x2196F3)))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

impl Default for Pressure {
    fn default() -> Self {
        Pressure::from_smoothers(14, SmootherId::Sma, SmootherId::Rma)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pressure_creation() {
        let p = Pressure::default();
        assert!(!p.is_ready());
        assert_eq!(p.value(), 0.0);
    }

    #[test]
    fn test_pressure_with_smoothers() {
        let mut p = Pressure::from_smoothers(10, SmootherId::Ema, SmootherId::Ema);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 0.5;
            let v = p.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(v.is_finite());
        }
        assert!(p.is_ready());
    }

    #[test]
    fn test_pressure_bullish() {
        let mut p = Pressure::from_smoothers(10, SmootherId::Ema, SmootherId::Rma);
        for i in 1..=30 {
            let low = 100.0 + i as f64;
            let high = low + 5.0;
            let close = high - 0.5; // close near high = bullish
            p.feed(&[high, low, close, 1000.0]);
        }
        assert!(p.is_ready());
        assert!(p.value() > 0.0, "Pressure should be positive when close near high, got {}", p.value());
    }

    #[test]
    fn test_pressure_bearish() {
        let mut p = Pressure::from_smoothers(10, SmootherId::Ema, SmootherId::Rma);
        for i in 1..=30 {
            let high = 200.0 - i as f64;
            let low = high - 5.0;
            let close = low + 0.5; // close near low = bearish
            p.feed(&[high, low, close, 1000.0]);
        }
        assert!(p.is_ready());
        assert!(p.value() < 0.0, "Pressure should be negative when close near low, got {}", p.value());
    }

    #[test]
    fn test_pressure_reset() {
        let mut p = Pressure::from_smoothers(10, SmootherId::Ema, SmootherId::Rma);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            p.feed(&[price + 2.0, price - 2.0, price + 1.0, 1000.0]);
        }
        assert!(p.is_ready());
        p.reset();
        assert!(!p.is_ready());
        assert_eq!(p.value(), 0.0);
        assert_eq!(p.value_cumulative(), 0.0);
    }

    #[test]
    fn test_pressure_finite_values() {
        let mut p = Pressure::from_smoothers(10, SmootherId::Sma, SmootherId::Rma);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = p.feed(&[price + 2.0, price - 2.0, price + 1.0, 1000.0]);
            assert!(value.is_finite(), "Pressure should always be finite");
        }
    }

    #[test]
    fn test_pressure_contract_create() {
        let cfg = <<Pressure as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut p = <Pressure as Indicator>::create(cfg);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            p.feed(&[price + 2.0, price - 2.0, price, 1000.0]);
        }
        assert!(p.is_ready());
    }
}
