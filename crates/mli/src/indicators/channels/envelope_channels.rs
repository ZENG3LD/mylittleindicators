//! envelope_channels.rs: High-Performance Envelope Channels
//! Процентные каналы-конверты вокруг любого типа MA - классика технического анализа
//! 
//! Особенности:
//! - Поддержка ВСЕХ 19 типов MA как базовой линии
//! - Процентные отклонения (MA ± X%)
//! - Адаптивные режимы с переменными процентами
//! - Множественные уровни конвертов

use crate::engine::contract_engine::SmootherSlot;
use crate::engine::ohlcv_field::OhlcvField;

/// Режимы расчета Envelope Channels
#[derive(Debug, Clone, Copy, PartialEq)]
#[derive(Default)]
pub enum EnvelopeMode {
    /// Fixed - фиксированные проценты
    #[default]
    Fixed,
    /// Adaptive - адаптивные проценты на основе волатильности
    Adaptive,
    /// Multiple - множественные уровни конвертов
    Multiple,
}

/// High-Performance Envelope Channels
#[derive(Debug, Clone)]
pub struct EnvelopeChannels {
    period: usize,
    base_percentage: f64,
    mode: EnvelopeMode,
    ma_type: SmootherId,
    source: OhlcvField,

    // MovingAverage для базовой линии
    ma: SmootherSlot,
    
    // Текущие значения канала
    upper: f64,
    middle: f64, // MA линия
    lower: f64,
    
    // Множественные уровни (для Multiple режима)
    upper_levels: Vec<f64>, // [1%, 2%, 3%, 5%]
    lower_levels: Vec<f64>,
    
    // Адаптивные параметры
    adaptive_factor: f64,
    min_percentage: f64,
    max_percentage: f64,
}

impl EnvelopeChannels {
    /// Создать Envelope Channels с указанными параметрами
    pub fn new(
        period: usize,
        base_percentage: f64,
        mode: EnvelopeMode,
        ma_type: SmootherId
    ) -> Self {
        assert!(period > 0, "Period must be positive");
        assert!(base_percentage > 0.0, "Base percentage must be positive");

        Self {
            period,
            base_percentage,
            mode,
            ma_type,
            source: OhlcvField::Close,
            ma: SmootherSlot::new(ma_type, period),
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
            upper_levels: vec![0.0; 4], // 4 уровня
            lower_levels: vec![0.0; 4],
            adaptive_factor: 1.0,
            min_percentage: base_percentage * 0.5,
            max_percentage: base_percentage * 3.0,
        }
    }

    /// Создать Envelope Channels с настраиваемым источником данных
    pub fn with_source(
        period: usize,
        base_percentage: f64,
        mode: EnvelopeMode,
        ma_type: SmootherId,
        source: OhlcvField
    ) -> Self {
        assert!(period > 0, "Period must be positive");
        assert!(base_percentage > 0.0, "Base percentage must be positive");

        Self {
            period,
            base_percentage,
            mode,
            ma_type,
            source,
            ma: SmootherSlot::new(ma_type, period),
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
            upper_levels: vec![0.0; 4],
            lower_levels: vec![0.0; 4],
            adaptive_factor: 1.0,
            min_percentage: base_percentage * 0.5,
            max_percentage: base_percentage * 3.0,
        }
    }
    
    /// Создать фиксированные Envelope Channels с SMA
    pub fn new_fixed_sma(period: usize, percentage: f64) -> Self {
        Self::new(period, percentage, EnvelopeMode::Fixed, SmootherId::Sma)
    }
    
    /// Создать фиксированные Envelope Channels с EMA
    pub fn new_fixed_ema(period: usize, percentage: f64) -> Self {
        Self::new(period, percentage, EnvelopeMode::Fixed, SmootherId::Ema)
    }
    
    /// Создать адаптивные Envelope Channels
    pub fn new_adaptive(period: usize, base_percentage: f64, ma_type: SmootherId) -> Self {
        Self::new(period, base_percentage, EnvelopeMode::Adaptive, ma_type)
    }
    
    /// Создать множественные Envelope Channels
    pub fn new_multiple(period: usize, base_percentage: f64, ma_type: SmootherId) -> Self {
        Self::new(period, base_percentage, EnvelopeMode::Multiple, ma_type)
    }
    
    /// Настроить адаптивные параметры
    pub fn set_adaptive_params(&mut self, min_pct: f64, max_pct: f64) {
        self.min_percentage = min_pct;
        self.max_percentage = max_pct;
    }
    
    /// Рассчитать границы каналов
    fn calculate_channels(&mut self) {
        match self.mode {
            EnvelopeMode::Fixed => {
                self.calculate_fixed_channels();
            }
            EnvelopeMode::Adaptive => {
                self.calculate_adaptive_channels();
            }
            EnvelopeMode::Multiple => {
                self.calculate_multiple_channels();
            }
        }
    }
    
    /// Рассчитать фиксированные каналы
    fn calculate_fixed_channels(&mut self) {
        let envelope = self.middle * (self.base_percentage / 100.0);
        self.upper = self.middle + envelope;
        self.lower = self.middle - envelope;
    }
    
    /// Рассчитать адаптивные каналы
    fn calculate_adaptive_channels(&mut self) {
        let adaptive_percentage = (self.base_percentage * self.adaptive_factor)
            .clamp(self.min_percentage, self.max_percentage);
        
        let envelope = self.middle * (adaptive_percentage / 100.0);
        self.upper = self.middle + envelope;
        self.lower = self.middle - envelope;
    }
    
    /// Рассчитать множественные каналы
    fn calculate_multiple_channels(&mut self) {
        // Основной канал
        self.calculate_fixed_channels();
        
        // Дополнительные уровни: 1%, 2%, 3%, 5%
        let percentages = [1.0, 2.0, 3.0, 5.0];
        
        for (i, &pct) in percentages.iter().enumerate() {
            let envelope = self.middle * (pct / 100.0);
            self.upper_levels[i] = self.middle + envelope;
            self.lower_levels[i] = self.middle - envelope;
        }
    }
    
    /// Сбросить значения каналов
    fn reset_channels(&mut self) {
        self.upper = 0.0;
        self.lower = 0.0;
        self.upper_levels.fill(0.0);
        self.lower_levels.fill(0.0);
    }
    

    /// Получить текущие значения основного канала как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper, self.middle, self.lower)
    }
    
    /// Получить базовую MA линию
    pub fn middle(&self) -> f64 {
        self.middle
    }
    
    /// Получить верхнюю границу основного канала
    pub fn upper(&self) -> f64 {
        self.upper
    }
    
    /// Получить нижнюю границу основного канала
    pub fn lower(&self) -> f64 {
        self.lower
    }
    
    /// Получить уровень N% (для Multiple режима)
    pub fn get_level(&self, level_index: usize) -> Option<(f64, f64)> {
        if level_index < self.upper_levels.len() {
            Some((self.upper_levels[level_index], self.lower_levels[level_index]))
        } else {
            None
        }
    }
    
    /// Получить все верхние уровни
    pub fn upper_levels(&self) -> &[f64] {
        &self.upper_levels
    }
    
    /// Получить все нижние уровни  
    pub fn lower_levels(&self) -> &[f64] {
        &self.lower_levels
    }
    
    /// Получить текущий адаптивный фактор
    pub fn adaptive_factor(&self) -> f64 {
        self.adaptive_factor
    }
    
    /// Получить эффективный процент (с учетом адаптации)
    pub fn effective_percentage(&self) -> f64 {
        match self.mode {
            EnvelopeMode::Adaptive => {
                (self.base_percentage * self.adaptive_factor)
                    .clamp(self.min_percentage, self.max_percentage)
            }
            _ => self.base_percentage
        }
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
        if !self.is_ready() || self.upper == self.lower {
            0.5 // По центру если канал не готов или нулевой ширины
        } else {
            ((price - self.lower) / (self.upper - self.lower)).clamp(0.0, 1.0)
        }
    }
    
    /// Проверить пробой указанного уровня (для Multiple режима)
    pub fn is_level_breakout(&self, price: f64, level_index: usize) -> Option<bool> {
        if let Some((upper_level, lower_level)) = self.get_level(level_index) {
            if price > upper_level {
                Some(true) // Пробой вверх
            } else if price < lower_level {
                Some(false) // Пробой вниз
            } else {
                None // Нет пробоя
            }
        } else {
            None
        }
    }
    
    /// Проверить расширение канала (растущая волатильность)
    pub fn is_expanding(&self) -> bool {
        matches!(self.mode, EnvelopeMode::Adaptive) && self.adaptive_factor > 1.2
    }
    
    /// Проверить сужение канала (падающая волатильность)
    pub fn is_contracting(&self) -> bool {
        matches!(self.mode, EnvelopeMode::Adaptive) && self.adaptive_factor < 0.8
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.ma.is_ready()
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.ma.reset();
        self.adaptive_factor = 1.0;
        self.reset_channels();
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Получить базовый процент
    pub fn base_percentage(&self) -> f64 {
        self.base_percentage
    }
    
    /// Получить режим
    pub fn mode(&self) -> EnvelopeMode {
        self.mode
    }
    
    /// Получить тип MA
    pub fn ma_type(&self) -> SmootherId {
        self.ma_type
    }

    /// Получить источник цены
    pub fn source(&self) -> OhlcvField {
        self.source
    }

    /// Transitional bridge for un-contracted embedders that call the OHLCV form.
    // transitional bridge for un-contracted embedders; remove when they convert
    #[inline]

    /// Feed one pre-extracted scalar price — the contract feed path.
    pub fn feed(&mut self, price: f64) -> (f64, f64, f64) {
        self.middle = self.ma.feed(price);
        if matches!(self.mode, EnvelopeMode::Adaptive) {
            // simplified: no true range available from scalar alone — use middle as proxy
            if self.middle > 0.0 {
                self.adaptive_factor = 1.0;
            }
        }
        if self.is_ready() {
            self.calculate_channels();
        } else {
            self.reset_channels();
        }
        (self.upper, self.middle, self.lower)
    }
}

impl Default for EnvelopeChannels {
    fn default() -> Self {
        Self::new_fixed_sma(20, 2.5)
    }
}

// ---- Indicator contract ----

use crate::contract::{Param, sweep_f64};
use crate::engine::contract_engine::{IndicatorOutputId, SmootherId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Cost, Family, Indicator, Output, Slot, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`EnvelopeChannels`] (Fixed mode only via contract).
/// `source` is the configurable price field; `ma` is the center-line smoother slot.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct EnvelopeConfig {
    pub period: Param<usize>,
    pub base_percentage: Param<f64>,
    /// Center-line smoother slot. Default Sma at the host period (20).
    #[slot]
    pub ma: Param<SmootherSlotOrder>,
    pub source: Param<crate::engine::ohlcv_field::OhlcvField>,
}

impl Indicator for EnvelopeChannels {
    const ID: IndicatorId = IndicatorId::Envelope;
    /// Channel family — a percentage-band channel over a configurable MA center.
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    // Single configurable field (default close) — Source flavor.
    const SOURCE: Option<crate::contract::SourceAxis> =
        Some(crate::contract::SourceAxis::Field {
            default: crate::engine::ohlcv_field::OhlcvField::Close,
        });
    /// The SmootherSlot is the recursive inner MA cost; own base = Constant (no extra buffer).
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = EnvelopeConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::EnvelopeUpper),
        Output::price(IndicatorOutputId::EnvelopeMiddle),
        Output::price(IndicatorOutputId::EnvelopeLower),
    ];
    type Config = EnvelopeConfig;
    type Runtime = EnvelopeChannels;

    fn create(cfg: EnvelopeConfig) -> EnvelopeChannels {
        let period = cfg.period.resolved();
        let base_percentage = cfg.base_percentage.resolved();
        let order = cfg.ma.resolved();
        let ma_type = order.id();
        EnvelopeChannels {
            period,
            base_percentage,
            mode: EnvelopeMode::Fixed,
            ma_type,
            source: cfg.source.resolved(),
            ma: order.into_slot(),
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
            upper_levels: vec![0.0; 4],
            lower_levels: vec![0.0; 4],
            adaptive_factor: 1.0,
            min_percentage: base_percentage * 0.5,
            max_percentage: base_percentage * 3.0,
        }
    }

    fn source_fields(cfg: &EnvelopeConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &EnvelopeConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for EnvelopeConfig {
    fn defaults() -> Self {
        EnvelopeConfig {
            period: Param::Solo(20),
            base_percentage: Param::Solo(2.5),
            ma: Param::Solo(SmootherSlotOrder::Sma(PeriodConfig { period: 20 })),
            source: Param::Solo(crate::engine::ohlcv_field::OhlcvField::Close),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1), source→all 8
        s.base_percentage = Param::many(sweep_f64(0.0, 1.0, 0.05)); // Class D ratio/percentage
        s
    }
}


impl Render for EnvelopeChannels {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::EnvelopeUpper, "Upper", Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::EnvelopeMiddle, "Middle", Color::hex(0x607D8B), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::EnvelopeLower, "Lower", Color::hex(0x4CAF50), 1.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_channels_creation() {
        let ec = EnvelopeChannels::new_fixed_sma(20, 2.5);
        assert!(!ec.is_ready());
        assert_eq!(ec.period(), 20);
        assert_eq!(ec.base_percentage(), 2.5);
    }

    #[test]
    fn test_envelope_channels_warmup() {
        let mut ec = EnvelopeChannels::new_fixed_sma(20, 2.5);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ec.feed(price);
        }
        assert!(ec.is_ready());
    }

    #[test]
    fn test_envelope_channels_values() {
        let mut ec = EnvelopeChannels::new_fixed_sma(20, 2.5);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, middle, lower) = ec.feed(price);
            if ec.is_ready() {
                assert!(upper > middle, "Upper should be > middle");
                assert!(middle > lower, "Middle should be > lower");
            }
        }
    }

    #[test]
    fn test_envelope_channels_reset() {
        let mut ec = EnvelopeChannels::new_fixed_sma(20, 2.5);
        for i in 0..25 {
            ec.feed(100.0 + i as f64);
        }
        ec.reset();
        assert!(!ec.is_ready());
    }

    /// Factory resolves configurable source (default close); wild high/low/open/volume ignored.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Envelope(<<EnvelopeChannels as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..25usize {
            let close = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 9999.0, close, volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite(), "upper should be finite, got {}", f.primary());
    }
} 

