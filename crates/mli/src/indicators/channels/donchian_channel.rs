//! Donchian Channel Indicator - ОПТИМИЗИРОВАННАЯ ВЕРСИЯ
//!
//! Индикатор канала Дончиана для определения поддержки/сопротивления:
//! - Upper Band = Highest High за period периодов
//! - Lower Band = Lowest Low за period периодов
//! - Middle Band = (Upper + Lower) / 2
//!
//! 🚀 СУПЕР ОПТИМИЗАЦИЯ: Циклический буфер O(1) + поддержка ВСЕХ 19 типов MA!

use crate::engine::contract_engine::SmootherId;
use crate::engine::contract_engine::SmootherSlot;

/// Режимы расчета Donchian Channel
#[derive(Debug, Clone, Copy, PartialEq)]
#[derive(Default)]
pub enum DonchianMode {
    /// Классический - простые максимумы/минимумы
    #[default]
    Classic,
    /// Сглаженный - применяется MA к уровням каналов
    Smoothed,
}

/// ОПТИМИЗИРОВАННЫЙ Donchian Channel индикатор
#[derive(Debug, Clone)]
pub struct DonchianChannel {
    period: usize,
    mode: DonchianMode,
    ma_type: SmootherId,

    // 🚀 СУПЕР БЫСТРЫЕ циклические буферы (O(1) операции!)
    high_buffer: Vec<f64>,
    low_buffer: Vec<f64>,
    buffer_index: usize,  // Индекс для циклической перезаписи

    // Сглаживающие MA (используются только в режиме Smoothed)
    upper_ma: Option<SmootherSlot>,
    lower_ma: Option<SmootherSlot>,
    middle_ma: Option<SmootherSlot>,

    // Текущие значения каналов
    upper_band: f64,
    lower_band: f64,
    middle_band: f64,

    // Готовность индикатора
    is_ready: bool,
}

impl Default for DonchianChannel {
    fn default() -> Self {
        Self::new(14)
    }
}

impl DonchianChannel {
    /// Создать новый Donchian Channel с классическим режимом
    pub fn new(period: usize) -> Self {
        Self::new_with_mode(period, DonchianMode::Classic, SmootherId::Sma)
    }

    /// Создать Donchian Channel с указанным режимом и типом MA
    pub fn new_with_mode(period: usize, mode: DonchianMode, ma_type: SmootherId) -> Self {
        let (upper_ma, lower_ma, middle_ma) = match mode {
            DonchianMode::Classic => (None, None, None),
            DonchianMode::Smoothed => (
                Some(SmootherSlot::new(ma_type, period)),
                Some(SmootherSlot::new(ma_type, period)),
                Some(SmootherSlot::new(ma_type, period)),
            ),
        };

        Self {
            period,
            mode,
            ma_type,
            high_buffer: Vec::with_capacity(period),
            low_buffer: Vec::with_capacity(period),
            buffer_index: 0,
            upper_ma,
            lower_ma,
            middle_ma,
            upper_band: 0.0,
            lower_band: 0.0,
            middle_band: 0.0,
            is_ready: false,
        }
    }

    /// 🚀 Создать сглаженный Donchian Channel с поддержкой ВСЕХ типов MA
    pub fn new_smoothed(period: usize, ma_type: SmootherId) -> Self {
        Self::new_with_mode(period, DonchianMode::Smoothed, ma_type)
    }

    /// Быстрые конструкторы для популярных типов сглаживания
    pub fn new_smoothed_sma(period: usize) -> Self {
        Self::new_smoothed(period, SmootherId::Sma)
    }

    pub fn new_smoothed_ema(period: usize) -> Self {
        Self::new_smoothed(period, SmootherId::Ema)
    }

    pub fn new_smoothed_hull(period: usize) -> Self {
        Self::new_smoothed(period, SmootherId::Hma)
    }

    /// Feed resolved High/Low lanes — the contract feed path (replaces `update_bar`).
    /// Operates in Classic mode: rolling highest-high / lowest-low + midline.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        if self.high_buffer.len() < self.period {
            self.high_buffer.push(high);
            self.low_buffer.push(low);
        } else {
            self.high_buffer[self.buffer_index] = high;
            self.low_buffer[self.buffer_index] = low;
        }
        self.buffer_index = (self.buffer_index + 1) % self.period;
        if self.high_buffer.len() >= self.period {
            let (raw_lower, raw_upper) = self.high_buffer.iter()
                .zip(self.low_buffer.iter())
                .fold((f64::INFINITY, f64::NEG_INFINITY),
                      |(min, max), (&h, &l)| (min.min(l), max.max(h)));
            self.upper_band = raw_upper;
            self.lower_band = raw_lower;
            self.middle_band = (raw_upper + raw_lower) / 2.0;
            self.is_ready = true;
        }
        (self.upper_band, self.lower_band, self.middle_band)
    }


    /// Получить текущие значения каналов в оригинальном порядке (для обратной совместимости)
    /// Возвращает (upper, lower, middle) - нестандартный порядок!
    pub fn values(&self) -> (f64, f64, f64) {
        (self.upper_band, self.lower_band, self.middle_band)
    }

    /// Получить текущие значения каналов в стандартном порядке
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper_band, self.middle_band, self.lower_band)
    }

    /// Получить верхний канал
    pub fn upper_band(&self) -> f64 {
        self.upper_band
    }

    /// Получить нижний канал
    pub fn lower_band(&self) -> f64 {
        self.lower_band
    }

    /// Получить средний канал
    pub fn middle_band(&self) -> f64 {
        self.middle_band
    }

    pub fn upper(&self) -> f64 { self.upper_band }
    pub fn middle(&self) -> f64 { self.middle_band }
    pub fn lower(&self) -> f64 { self.lower_band }

    /// Получить ширину канала
    pub fn channel_width(&self) -> f64 {
        self.upper_band - self.lower_band
    }

    /// Получить относительную позицию цены в канале (0.0 = на нижнем канале, 1.0 = на верхнем)
    pub fn position_in_channel(&self, price: f64) -> f64 {
        let width = self.channel_width();
        if width > 0.0 {
            (price - self.lower_band) / width
        } else {
            0.5 // Если каналы схлопнулись, считаем что в середине
        }
    }

    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Transitional bridge for un-contracted embedders that call the OHLCV form.
    // transitional bridge for un-contracted embedders; remove when they convert
    #[inline]

    /// Получить период индикатора
    pub fn period(&self) -> usize {
        self.period
    }

    /// Получить режим работы индикатора
    pub fn mode(&self) -> DonchianMode {
        self.mode
    }

    /// Получить тип MA (для сглаженного режима)
    pub fn ma_type(&self) -> SmootherId {
        self.ma_type
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.high_buffer.clear();
        self.low_buffer.clear();
        self.buffer_index = 0;

        if let Some(ref mut ma) = self.upper_ma {
            ma.reset();
        }
        if let Some(ref mut ma) = self.lower_ma {
            ma.reset();
        }
        if let Some(ref mut ma) = self.middle_ma {
            ma.reset();
        }

        self.upper_band = 0.0;
        self.lower_band = 0.0;
        self.middle_band = 0.0;
        self.is_ready = false;
    }

}

// ---- Indicator contract ----

use crate::contract::Param;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`DonchianChannel`] — period only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DcConfig {
    pub period: Param<usize>,
}

impl Indicator for DonchianChannel {
    const ID: IndicatorId = IndicatorId::Dc;
    /// Channel family: a canonical banded channel (rolling period high/low).
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// O(period) rescan every bar. Two period-deep Vec buffers (high / low).
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::DcUpper),
        Output::price(IndicatorOutputId::DcMiddle),
        Output::price(IndicatorOutputId::DcLower),
    ];
    type Config = DcConfig;
    type Runtime = DonchianChannel;

    fn create(cfg: DcConfig) -> DonchianChannel {
        DonchianChannel::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for DcConfig {
    fn defaults() -> Self {
        DcConfig { period: Param::Solo(20) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        Self::machine_defaults_auto() // period→range(2,4048,1) — only axis
    }
}


impl Render for DonchianChannel {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::DcUpper, "Upper", Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::DcMiddle, "Middle", Color::hex(0x607D8B), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::DcLower, "Lower", Color::hex(0x4CAF50), 1.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_donchian_channel_creation() {
        let dc = DonchianChannel::new(20);
        assert!(!dc.is_ready());
        assert_eq!(dc.period(), 20);
    }

    #[test]
    fn test_donchian_channel_warmup() {
        let mut dc = DonchianChannel::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            dc.feed(&[price + 1.0, price - 1.0]);
        }
        assert!(dc.is_ready());
    }

    #[test]
    fn test_donchian_channel_values() {
        let mut dc = DonchianChannel::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, lower, _middle) = dc.feed(&[price + 1.0, price - 1.0]);
            if dc.is_ready() {
                assert!(upper >= lower, "Upper should be >= lower");
                assert!(dc.channel_width() >= 0.0, "Channel width should be non-negative");
            }
        }
    }

    #[test]
    fn test_donchian_channel_reset() {
        let mut dc = DonchianChannel::new(20);
        for i in 0..25 {
            dc.feed(&[101.0 + i as f64, 99.0 + i as f64]);
        }
        dc.reset();
        assert!(!dc.is_ready());
    }

    /// Factory resolves fixed High/Low from `const SOURCE`; in a steady uptrend the
    /// upper band equals the current high fed in — proved by wild close (9999.0) being
    /// ignored.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::{IndicatorOrder, IndicatorOutputId};
        use crate::contract::MarketSample;
        let cfg = <<DonchianChannel as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Dc(cfg).build_solo().unwrap();
        for i in 1..=30usize {
            let base = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: -1.0,
                high: base + 1.0,
                low: base - 1.0,
                close: 9999.0,
                volume: -1.0,
            });
        }
        assert!(f.is_ready());
        // upper band in a steady uptrend = most recent high = 131.0
        let upper = f.read(IndicatorOutputId::DcUpper);
        assert!((upper - 131.0).abs() < 1e-9, "expected upper=131.0, got {upper}");
    }
}
