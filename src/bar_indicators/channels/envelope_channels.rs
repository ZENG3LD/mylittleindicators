//! envelope_channels.rs: High-Performance Envelope Channels
//! Процентные каналы-конверты вокруг любого типа MA - классика технического анализа
//! 
//! Особенности:
//! - Поддержка ВСЕХ 19 типов MA как базовой линии
//! - Процентные отклонения (MA ± X%)
//! - Адаптивные режимы с переменными процентами
//! - Множественные уровни конвертов

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::average::moving_average::MovingAverageProvider;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;
use serde::{Serialize, Deserialize};

/// Режимы расчета Envelope Channels
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    ma_type: MovingAverageType,
    #[allow(dead_code)]
    source: OhlcvField,

    // MovingAverage для базовой линии
    ma: MovingAverageProvider,
    
    // Circular buffer для адаптивного расчета волатильности - O(1) operations
    volatility_buffer: Vec<f64>,
    volatility_index: usize,
    volatility_filled: bool,
    current_volatility: f64,
    
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
        ma_type: MovingAverageType
    ) -> Self {
        assert!(period > 0, "Period must be positive");
        assert!(base_percentage > 0.0, "Base percentage must be positive");

        Self {
            period,
            base_percentage,
            mode,
            ma_type,
            source: OhlcvField::Close,
            ma: MovingAverageProvider::new(ma_type, period),
            volatility_buffer: Vec::with_capacity(period),
            volatility_index: 0,
            volatility_filled: false,
            current_volatility: 0.0,
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
        ma_type: MovingAverageType,
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
            ma: MovingAverageProvider::new(ma_type, period),
            volatility_buffer: Vec::with_capacity(period),
            volatility_index: 0,
            volatility_filled: false,
            current_volatility: 0.0,
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
        Self::new(period, percentage, EnvelopeMode::Fixed, MovingAverageType::SMA)
    }
    
    /// Создать фиксированные Envelope Channels с EMA
    pub fn new_fixed_ema(period: usize, percentage: f64) -> Self {
        Self::new(period, percentage, EnvelopeMode::Fixed, MovingAverageType::EMA)
    }
    
    /// Создать адаптивные Envelope Channels
    pub fn new_adaptive(period: usize, base_percentage: f64, ma_type: MovingAverageType) -> Self {
        Self::new(period, base_percentage, EnvelopeMode::Adaptive, ma_type)
    }
    
    /// Создать множественные Envelope Channels
    pub fn new_multiple(period: usize, base_percentage: f64, ma_type: MovingAverageType) -> Self {
        Self::new(period, base_percentage, EnvelopeMode::Multiple, ma_type)
    }
    
    /// Настроить адаптивные параметры
    pub fn set_adaptive_params(&mut self, min_pct: f64, max_pct: f64) {
        self.min_percentage = min_pct;
        self.max_percentage = max_pct;
    }
    
    /// Обновить каналы новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> (f64, f64, f64) {
        // Обновляем базовую MA линию
        self.middle = self.ma.update_bar(open, high, low, close, volume);
        
        // Обновляем волатильность для адаптивного режима
        if matches!(self.mode, EnvelopeMode::Adaptive) {
            self.update_volatility(high, low, close);
        }
        
        // Рассчитываем каналы если готово
        if self.is_ready() {
            self.calculate_channels();
        } else {
            self.reset_channels();
        }
        
        (self.upper, self.middle, self.lower)
    }
    
    /// Обновить волатильность для адаптивного режима
    fn update_volatility(&mut self, high: f64, low: f64, close: f64) {
        // Используем True Range как меру волатильности
        let prev_close = if !self.volatility_buffer.is_empty() {
            *self.volatility_buffer.last().unwrap()
        } else {
            close
        };
        
        let true_range = (high - low)
            .max((high - prev_close).abs())
            .max((low - prev_close).abs());
        
        // Добавляем в circular buffer волатильности - O(1) операция
        if self.volatility_filled {
            // Перезаписываем старые значения циклически
            self.volatility_buffer[self.volatility_index] = true_range;
        } else {
            // Заполняем буфер в первый раз
            self.volatility_buffer.push(true_range);
        }
        
        // Обновляем индекс циклически
        self.volatility_index = (self.volatility_index + 1) % self.period;
        
        // Проверяем заполненность буфера
        if self.volatility_buffer.len() == self.period && !self.volatility_filled {
            self.volatility_filled = true;
        }
        
        // Рассчитываем среднюю волатильность
        if !self.volatility_buffer.is_empty() {
            self.current_volatility = self.volatility_buffer.iter().sum::<f64>() / self.volatility_buffer.len() as f64;
            
            // Нормализуем волатильность относительно цены
            if self.middle > 0.0 {
                let volatility_pct = (self.current_volatility / self.middle) * 100.0;
                
                // Адаптивный фактор: высокая волатильность = шире каналы
                self.adaptive_factor = (1.0 + volatility_pct / 10.0).clamp(0.5, 3.0);
            }
        }
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
    
    /// Получить текущие значения основного канала как типизированный IndicatorValue
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.upper,
            middle: self.middle,
            lower: self.lower,
        }
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
    
    /// Получить текущую волатильность
    pub fn current_volatility(&self) -> f64 {
        self.current_volatility
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
        self.volatility_buffer.clear();
        self.current_volatility = 0.0;
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
    pub fn ma_type(&self) -> MovingAverageType {
        self.ma_type
    }
}

impl Default for EnvelopeChannels {
    fn default() -> Self {
        Self::new_fixed_sma(20, 2.5) // 20-период SMA с 2.5% конвертами
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
            ec.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ec.is_ready());
    }

    #[test]
    fn test_envelope_channels_values() {
        let mut ec = EnvelopeChannels::new_fixed_sma(20, 2.5);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, middle, lower) = ec.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
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
            ec.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        ec.reset();
        assert!(!ec.is_ready());
    }
} 






















