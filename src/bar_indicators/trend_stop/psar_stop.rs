//! PSAR Stop - индикатор динамических уровней на основе Parabolic SAR
//! 
//! Вычисляет значения Parabolic SAR для создания динамических уровней поддержки/сопротивления.
//! Индикатор НЕ содержит логику стопов - только возвращает уровни SAR.
//! Логика остановки позиций реализуется в стратегиях на основе этих уровней.

use crate::bar_indicators::momentum::parabolic_sar::ParabolicSAR;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// PSAR Stop индикатор - вычисляет динамические уровни на основе Parabolic SAR
#[derive(Debug, Clone)]
pub struct PSARStop {
    psar: ParabolicSAR,
    current_level: f64,
    trend_direction: i8, // 1 = up trend, -1 = down trend
}

impl PSARStop {
    /// Создать PSAR Stop с параметрами по умолчанию (0.02, 0.02, 0.20)
    pub fn new() -> Self {
        Self::with_params(0.02, 0.02, 0.20)
    }
    
    /// Создать PSAR Stop с настраиваемыми параметрами
    /// * `af_start` - начальный фактор ускорения (обычно 0.02)
    /// * `af_increment` - шаг увеличения AF (обычно 0.02) 
    /// * `af_max` - максимальный AF (обычно 0.20)
    pub fn with_params(af_start: f64, af_increment: f64, af_max: f64) -> Self {
        Self {
            psar: ParabolicSAR::with_params(af_start, af_increment, af_max),
            current_level: 0.0,
            trend_direction: 1,
        }
    }
    
    /// Создать PSAR Stop с широким диапазоном параметров для оптимизации
    pub fn for_optimization(af_start: f64, af_increment: f64, af_max: f64) -> Self {
        Self::with_params(af_start, af_increment, af_max)
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let psar_value = self.psar.update_bar(open, high, low, close, volume);
        self.current_level = psar_value;
        
        // Определяем направление тренда (SAR выше цены = down trend, ниже цены = up trend)
        self.trend_direction = if psar_value < close { 1 } else { -1 };
        
        self.current_level
    }
    
    /// Получить текущий уровень PSAR
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
        self.psar.is_ready()
    }
    
    /// Получить уровень для лонг позиций (SAR при восходящем тренде)
    pub fn long_level(&self) -> Option<f64> {
        if self.trend_direction == 1 && self.is_ready() {
            Some(self.current_level)
        } else {
            None
        }
    }
    
    /// Получить уровень для шорт позиций (SAR при нисходящем тренде) 
    pub fn short_level(&self) -> Option<f64> {
        if self.trend_direction == -1 && self.is_ready() {
            Some(self.current_level)
        } else {
            None
        }
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.psar.reset();
        self.current_level = 0.0;
        self.trend_direction = 1;
    }
    
    /// Получить параметры индикатора
    pub fn params(&self) -> (f64, f64, f64) {
        self.psar.params()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psar_stop_creation() {
        let ind = PSARStop::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_psar_stop_with_params() {
        let ind = PSARStop::with_params(0.02, 0.02, 0.2);
        assert!(!ind.is_ready());
        assert_eq!(ind.params(), (0.02, 0.02, 0.2));
    }

    #[test]
    fn test_psar_stop_warmup() {
        let mut ind = PSARStop::with_params(0.02, 0.02, 0.2);
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_psar_stop_values_finite() {
        let mut ind = PSARStop::with_params(0.02, 0.02, 0.2);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_psar_stop_trend_direction() {
        let mut ind = PSARStop::with_params(0.02, 0.02, 0.2);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        let dir = ind.trend_direction();
        assert!(dir == 1 || dir == -1);
    }

    #[test]
    fn test_psar_stop_reset() {
        let mut ind = PSARStop::with_params(0.02, 0.02, 0.2);
        for i in 0..15 {
            ind.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
} 