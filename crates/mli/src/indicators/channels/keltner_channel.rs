//! keltner_channel.rs: High-Performance Keltner Channels
//! Каналы Кельтнера - ATR-based адаптивные каналы
//!
//! Особенности:
//! - Использует готовые MovingAverage и Atr компоненты
//! - 3 режима расчета центральной линии
//! - ALL 19 MA types для центральной линии и ATR
//! - Полная поддержка оптимизации через перебор типов

use crate::engine::contract_engine::SmootherSlot;
use crate::engine::ohlcv_field::OhlcvField;
use crate::indicators::volatility::atr::Atr;

/// Режимы расчета Keltner Channel
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(Default)]
#[derive(mli_contract_macros::ParamScalar)]
pub enum KeltnerMode {
    /// Classic - использует Typical Price (HLC/3) для средней линии
    #[default]
    Classic,
    /// Close - использует Close для средней линии
    Close,
    /// HLC - использует (High + Low + Close) / 3 для средней линии
    HLC,
}

/// High-Performance Keltner Channel
/// Архитектура: SmootherSlot для центральной линии + Atr для полос
#[derive(Debug, Clone)]
pub struct KeltnerChannel {
    // Параметры
    period: usize,
    multiplier: f64,
    mode: KeltnerMode,
    source: OhlcvField,

    // Компоненты (используем готовые!)
    center_ma: SmootherSlot,     // ✅ Центральная линия через SmootherSlot
    atr: Atr,                     // ✅ ATR через готовый компонент

    // Текущие значения канала
    upper: f64,
    middle: f64,
    lower: f64,
}

impl KeltnerChannel {
    /// Создать Keltner Channel с указанными параметрами
    pub fn new_classic(period: usize, multiplier: f64) -> Self {
        Self::new_classic_with_source(period, multiplier, OhlcvField::Close)
    }

    /// Создать Classic Keltner Channel с указанным источником данных
    pub fn new_classic_with_source(period: usize, multiplier: f64, source: OhlcvField) -> Self {
        Self::from_smoothers(
            SmootherId::Sma,
            SmootherId::Rma,
            period,
            multiplier,
            KeltnerMode::Classic,
            source,
        )
    }

    /// Build from narrow `SmootherId`s — legacy bridge for the 4+ derivatives that
    /// still construct KeltnerChannel directly (they have not yet migrated to
    /// `KeltnerConfig`). Keeps the existing call sites compiling without change.
    pub fn from_smoothers(
        center: SmootherId,
        atr_smoother: SmootherId,
        period: usize,
        multiplier: f64,
        mode: KeltnerMode,
        source: OhlcvField,
    ) -> Self {
        let p = period.max(1);
        assert!(multiplier > 0.0, "Multiplier must be positive");
        Self {
            period: p,
            multiplier,
            mode,
            source,
            center_ma: SmootherSlot::new(center, p),
            atr: Atr::from_smoother(p, atr_smoother),
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    /// Feed HIGH, LOW, CLOSE (index order matches `const SOURCE`).
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64, f64) {
        let high   = lanes[0];
        let low    = lanes[1];
        let close  = lanes[2];
        // source field was already resolved by the factory; use the resolved scalar
        // (the factory extracts it via `source_fields`). For standalone (non-factory)
        // use, we compute from the provided close — KeltnerChannel's center defaults
        // to close when called via `feed`.
        let center_price = self.source.extract(0.0, high, low, close, 0.0);
        self.middle = self.center_ma.feed(center_price);
        let atr_value = self.atr.feed(&[high, low, close]);
        if self.is_ready() {
            self.upper = self.middle + self.multiplier * atr_value;
            self.lower = self.middle - self.multiplier * atr_value;
        } else {
            self.upper = 0.0;
            self.lower = 0.0;
        }
        (self.upper, self.middle, self.lower)
    }



    /// Получить текущие значения канала как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper, self.middle, self.lower)
    }

    /// Получить верхнюю границу канала
    pub fn upper(&self) -> f64 {
        self.upper
    }

    /// Получить среднюю линию канала
    pub fn middle(&self) -> f64 {
        self.middle
    }

    /// Получить нижнюю границу канала
    pub fn lower(&self) -> f64 {
        self.lower
    }

    /// Получить ширину канала
    pub fn channel_width(&self) -> f64 {
        if self.is_ready() {
            self.upper - self.lower
        } else {
            0.0
        }
    }

    /// Получить позицию цены в канале (0.0 = нижняя граница, 1.0 = верхняя граница)
    pub fn position_in_channel(&self, price: f64) -> f64 {
        let width = self.channel_width();
        if width > 0.0 {
            (price - self.lower) / width
        } else {
            0.5
        }
    }

    /// Получить расстояние цены от центральной линии в единицах ATR
    pub fn distance_from_center_atr(&self, price: f64) -> f64 {
        if self.is_ready() && self.atr.value() > 0.0 {
            (price - self.middle) / self.atr.value()
        } else {
            0.0
        }
    }

    /// Проверить пробой верхней границы
    pub fn is_upper_breakout(&self, price: f64) -> bool {
        self.is_ready() && price > self.upper
    }

    /// Проверить пробой нижней границы
    pub fn is_lower_breakout(&self, price: f64) -> bool {
        self.is_ready() && price < self.lower
    }

    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.center_ma.is_ready() && self.atr.is_ready()
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.center_ma.reset();
        self.atr.reset();
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }

    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }

    /// Получить множитель
    pub fn multiplier(&self) -> f64 {
        self.multiplier
    }

    /// Получить режим
    pub fn mode(&self) -> KeltnerMode {
        self.mode
    }

    /// Получить текущее значение ATR
    pub fn atr_value(&self) -> f64 {
        self.atr.value()
    }
}

impl Default for KeltnerChannel {
    fn default() -> Self {
        Self::new_classic(20, 2.0)
    }
}

// ─── Contract ────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::{IndicatorOutputId, SmootherId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::{Param, sweep_f64};
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::{Color, Render, RenderOutput, RenderSpec};

/// Typed dual-mode config for [`KeltnerChannel`].
///
/// Two independent smoother slots: center MA (default Sma at period 20) and ATR smoothing
/// (default Rma at period 20). Each slot carries its own period axis, independent of the
/// host `period` field.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct KeltnerConfig {
    pub period: Param<usize>,
    pub multiplier: Param<f64>,
    pub mode: Param<KeltnerMode>,
    /// Center-line smoother slot. Default Sma at the host period (20).
    #[slot]
    pub ma: Param<SmootherSlotOrder>,
    /// ATR smoother slot. Default Rma (Wilder) at the host period (20).
    #[slot]
    pub atr_ma: Param<SmootherSlotOrder>,
}

use crate::contract::Slot;

impl Indicator for KeltnerChannel {
    const ID: IndicatorId = IndicatorId::Kc;
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed High/Low/Close triple for the channel computation.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[],
    };
    const SLOTS: &'static [Slot] = KeltnerConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::KcUpper),
        Output::price(IndicatorOutputId::KcMiddle),
        Output::price(IndicatorOutputId::KcLower),
    ];
    type Config = KeltnerConfig;
    type Runtime = Self;

    fn create(cfg: KeltnerConfig) -> Self {
        let p = cfg.period.resolved().max(1);
        let ma_order = cfg.ma.resolved();
        let atr_order = cfg.atr_ma.resolved();
        Self {
            period: p,
            multiplier: cfg.multiplier.resolved(),
            mode: cfg.mode.resolved(),
            source: OhlcvField::Close,
            center_ma: ma_order.into_slot(),
            atr: Atr::from_smoother(atr_order.period(), atr_order.id()),
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    fn slot_members(cfg: &KeltnerConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for KeltnerConfig {
    /// Period 20, multiplier 2.0, Classic mode, SMA center at period 20, RMA ATR at period 20.
    fn defaults() -> Self {
        KeltnerConfig {
            period: Param::Solo(20),
            multiplier: Param::Solo(2.0),
            mode: Param::Solo(KeltnerMode::Classic),
            ma: Param::Solo(SmootherSlotOrder::Sma(PeriodConfig { period: 20 })),
            atr_ma: Param::Solo(SmootherSlotOrder::Rma(PeriodConfig { period: 20 })),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1)
        s.multiplier = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s.mode = Param::many(vec![
            KeltnerMode::Classic,
            KeltnerMode::Close,
            KeltnerMode::HLC,
        ]); // Class Q enum — all variants
        s
    }
}


impl Render for KeltnerChannel {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::KcUpper, "Upper", Color::hex(0xF44336))
            .line_output(IndicatorOutputId::KcMiddle, "Middle", Color::hex(0x607D8B))
            .line_output(IndicatorOutputId::KcLower, "Lower", Color::hex(0x4CAF50))
            .output(RenderOutput::band(IndicatorOutputId::KcLower, "Band Fill", Color::hex(0x607D8B)))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keltner_channel_creation() {
        let kc = KeltnerChannel::new_classic(20, 2.0);
        assert!(!kc.is_ready());
        assert_eq!(kc.period(), 20);
        assert_eq!(kc.multiplier(), 2.0);
    }

    #[test]
    fn test_keltner_channel_warmup() {
        let mut kc = KeltnerChannel::new_classic(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kc.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(kc.is_ready());
    }

    #[test]
    fn test_keltner_channel_values() {
        let mut kc = KeltnerChannel::new_classic(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, middle, lower) = kc.feed(&[price + 1.0, price - 1.0, price]);
            if kc.is_ready() {
                assert!(upper > middle, "Upper should be > middle");
                assert!(middle > lower, "Middle should be > lower");
            }
        }
    }

    #[test]
    fn test_keltner_channel_reset() {
        let mut kc = KeltnerChannel::new_classic(20, 2.0);
        for i in 0..25 {
            kc.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        kc.reset();
        assert!(!kc.is_ready());
    }

    #[test]
    fn test_keltner_channel_with_source() {
        let mut kc_close = KeltnerChannel::new_classic_with_source(3, 2.0, OhlcvField::Close);
        let mut kc_hl2 = KeltnerChannel::new_classic_with_source(3, 2.0, OhlcvField::HL2);

        let bars = vec![
            (100.0, 110.0, 90.0, 105.0, 1000.0),
            (105.0, 115.0, 95.0, 110.0, 1200.0),
            (110.0, 120.0, 100.0, 115.0, 800.0),
            (115.0, 125.0, 105.0, 120.0, 900.0),
        ];

        for (_o, h, l, c, _v) in &bars {
            kc_close.feed(&[*h, *l, *c]);
            kc_hl2.feed(&[*h, *l, *c]);
        }

        assert_ne!(kc_close.middle(), kc_hl2.middle(),
                   "Middle values should differ when using different sources");
        assert!(kc_close.is_ready());
        assert!(kc_hl2.is_ready());
    }

    #[test]
    fn factory_feeds_resolved_keltner() {
        use crate::engine::contract_engine::{IndicatorOrder, IndicatorOutputId};
        use crate::contract::MarketSample;
        let cfg = <<KeltnerChannel as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Kc(cfg).build_solo().unwrap();
        for i in 0..30 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: price + 1.0, low: price - 1.0, close: price, volume: 1000.0,
            });
        }
        // After warmup the channel should be ready and upper > lower
        let upper = f.read(IndicatorOutputId::KcUpper);
        let middle = f.read(IndicatorOutputId::KcMiddle);
        let lower = f.read(IndicatorOutputId::KcLower);
        assert!(upper > lower, "upper={} lower={}", upper, lower);
        assert!(middle > lower);
    }
}
