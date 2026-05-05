//! SuperTrend Stop - индикатор динамических уровней на основе SuperTrend
//! 
//! Вычисляет уровни SuperTrend для создания динамических уровней поддержки/сопротивления.
//! Индикатор НЕ содержит логику стопов - только возвращает уровни SuperTrend.
//! Логика остановки позиций реализуется в стратегиях на основе этих уровней.

use crate::bar_indicators::trend::supertrend::Supertrend;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// SuperTrend Stop индикатор - вычисляет динамические уровни на основе SuperTrend
#[derive(Debug, Clone)]
pub struct SuperTrendStop {
    supertrend: Supertrend,
    current_level: f64,
    trend_direction: i8, // 1 = up trend, -1 = down trend
}

impl SuperTrendStop {
    /// Создать SuperTrend Stop с параметрами по умолчанию (period=10, multiplier=3.0)
    pub fn new() -> Self {
        Self::with_params(10, 3.0)
    }
    
    /// Создать SuperTrend Stop с настраиваемыми параметрами
    /// * `period` - период для ATR (обычно 10-14)
    /// * `multiplier` - множитель ATR (обычно 2.0-3.0)
    pub fn with_params(period: usize, multiplier: f64) -> Self {
        Self {
            supertrend: Supertrend::with_params(period, multiplier),
            current_level: 0.0,
            trend_direction: 1,
        }
    }
    
    /// Создать SuperTrend Stop с широким диапазоном параметров для оптимизации
    pub fn for_optimization(period: usize, multiplier: f64) -> Self {
        Self::with_params(period, multiplier)
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let supertrend_value = self.supertrend.update_bar(open, high, low, close, volume);
        self.current_level = supertrend_value;
        
        // Получаем направление тренда из SuperTrend
        self.trend_direction = self.supertrend.trend_direction();
        
        self.current_level
    }
    
    /// Получить текущий уровень SuperTrend
    pub fn level(&self) -> f64 {
        self.current_level
    }
    
    /// Получить текущий уровень (alias для level)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_level)
    }
    
    /// Получить направление тренда (1 = восходящий, -1 = нисходящий)
    pub fn trend_direction(&self) -> i8 {
        self.trend_direction
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.supertrend.is_ready()
    }
    
    /// Получить уровень для лонг позиций (SuperTrend при восходящем тренде)
    pub fn long_level(&self) -> Option<f64> {
        if self.trend_direction == 1 && self.is_ready() {
            Some(self.current_level)
        } else {
            None
        }
    }
    
    /// Получить уровень для шорт позиций (SuperTrend при нисходящем тренде)
    pub fn short_level(&self) -> Option<f64> {
        if self.trend_direction == -1 && self.is_ready() {
            Some(self.current_level)
        } else {
            None
        }
    }
    
    /// Получить информацию об уровне (поддержка или сопротивление)
    pub fn support_resistance_info(&self) -> (&'static str, f64) {
        self.supertrend.support_resistance_level()
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.supertrend.reset();
        self.current_level = 0.0;
        self.trend_direction = 1;
    }
    
    /// Получить параметры индикатора (period, multiplier)
    pub fn params(&self) -> (usize, f64) {
        self.supertrend.parameters()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supertrend_stop_creation() {
        let ind = SuperTrendStop::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_supertrend_stop_with_params() {
        let ind = SuperTrendStop::with_params(14, 2.5);
        assert!(!ind.is_ready());
        assert_eq!(ind.params(), (14, 2.5));
    }

    #[test]
    fn test_supertrend_stop_warmup() {
        let mut ind = SuperTrendStop::with_params(10, 3.0);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_supertrend_stop_values_finite() {
        let mut ind = SuperTrendStop::with_params(10, 3.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_supertrend_stop_trend_direction() {
        let mut ind = SuperTrendStop::with_params(10, 3.0);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        let dir = ind.trend_direction();
        assert!(dir == 1 || dir == -1);
    }

    #[test]
    fn test_supertrend_stop_reset() {
        let mut ind = SuperTrendStop::with_params(10, 3.0);
        for i in 0..20 {
            ind.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
} 