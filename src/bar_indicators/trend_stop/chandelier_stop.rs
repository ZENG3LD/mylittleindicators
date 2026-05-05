//! Chandelier Stop - индикатор динамических уровней на основе Chandelier Exit
//! 
//! Вычисляет уровни Chandelier на основе ATR от экстремальных цен за период:
//! - Лонг стоп: highest_high за N периодов - (ATR × multiplier)  
//! - Шорт стоп: lowest_low за N периодов + (ATR × multiplier)
//! 
//! В отличие от ATR Trailing, Chandelier "прыгает" вместе с новыми экстремумами,
//! а не использует трейлинг логику.
//! 
//! Индикатор НЕ содержит логику стопов - только возвращает динамические уровни.
//! Логика остановки позиций реализуется в стратегиях на основе этих уровней.

use arrayvec::ArrayVec;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Chandelier Stop индикатор - уровни на основе ATR от экстремумов
#[derive(Debug, Clone)]
pub struct ChandelierStop {
    period: usize,
    multiplier: f64,
    
    // ATR для расчета волатильности
    atr: Atr,
    
    // Буферы для поиска экстремумов
    highs: ArrayVec<f64, 512>,
    lows: ArrayVec<f64, 512>,
    
    // Текущие уровни
    long_stop: f64,     // Стоп для лонга 
    short_stop: f64,    // Стоп для шорта
    
    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl ChandelierStop {
    /// Создать Chandelier Stop с параметрами по умолчанию (period=22, multiplier=3.0)
    pub fn new() -> Self {
        Self::with_params(22, 3.0)
    }
    
    /// Создать Chandelier Stop с настраиваемыми параметрами
    /// * `period` - период для поиска экстремумов и ATR
    /// * `multiplier` - множитель ATR для расчета уровней
    pub fn with_params(period: usize, multiplier: f64) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        assert!(multiplier > 0.0, "Multiplier must be greater than 0");
        
        Self {
            period,
            multiplier,
            atr: Atr::new(period, MovingAverageType::RMA),
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            long_stop: 0.0,
            short_stop: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Создать Chandelier Stop с настраиваемым типом сглаживания для ATR
    pub fn with_atr_type(period: usize, multiplier: f64, atr_type: MovingAverageType) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        assert!(multiplier > 0.0, "Multiplier must be greater than 0");
        
        Self {
            period,
            multiplier,
            atr: Atr::new(period, atr_type),
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            long_stop: 0.0,
            short_stop: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Создать для оптимизации с широким диапазоном параметров
    pub fn for_optimization(period: usize, multiplier: f64, atr_type: MovingAverageType) -> Self {
        Self::with_atr_type(period, multiplier, atr_type)
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> (f64, f64) {
        self.bars_count += 1;
        
        // Обновляем ATR
        let atr_value = self.atr.update_bar(open, high, low, close, volume);
        
        // Добавляем в буферы (с ограничением размера)
        if self.highs.len() >= self.period {
            self.highs.remove(0);
            self.lows.remove(0);
        }
        self.highs.push(high);
        self.lows.push(low);
        
        // Находим экстремумы за весь период
        let highest_high = self.highs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let lowest_low = self.lows.iter().cloned().fold(f64::INFINITY, f64::min);
        
        // Вычисляем уровни Chandelier
        self.long_stop = highest_high - (atr_value * self.multiplier);
        self.short_stop = lowest_low + (atr_value * self.multiplier);
        
        // Готов когда есть достаточно данных
        self.is_ready = self.bars_count >= self.period && self.atr.is_ready();
        
        (self.long_stop, self.short_stop)
    }
    
    /// Получить уровень для лонг позиций (стоп снизу)
    pub fn long_stop(&self) -> f64 {
        self.long_stop
    }
    
    /// Получить уровень для шорт позиций (стоп сверху)
    pub fn short_stop(&self) -> f64 {
        self.short_stop
    }
    
    /// Получить основной уровень (лонг по умолчанию)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.long_stop)
    }
    
    /// Получить оба уровня как кортеж (long, short)
    pub fn levels(&self) -> (f64, f64) {
        (self.long_stop, self.short_stop)
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить текущее значение ATR
    pub fn atr_value(&self) -> f64 {
        self.atr.value().main()
    }
    
    /// Получить highest high за период
    pub fn highest_high(&self) -> f64 {
        if self.highs.is_empty() {
            0.0
        } else {
            self.highs.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        }
    }
    
    /// Получить lowest low за период
    pub fn lowest_low(&self) -> f64 {
        if self.lows.is_empty() {
            0.0
        } else {
            self.lows.iter().cloned().fold(f64::INFINITY, f64::min)
        }
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.atr.reset();
        self.highs.clear();
        self.lows.clear();
        self.long_stop = 0.0;
        self.short_stop = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
    
    /// Получить параметры индикатора (period, multiplier)
    pub fn params(&self) -> (usize, f64) {
        (self.period, self.multiplier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chandelier_stop_creation() {
        let ind = ChandelierStop::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.long_stop(), 0.0);
    }

    #[test]
    fn test_chandelier_stop_with_params() {
        let ind = ChandelierStop::with_params(20, 2.5);
        assert!(!ind.is_ready());
        assert_eq!(ind.params(), (20, 2.5));
    }

    #[test]
    fn test_chandelier_stop_warmup() {
        let mut ind = ChandelierStop::with_params(22, 3.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_chandelier_stop_values_finite() {
        let mut ind = ChandelierStop::with_params(22, 3.0);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (long, short) = ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(long.is_finite());
            assert!(short.is_finite());
        }
    }

    #[test]
    fn test_chandelier_stop_reset() {
        let mut ind = ChandelierStop::with_params(22, 3.0);
        for i in 0..30 {
            ind.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.long_stop(), 0.0);
    }
} 