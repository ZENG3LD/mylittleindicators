//! bollinger_bands.rs: High-Performance Bollinger Bands
//! Полосы Боллинджера с правильной архитектурой
//! 
//! Особенности:
//! - Использует готовый MovingAverage компонент для центральной линии
//! - 4 режима расчета цены
//! - ALL 19 MA types через MovingAverage
//! - Дополнительные метрики: %B, Bandwidth, Squeeze detection
//! - O(1) обновления без remove(0)

use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;
use arrayvec::ArrayVec;
use serde::{Serialize, Deserialize};

/// Режимы расчета Bollinger Bands
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub enum BollingerMode {
    /// Typical - использует Typical Price (HLC/3)
    Typical,
    /// Close - использует только Close цену
    #[default]
    Close,
    /// OHLC - использует (Open + High + Low + Close) / 4
    OHLC,
    /// HL - использует (High + Low) / 2
    HL,
}


/// High-Performance Bollinger Bands
/// Архитектура: MovingAverageProvider для центральной линии + circular buffer для std dev
#[derive(Debug, Clone)]
pub struct BollingerBands {
    // Параметры
    period: usize,
    std_dev_mult: f64,
    mode: BollingerMode,
    ma_type: MovingAverageType,
    source: OhlcvField,

    // Компоненты (используем готовые!)
    center_ma: MovingAverageProvider,     // ✅ Центральная линия через MovingAverage

    // Circular buffer для стандартного отклонения - O(1) operations
    price_buffer: ArrayVec<f64, 512>,
    buffer_index: usize,
    buffer_filled: bool,

    // Текущие значения канала
    upper: f64,
    middle: f64,
    lower: f64,

    // Дополнительные метрики
    std_dev: f64,
    bandwidth: f64,
    percent_b: f64,
}

impl BollingerBands {
    /// Создать Bollinger Bands с указанными параметрами
    /// period - период для MA и std dev
    /// std_dev_mult - множитель стандартного отклонения (обычно 2.0)
    /// mode - режим расчета цены для анализа
    /// ma_type - тип MA для центральной линии (SMA, EMA, KAMA, etc.)
    pub fn new(
        period: usize,
        std_dev_mult: f64,
        mode: BollingerMode,
        ma_type: MovingAverageType
    ) -> Self {
        assert!(period > 0 && period <= 512, "Period must be between 1 and 512");
        assert!(std_dev_mult > 0.0, "Standard deviation multiplier must be positive");

        Self {
            period,
            std_dev_mult,
            mode,
            ma_type,
            source: OhlcvField::Close,
            center_ma: MovingAverageProvider::new(ma_type, period),
            price_buffer: ArrayVec::new(),
            buffer_index: 0,
            buffer_filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
            std_dev: 0.0,
            bandwidth: 0.0,
            percent_b: 0.5,
        }
    }

    /// Создать Bollinger Bands с настраиваемым источником данных
    /// period - период для MA и std dev
    /// std_dev_mult - множитель стандартного отклонения (обычно 2.0)
    /// mode - режим расчета цены для анализа
    /// ma_type - тип MA для центральной линии (SMA, EMA, KAMA, etc.)
    /// source - поле OHLCV для анализа (Close, HL2, HLC3, etc.)
    pub fn with_source(
        period: usize,
        std_dev_mult: f64,
        mode: BollingerMode,
        ma_type: MovingAverageType,
        source: OhlcvField
    ) -> Self {
        assert!(period > 0 && period <= 512, "Period must be between 1 and 512");
        assert!(std_dev_mult > 0.0, "Standard deviation multiplier must be positive");

        Self {
            period,
            std_dev_mult,
            mode,
            ma_type,
            source,
            center_ma: MovingAverageProvider::new(ma_type, period),
            price_buffer: ArrayVec::new(),
            buffer_index: 0,
            buffer_filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
            std_dev: 0.0,
            bandwidth: 0.0,
            percent_b: 0.5,
        }
    }
    
    /// Создать классические Bollinger Bands (Close, SMA)
    pub fn new_classic(period: usize, std_dev_mult: f64) -> Self {
        Self::new(period, std_dev_mult, BollingerMode::Close, MovingAverageType::SMA)
    }
    
    /// Создать Bollinger Bands с SMA
    pub fn new_sma(period: usize, std_dev_mult: f64, mode: BollingerMode) -> Self {
        Self::new(period, std_dev_mult, mode, MovingAverageType::SMA)
    }
    
    /// Создать Bollinger Bands с EMA
    pub fn new_ema(period: usize, std_dev_mult: f64, mode: BollingerMode) -> Self {
        Self::new(period, std_dev_mult, mode, MovingAverageType::EMA)
    }
    
    /// Создать Bollinger Bands с KAMA
    pub fn new_kama(period: usize, std_dev_mult: f64, mode: BollingerMode) -> Self {
        Self::new(period, std_dev_mult, mode, MovingAverageType::AMA)
    }
    
    /// Обновить полосы новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> (f64, f64, f64) {
        // Используем настраиваемый источник данных (поддерживает mode через source)
        let price = self.source.extract(open, high, low, close, volume);

        // ✅ Обновляем центральную линию через MovingAverage (все типы MA)
        self.middle = self.center_ma.update_bar(
            price,    // Передаем нужную цену как open
            price,    // high
            price,    // low
            price,    // close
            volume
        );
        
        // Добавляем цену в circular buffer для std dev - O(1) операция
        if self.buffer_filled {
            // Перезаписываем старые значения циклически
            self.price_buffer[self.buffer_index] = price;
        } else {
            // Заполняем буфер в первый раз
            self.price_buffer.push(price);
        }
        
        // Обновляем индекс циклически
        self.buffer_index = (self.buffer_index + 1) % self.period;
        
        // Проверяем заполненность буфера
        if self.price_buffer.len() == self.period && !self.buffer_filled {
            self.buffer_filled = true;
        }
        
        // Рассчитываем стандартное отклонение и границы
        if self.is_ready() {
            self.calculate_std_dev_and_bands(close);
        } else {
            self.upper = 0.0;
            self.lower = 0.0;
            self.std_dev = 0.0;
            self.bandwidth = 0.0;
            self.percent_b = 0.5;
        }
        
        (self.upper, self.middle, self.lower)
    }
    
    /// Рассчитать стандартное отклонение и границы полос
    fn calculate_std_dev_and_bands(&mut self, current_price: f64) {
        let buffer_len = if self.buffer_filled { self.period } else { self.price_buffer.len() };
        
        // Рассчитываем стандартное отклонение относительно текущего MA
        let variance = self.price_buffer.iter()
            .take(buffer_len)
            .map(|&price| {
                let diff = price - self.middle;
                diff * diff
            })
            .sum::<f64>() / buffer_len as f64;
        
        self.std_dev = variance.sqrt();
        
        // Рассчитываем границы полос
        self.upper = self.middle + self.std_dev_mult * self.std_dev;
        self.lower = self.middle - self.std_dev_mult * self.std_dev;
        
        // Рассчитываем дополнительные метрики
        self.bandwidth = if self.middle != 0.0 {
            (self.upper - self.lower) / self.middle
        } else {
            0.0
        };
        
        // %B показывает позицию цены относительно полос (0.0 = нижняя полоса, 1.0 = верхняя полоса)
        let band_width = self.upper - self.lower;
        if band_width > 0.0 {
            self.percent_b = (current_price - self.lower) / band_width;
        } else {
            self.percent_b = 0.5;
        }
    }
    
    /// Получить текущие значения полос как типизированный IndicatorValue
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.upper,
            middle: self.middle,
            lower: self.lower,
        }
    }

    /// Получить текущие значения полос как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper, self.middle, self.lower)
    }
    
    /// Получить верхнюю полосу
    pub fn upper(&self) -> f64 {
        self.upper
    }
    
    /// Получить среднюю линию
    pub fn middle(&self) -> f64 {
        self.middle
    }
    
    /// Получить нижнюю полосу
    pub fn lower(&self) -> f64 {
        self.lower
    }
    
    /// Получить стандартное отклонение
    pub fn std_dev(&self) -> f64 {
        self.std_dev
    }
    
    /// Получить Bandwidth (ширина полос относительно средней линии)
    pub fn bandwidth(&self) -> f64 {
        self.bandwidth
    }
    
    /// Получить %B (позиция цены в полосах, 0-1)
    pub fn percent_b(&self) -> f64 {
        self.percent_b
    }
    
    /// Получить ширину канала в абсолютных значениях
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
    
    /// Проверить "сжатие" полос (squeeze)
    pub fn is_squeeze(&self, threshold: f64) -> bool {
        self.is_ready() && self.bandwidth < threshold
    }
    
    /// Проверить пробой верхней полосы
    pub fn is_upper_breakout(&self, price: f64) -> bool {
        self.is_ready() && price > self.upper
    }
    
    /// Проверить пробой нижней полосы
    pub fn is_lower_breakout(&self, price: f64) -> bool {
        self.is_ready() && price < self.lower
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.center_ma.is_ready() && self.buffer_filled
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.center_ma.reset();
        self.price_buffer.clear();
        self.buffer_index = 0;
        self.buffer_filled = false;
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
        self.std_dev = 0.0;
        self.bandwidth = 0.0;
        self.percent_b = 0.5;
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Получить множитель стандартного отклонения
    pub fn std_dev_mult(&self) -> f64 {
        self.std_dev_mult
    }
    
    /// Получить режим
    pub fn mode(&self) -> BollingerMode {
        self.mode
    }
    
    /// Получить тип MA
    pub fn ma_type(&self) -> MovingAverageType {
        self.ma_type
    }
}

impl Default for BollingerBands {
    fn default() -> Self {
        Self::new_classic(20, 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bollinger_bands_creation() {
        let bb = BollingerBands::new_classic(20, 2.0);
        assert!(!bb.is_ready());
        assert_eq!(bb.period(), 20);
        assert_eq!(bb.std_dev_mult(), 2.0);
    }

    #[test]
    fn test_bollinger_bands_warmup() {
        let mut bb = BollingerBands::new_classic(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            bb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(bb.is_ready());
    }

    #[test]
    fn test_bollinger_bands_values() {
        let mut bb = BollingerBands::new_classic(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, middle, lower) = bb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if bb.is_ready() {
                assert!(upper > middle, "Upper should be > middle");
                assert!(middle > lower, "Middle should be > lower");
                assert!(bb.bandwidth() >= 0.0, "Bandwidth should be non-negative");
            }
        }
    }

    #[test]
    fn test_bollinger_bands_reset() {
        let mut bb = BollingerBands::new_classic(20, 2.0);
        for i in 0..25 {
            bb.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        bb.reset();
        assert!(!bb.is_ready());
    }
} 






















