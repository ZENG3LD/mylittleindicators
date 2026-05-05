//! Keltner Stop - индикатор динамических уровней на основе каналов Кельтнера
//! 
//! Вычисляет уровни на основе каналов Кельтнера:
//! - Средняя линия: EMA типичной цены (HLC/3)
//! - Верхняя полоса: EMA + (ATR × multiplier) 
//! - Нижняя полоса: EMA - (ATR × multiplier)
//! 
//! Может использовать разные типы средних и настраиваемые множители ATR.
//! Полосы каналов служат как динамические уровни поддержки/сопротивления.
//! 
//! Индикатор НЕ содержит логику стопов - только возвращает полосы каналов.
//! Логика остановки позиций реализуется в стратегиях на основе этих уровней.

use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Keltner Stop индикатор - уровни на основе каналов Кельтнера
#[derive(Debug, Clone)]
pub struct KeltnerStop {
    period: usize,
    multiplier: f64,
    ma_type: MovingAverageType,
    atr_type: MovingAverageType,
    
    // Индикаторы
    price_ma: MovingAverageProvider,    // MA типичной цены
    atr: Atr,                   // ATR для волатильности
    
    // Текущие значения
    middle_line: f64,           // Средняя линия (EMA цены)
    upper_band: f64,            // Верхняя полоса
    lower_band: f64,            // Нижняя полоса
    
    // Состояние  
    bars_count: usize,
    is_ready: bool,
}

impl KeltnerStop {
    /// Создать Keltner Stop с параметрами по умолчанию
    /// (period=20, multiplier=2.0, EMA, Wilder ATR)
    pub fn new() -> Self {
        Self::with_params(20, 2.0, MovingAverageType::EMA, MovingAverageType::RMA)
    }
    
    /// Создать Keltner Stop с настраиваемыми параметрами
    /// * `period` - период для MA и ATR  
    /// * `multiplier` - множитель ATR для полос
    /// * `ma_type` - тип скользящей средней для центральной линии
    /// * `atr_type` - тип сглаживания для ATR
    pub fn with_params(
        period: usize,
        multiplier: f64,
        ma_type: MovingAverageType,
        atr_type: MovingAverageType,
    ) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        assert!(multiplier > 0.0, "Multiplier must be greater than 0");
        
        Self {
            period,
            multiplier,
            ma_type,
            atr_type,
            price_ma: MovingAverageProvider::new(ma_type, period),
            atr: Atr::new(period, atr_type),
            middle_line: 0.0,
            upper_band: 0.0,
            lower_band: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Создать для оптимизации с широким диапазоном параметров
    pub fn for_optimization(
        period: usize,
        multiplier: f64,
        ma_type: MovingAverageType,
        atr_type: MovingAverageType,
    ) -> Self {
        Self::with_params(period, multiplier, ma_type, atr_type)
    }
    
    /// Обновить индикатор новым баром  
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> (f64, f64, f64) {
        self.bars_count += 1;
        
        // Вычисляем типичную цену (HLC/3)
        let typical_price = (high + low + close) / 3.0;
        
        // Обновляем скользящую среднюю типичной цены
        self.middle_line = self.price_ma.update_bar(typical_price, typical_price, typical_price, typical_price, volume);
        
        // Обновляем ATR
        let atr_value = self.atr.update_bar(open, high, low, close, volume);
        
        // Вычисляем полосы каналов
        let atr_offset = atr_value * self.multiplier;
        self.upper_band = self.middle_line + atr_offset;
        self.lower_band = self.middle_line - atr_offset;
        
        // Готов когда есть достаточно данных
        self.is_ready = self.bars_count >= self.period && self.atr.is_ready();
        
        (self.lower_band, self.upper_band, self.middle_line)
    }
    
    /// Получить нижнюю полосу (уровень для лонг позиций)
    pub fn lower_band(&self) -> f64 {
        self.lower_band
    }
    
    /// Получить верхнюю полосу (уровень для шорт позиций)
    pub fn upper_band(&self) -> f64 {
        self.upper_band
    }
    
    /// Получить среднюю линию
    pub fn middle_line(&self) -> f64 {
        self.middle_line
    }
    
    /// Получить основной уровень (нижняя полоса по умолчанию)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.lower_band)
    }
    
    /// Получить уровни как кортеж (lower, upper, middle)
    pub fn levels(&self) -> (f64, f64, f64) {
        (self.lower_band, self.upper_band, self.middle_line)
    }
    
    /// Получить ширину канала
    pub fn channel_width(&self) -> f64 {
        self.upper_band - self.lower_band
    }
    
    /// Получить позицию цены в канале (0.0 = нижняя полоса, 1.0 = верхняя)
    pub fn price_position(&self, price: f64) -> f64 {
        if self.channel_width() == 0.0 {
            return 0.5;
        }
        (price - self.lower_band) / self.channel_width()
    }
    
    /// Проверить пробой верхней полосы
    pub fn is_above_upper(&self, price: f64) -> bool {
        price > self.upper_band
    }
    
    /// Проверить пробой нижней полосы
    pub fn is_below_lower(&self, price: f64) -> bool {
        price < self.lower_band
    }
    
    /// Проверить нахождение внутри канала
    pub fn is_inside_channel(&self, price: f64) -> bool {
        price >= self.lower_band && price <= self.upper_band
    }
    
    /// Получить текущее значение ATR
    pub fn atr_value(&self) -> f64 {
        self.atr.value().main()
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.price_ma.reset();
        self.atr.reset();
        self.middle_line = 0.0;
        self.upper_band = 0.0;
        self.lower_band = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
    
    /// Получить параметры индикатора
    pub fn params(&self) -> (usize, f64, MovingAverageType, MovingAverageType) {
        (self.period, self.multiplier, self.ma_type, self.atr_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keltner_stop_creation() {
        let ind = KeltnerStop::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_keltner_stop_with_params() {
        let ind = KeltnerStop::with_params(20, 2.5, MovingAverageType::EMA, MovingAverageType::RMA);
        assert!(!ind.is_ready());
        let (period, mult, _, _) = ind.params();
        assert_eq!(period, 20);
        assert_eq!(mult, 2.5);
    }

    #[test]
    fn test_keltner_stop_warmup() {
        let mut ind = KeltnerStop::with_params(10, 2.0, MovingAverageType::EMA, MovingAverageType::RMA);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_keltner_stop_values_finite() {
        let mut ind = KeltnerStop::with_params(10, 2.0, MovingAverageType::EMA, MovingAverageType::RMA);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (lower, upper, mid) = ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(lower.is_finite());
            assert!(upper.is_finite());
            assert!(mid.is_finite());
        }
    }

    #[test]
    fn test_keltner_stop_reset() {
        let mut ind = KeltnerStop::with_params(10, 2.0, MovingAverageType::EMA, MovingAverageType::RMA);
        for i in 0..20 {
            ind.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
} 