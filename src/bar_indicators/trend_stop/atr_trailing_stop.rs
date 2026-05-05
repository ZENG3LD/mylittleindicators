//! ATR Trailing Stop - индикатор простых динамических трейлинг уровней
//! 
//! Вычисляет трейлинг уровни на основе ATR от экстремальных цен:
//! - Лонг уровень: highest_high - (ATR × multiplier) - только поднимается
//! - Шорт уровень: lowest_low + (ATR × multiplier) - только опускается  
//! 
//! Индикатор НЕ содержит логику стопов - только возвращает динамические уровни.
//! Логика остановки позиций реализуется в стратегиях на основе этих уровней.

use arrayvec::ArrayVec;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// ATR Trailing Stop индикатор - простые трейлинг уровни на основе ATR
#[derive(Debug, Clone)]
pub struct ATRTrailingStop {
    period: usize,
    multiplier: f64,
    
    // ATR для расчета волатильности
    atr: Atr,
    
    // Буферы для экстремумов
    highs: ArrayVec<f64, 512>,
    lows: ArrayVec<f64, 512>,
    
    // Текущие трейлинг уровни
    long_level: f64,    // Трейлинг стоп для лонга (растет)
    short_level: f64,   // Трейлинг стоп для шорта (падает)
    
    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl ATRTrailingStop {
    /// Создать ATR Trailing Stop с параметрами по умолчанию (period=14, multiplier=2.0)
    pub fn new() -> Self {
        Self::with_params(14, 2.0)
    }
    
    /// Создать ATR Trailing Stop с настраиваемыми параметрами
    /// * `period` - период для ATR и поиска экстремумов
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
            long_level: 0.0,
            short_level: f64::MAX,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Создать ATR Trailing Stop с настраиваемым типом сглаживания для ATR
    pub fn with_atr_type(period: usize, multiplier: f64, atr_type: MovingAverageType) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        assert!(multiplier > 0.0, "Multiplier must be greater than 0");
        
        Self {
            period,
            multiplier,
            atr: Atr::new(period, atr_type),
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            long_level: 0.0,
            short_level: f64::MAX,
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
        
        // Добавляем в буферы
        if self.highs.len() >= self.period {
            self.highs.remove(0);
            self.lows.remove(0);
        }
        self.highs.push(high);
        self.lows.push(low);
        
        // Находим экстремумы за период
        let highest_high = self.highs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let lowest_low = self.lows.iter().cloned().fold(f64::INFINITY, f64::min);
        
        // Вычисляем новые уровни
        let new_long_level = highest_high - (atr_value * self.multiplier);
        let new_short_level = lowest_low + (atr_value * self.multiplier);
        
        // Трейлинг логика - уровни могут только улучшаться
        if self.bars_count == 1 {
            // Инициализация
            self.long_level = new_long_level;
            self.short_level = new_short_level;
        } else {
            // Лонг уровень может только расти (трейлинг вверх)
            if new_long_level > self.long_level || close < self.long_level {
                self.long_level = new_long_level;
            }
            
            // Шорт уровень может только падать (трейлинг вниз)
            if new_short_level < self.short_level || close > self.short_level {
                self.short_level = new_short_level;
            }
        }
        
        // Готов когда есть достаточно данных для ATR
        self.is_ready = self.bars_count >= self.period && self.atr.is_ready();
        
        (self.long_level, self.short_level)
    }
    
    /// Получить уровень для лонг позиций (трейлинг стоп снизу)
    pub fn long_level(&self) -> f64 {
        self.long_level
    }
    
    /// Получить уровень для шорт позиций (трейлинг стоп сверху)
    pub fn short_level(&self) -> f64 {
        self.short_level
    }
    
    /// Получить основной уровень (лонг по умолчанию)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.long_level)
    }
    
    /// Получить оба уровня как кортеж (long, short)
    pub fn levels(&self) -> (f64, f64) {
        (self.long_level, self.short_level)
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить текущее значение ATR
    pub fn atr_value(&self) -> f64 {
        self.atr.value().main()
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.atr.reset();
        self.highs.clear();
        self.lows.clear();
        self.long_level = 0.0;
        self.short_level = f64::MAX;
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
    fn test_atr_trailing_stop_creation() {
        let ind = ATRTrailingStop::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.long_level(), 0.0);
    }

    #[test]
    fn test_atr_trailing_stop_with_params() {
        let ind = ATRTrailingStop::with_params(20, 3.0);
        assert!(!ind.is_ready());
        assert_eq!(ind.params(), (20, 3.0));
    }

    #[test]
    fn test_atr_trailing_stop_warmup() {
        let mut ind = ATRTrailingStop::with_params(14, 2.0);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_atr_trailing_stop_values_finite() {
        let mut ind = ATRTrailingStop::with_params(14, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (long, short) = ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(long.is_finite());
            assert!(short.is_finite());
        }
    }

    #[test]
    fn test_atr_trailing_stop_reset() {
        let mut ind = ATRTrailingStop::with_params(14, 2.0);
        for i in 0..20 {
            ind.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.long_level(), 0.0);
    }
} 