//! Keltner Stop - динамические уровни на основе каналов Кельтнера
//!
//! Вычисляет уровни на основе каналов Кельтнера:
//! - Средняя линия: MA типичной цены (HLC/3)
//! - Верхняя полоса: MA + (ATR × multiplier)
//! - Нижняя полоса: MA - (ATR × multiplier)
//!
//! Индикатор НЕ содержит логику стопов — только возвращает полосы каналов.
//! Основной (контрактный) выход — нижняя полоса (уровень для лонг позиций).

use crate::indicators::volatility::atr::Atr;
use crate::engine::contract_engine::{SmootherId, SmootherSlot};

/// Keltner Stop индикатор — уровни на основе каналов Кельтнера
#[derive(Debug, Clone)]
pub struct KeltnerStop {
    period: usize,
    multiplier: f64,

    // Smoothers
    price_ma: SmootherSlot,
    atr: Atr,

    // Current values
    middle_line: f64,
    upper_band: f64,
    lower_band: f64,

    // State
    bars_count: usize,
    is_ready: bool,
}

impl KeltnerStop {
    /// Default ctor: period=20, multiplier=2.0, center=EMA, atr=RMA.
    pub fn new() -> Self {
        Self::from_smoothers(SmootherId::Ema, SmootherId::Rma, 20, 2.0)
    }

    /// Build from narrow `SmootherId`s for the center MA and the ATR smoother.
    /// Legacy bridge; the contract path goes through `KeltnerStopConfig`.
    pub fn from_smoothers(
        ma_id: SmootherId,
        atr_id: SmootherId,
        period: usize,
        multiplier: f64,
    ) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            multiplier,
            price_ma: SmootherSlot::new(ma_id, p),
            atr: Atr::from_smoother(p, atr_id),
            middle_line: 0.0,
            upper_band: 0.0,
            lower_band: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Feed a bar [high, low, close] (matches `const SOURCE` order: H/L/C).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];

        self.bars_count += 1;

        // Typical price HLC/3
        let typical_price = (high + low + close) / 3.0;

        // Center MA of typical price
        self.middle_line = self.price_ma.feed(typical_price);

        // ATR (uses H/L/C internally via update_bar bridge)
        let atr_value = self.atr.feed(&[high, low, close]);

        // Channel bands
        let offset = atr_value * self.multiplier;
        self.upper_band = self.middle_line + offset;
        self.lower_band = self.middle_line - offset;

        self.is_ready = self.bars_count >= self.period && self.atr.is_ready();

        self.lower_band
    }

    pub fn lower_band(&self) -> f64 { self.lower_band }
    pub fn upper_band(&self) -> f64 { self.upper_band }
    pub fn middle_line(&self) -> f64 { self.middle_line }

    /// Contract output: the lower band (long stop level).
    pub fn value(&self) -> f64 {
        self.lower_band
    }

    pub fn is_ready(&self) -> bool { self.is_ready }

    pub fn reset(&mut self) {
        self.price_ma.reset();
        self.atr.reset();
        self.middle_line = 0.0;
        self.upper_band = 0.0;
        self.lower_band = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
}

impl Default for KeltnerStop {
    fn default() -> Self { Self::new() }
}

// ── Contract ────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::{IndicatorOutputId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity, sweep_f64};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`KeltnerStop`] — period, multiplier, center-MA slot, ATR slot.
/// ABSORB: builds `into_slot()` for center MA and `Atr::from_smoother` for the ATR — no child config.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct KeltnerStopConfig {
    pub period: Param<usize>,
    pub multiplier: Param<f64>,
    /// Center-price MA smoother order — default `Ema` at the host `period`.
    #[slot]
    pub ma_smoother: Param<SmootherSlotOrder>,
    /// ATR smoother order — default `Rma` at the host `period`.
    #[slot]
    pub atr_smoother: Param<SmootherSlotOrder>,
}

impl Indicator for KeltnerStop {
    const ID: IndicatorId = IndicatorId::Kelts;
    /// FLAG: uses channel math internally but emits a single stop level — not a pluggable
    /// channel family member. Using `&[]` until confirmed otherwise.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/C lanes — needed for typical price + ATR.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// O(1) per bar: center MA + ATR are driven through slots.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = KeltnerStopConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Kelts)];
    type Config = KeltnerStopConfig;
    type Runtime = KeltnerStop;

    fn create(cfg: KeltnerStopConfig) -> KeltnerStop {
        let period = cfg.period.resolved().max(1);
        let ma_order = cfg.ma_smoother.resolved();
        let atr_order = cfg.atr_smoother.resolved();
        KeltnerStop {
            period,
            multiplier: cfg.multiplier.resolved(),
            price_ma: ma_order.into_slot(),
            atr: Atr::from_smoother(atr_order.period(), atr_order.id()),
            middle_line: 0.0,
            upper_band: 0.0,
            lower_band: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    fn slot_members(cfg: &KeltnerStopConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for KeltnerStopConfig {
    fn defaults() -> Self {
        KeltnerStopConfig {
            period: Param::Solo(20),
            multiplier: Param::Solo(2.0),
            ma_smoother: Param::Solo(SmootherSlotOrder::Ema(PeriodConfig { period: 20 })),
            atr_smoother: Param::Solo(SmootherSlotOrder::Rma(PeriodConfig { period: 20 })),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1)
        s.multiplier = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for KeltnerStop {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Kelts, "Keltner Stop", Color::hex(0x009688))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keltner_stop_creation() {
        let ind = KeltnerStop::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_keltner_stop_warmup() {
        let mut ind = KeltnerStop::from_smoothers(SmootherId::Ema, SmootherId::Rma, 10, 2.0);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_keltner_stop_values_finite() {
        let mut ind = KeltnerStop::from_smoothers(SmootherId::Ema, SmootherId::Rma, 10, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let lower = ind.feed(&[price + 1.0, price - 1.0, price]);
            assert!(lower.is_finite());
            assert!(ind.upper_band().is_finite());
            assert!(ind.middle_line().is_finite());
        }
    }

    #[test]
    fn test_keltner_stop_reset() {
        let mut ind = KeltnerStop::from_smoothers(SmootherId::Ema, SmootherId::Rma, 10, 2.0);
        for i in 0..20 {
            ind.feed(&[105.0, 95.0, 100.0 + i as f64]);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_kelts() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<KeltnerStop as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Kelts(cfg).build_solo().unwrap();
        for i in 0..30 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 1000.0,
            });
        }
        // lower band (the contract output) must be finite
        assert!(f.primary().is_finite());
    }
}
