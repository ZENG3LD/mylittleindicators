//! Fractal Adaptive Moving Average (FRAMA)
//! Фрактальная адаптивная скользящая средняя
//! Адаптируется к рыночным условиям на основе фрактальной размерности

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Метод вычисления фрактальной размерности
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FractalMethod {
    Standard,       // Стандартный метод Эрса
    Improved,       // Улучшенный метод с дополнительным сглаживанием
    Dynamic,        // Динамический метод с адаптивным периодом
    Robust,         // Устойчивый к выбросам метод
}

/// Результат FRAMA
#[derive(Debug, Clone)]
pub struct FramaResult {
    pub value: f64,                 // Значение FRAMA
    pub fractal_dimension: f64,     // Фрактальная размерность (1-2)
    pub efficiency: f64,            // Эффективность движения (0-1)
    pub alpha: f64,                 // Текущий коэффициент сглаживания
    pub noise_level: f64,           // Уровень шума
    pub trend_strength: f64,        // Сила тренда
    pub volatility_adjustment: f64, // Поправка на волатильность
}

impl Default for FramaResult {
    fn default() -> Self {
        Self::new()
    }
}

impl FramaResult {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            fractal_dimension: 1.5,
            efficiency: 0.0,
            alpha: 0.0,
            noise_level: 0.0,
            trend_strength: 0.0,
            volatility_adjustment: 1.0,
        }
    }
}

/// Fractal Adaptive Moving Average
#[derive(Debug, Clone)]
pub struct FractalAdaptiveMovingAverage {
    // Параметры
    period: usize,
    method: FractalMethod,
    
    // Данные
    prices: ArrayVec<f64, 512>,         // Окно цен
    high_prices: ArrayVec<f64, 512>,    // Максимумы
    low_prices: ArrayVec<f64, 512>,     // Минимумы
    
    // Параметры адаптации
    min_alpha: f64,             // Минимальный альфа (медленная адаптация)
    max_alpha: f64,             // Максимальный альфа (быстрая адаптация)
    
    // Результаты
    current_result: FramaResult,
    
    // История для анализа
    dimension_history: ArrayVec<f64, 100>,
    alpha_history: ArrayVec<f64, 100>,
    efficiency_history: ArrayVec<f64, 100>,
    
    // Дополнительные данные для улучшенных методов
    smoothed_dimension: f64,
    dimension_ema_alpha: f64,
    
    // Статистики
    trend_periods: usize,
    ranging_periods: usize,
    high_noise_periods: usize,
    
    // Состояние
    is_initialized: bool,
}

impl FractalAdaptiveMovingAverage {
    pub fn new(period: usize, method: FractalMethod) -> Self {
        let period = period.clamp(4, 512); // Минимум 4 для расчета размерности
        
        Self {
            period,
            method,
            prices: ArrayVec::new(),
            high_prices: ArrayVec::new(),
            low_prices: ArrayVec::new(),
            min_alpha: 0.01,  // 1% для медленной адаптации
            max_alpha: 1.0,   // 100% для мгновенной адаптации
            current_result: FramaResult::new(),
            dimension_history: ArrayVec::new(),
            alpha_history: ArrayVec::new(),
            efficiency_history: ArrayVec::new(),
            smoothed_dimension: 1.5,
            dimension_ema_alpha: 0.2,
            trend_periods: 0,
            ranging_periods: 0,
            high_noise_periods: 0,
            is_initialized: false,
        }
    }
    
    /// Обновление FRAMA с OHLC данными
    pub fn update_ohlc(&mut self, _open: f64, high: f64, low: f64, close: f64) -> &FramaResult {
        self.update_with_hl(close, high, low)
    }
    
    /// Обновление FRAMA с ценой и high/low
    pub fn update_with_hl(&mut self, price: f64, high: f64, low: f64) -> &FramaResult {
        // Добавляем данные
        if self.prices.len() >= self.period {
            self.prices.remove(0);
            self.high_prices.remove(0);
            self.low_prices.remove(0);
        }
        
        if !self.prices.is_full() {
            self.prices.push(price);
        }
        if !self.high_prices.is_full() {
            self.high_prices.push(high);
        }
        if !self.low_prices.is_full() {
            self.low_prices.push(low);
        }
        
        if self.prices.len() < self.period {
            self.current_result.value = price;
            return &self.current_result;
        }
        
        // Вычисляем фрактальную размерность
        let dimension = self.calculate_fractal_dimension();
        self.current_result.fractal_dimension = dimension;
        
        // Сглаживаем размерность для улучшенных методов
        if matches!(self.method, FractalMethod::Improved | FractalMethod::Dynamic) {
            self.smoothed_dimension = self.dimension_ema_alpha * dimension + 
                                    (1.0 - self.dimension_ema_alpha) * self.smoothed_dimension;
        } else {
            self.smoothed_dimension = dimension;
        }
        
        // Вычисляем коэффициент сглаживания
        let alpha = self.calculate_alpha();
        self.current_result.alpha = alpha;
        
        // Обновляем FRAMA
        if !self.is_initialized {
            self.current_result.value = price;
            self.is_initialized = true;
        } else {
            self.current_result.value = alpha * price + (1.0 - alpha) * self.current_result.value;
        }
        
        // Вычисляем дополнительные метрики
        self.calculate_additional_metrics();
        
        // Сохраняем историю
        self.save_history();
        
        &self.current_result
    }
    
    /// Обновление FRAMA только с ценой закрытия
    pub fn update(&mut self, price: f64) -> &FramaResult {
        // Используем цену как high и low (упрощенный режим)
        self.update_with_hl(price, price, price)
    }
    
    /// Вычисление фрактальной размерности
    fn calculate_fractal_dimension(&self) -> f64 {
        match self.method {
            FractalMethod::Standard => self.calculate_standard_dimension(),
            FractalMethod::Improved => self.calculate_improved_dimension(),
            FractalMethod::Dynamic => self.calculate_dynamic_dimension(),
            FractalMethod::Robust => self.calculate_robust_dimension(),
        }
    }
    
    /// Стандартный метод Эрса
    fn calculate_standard_dimension(&self) -> f64 {
        let n = self.period;
        let n1 = (self.max_value() - self.min_value()) / n as f64;
        
        if n1 <= 1e-12 {
            return 1.0;
        }
        
        let n2 = self.calculate_total_variation() / (n - 1) as f64;
        
        if n2 <= 1e-12 || n1 <= 1e-12 {
            return 1.0;
        }
        
        let dimension = (n2 / n1).ln() / (2.0_f64.ln());
        
        // Ограничиваем размерность в разумных пределах
        dimension.clamp(1.0, 2.0)
    }
    
    /// Улучшенный метод с дополнительным сглаживанием
    fn calculate_improved_dimension(&self) -> f64 {
        let base_dimension = self.calculate_standard_dimension();
        
        // Добавляем коррекцию на основе волатильности
        let volatility = self.calculate_volatility();
        let volatility_factor = 1.0 + (volatility - 0.5).max(0.0) * 0.2;
        
        let adjusted_dimension = base_dimension * volatility_factor;
        adjusted_dimension.clamp(1.0, 2.0)
    }
    
    /// Динамический метод с адаптивным периодом
    fn calculate_dynamic_dimension(&self) -> f64 {
        // Используем адаптивный период на основе текущей волатильности
        let volatility = self.calculate_volatility();
        let adaptive_period = if volatility > 0.7 {
            (self.period as f64 * 0.7) as usize  // Короткий период для высокой волатильности
        } else if volatility < 0.3 {
            (self.period as f64 * 1.3) as usize  // Длинный период для низкой волатильности
        } else {
            self.period
        }.min(self.prices.len());
        
        // Рассчитываем размерность для адаптивного периода
        let start_idx = self.prices.len() - adaptive_period;
        let max_val = self.high_prices[start_idx..].iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min_val = self.low_prices[start_idx..].iter().fold(f64::INFINITY, |a, &b| a.min(b));
        
        let n1 = (max_val - min_val) / adaptive_period as f64;
        
        let mut total_variation = 0.0;
        for i in (start_idx + 1)..self.prices.len() {
            total_variation += (self.prices[i] - self.prices[i - 1]).abs();
        }
        let n2 = total_variation / (adaptive_period - 1) as f64;
        
        if n2 <= 1e-12 || n1 <= 1e-12 {
            return 1.0;
        }
        
        let dimension = (n2 / n1).ln() / (2.0_f64.ln());
        dimension.clamp(1.0, 2.0)
    }
    
    /// Устойчивый к выбросам метод
    fn calculate_robust_dimension(&self) -> f64 {
        // Используем медианы вместо средних для устойчивости к выбросам
        let mut price_changes: ArrayVec<f64, 512> = ArrayVec::new();
        
        for i in 1..self.prices.len() {
            let change = (self.prices[i] - self.prices[i - 1]).abs();
            if !price_changes.is_full() {
                price_changes.push(change);
            }
        }
        
        // Сортируем для вычисления медианы
        let mut sorted_changes = price_changes.clone();
        sorted_changes.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let median_change = if !sorted_changes.is_empty() {
            let mid = sorted_changes.len() / 2;
            if sorted_changes.len().is_multiple_of(2) {
                (sorted_changes[mid - 1] + sorted_changes[mid]) / 2.0
            } else {
                sorted_changes[mid]
            }
        } else {
            0.0
        };
        
        // Устойчивая оценка размаха
        let mut hl_ranges: ArrayVec<f64, 512> = ArrayVec::new();
        for i in 0..self.high_prices.len() {
            if !hl_ranges.is_full() {
                hl_ranges.push(self.high_prices[i] - self.low_prices[i]);
            }
        }
        
        hl_ranges.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let _median_range = if !hl_ranges.is_empty() {
            let mid = hl_ranges.len() / 2;
            if hl_ranges.len().is_multiple_of(2) {
                (hl_ranges[mid - 1] + hl_ranges[mid]) / 2.0
            } else {
                hl_ranges[mid]
            }
        } else {
            0.0
        };
        
        let total_range = self.max_value() - self.min_value();
        let n1 = total_range / self.period as f64;
        let n2 = median_change;
        
        if n2 <= 1e-12 || n1 <= 1e-12 {
            return 1.0;
        }
        
        let dimension = (n2 / n1).ln() / (2.0_f64.ln());
        dimension.clamp(1.0, 2.0)
    }
    
    /// Вычисление коэффициента сглаживания
    fn calculate_alpha(&self) -> f64 {
        let dimension = match self.method {
            FractalMethod::Improved | FractalMethod::Dynamic => self.smoothed_dimension,
            _ => self.current_result.fractal_dimension,
        };
        
        // Стандартная формула FRAMA
        let alpha = (-4.6 * (dimension - 1.0)).exp();
        
        // Ограничиваем диапазон
        alpha.max(self.min_alpha).min(self.max_alpha)
    }
    
    /// Вспомогательные методы
    fn max_value(&self) -> f64 {
        self.high_prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))
    }
    
    fn min_value(&self) -> f64 {
        self.low_prices.iter().fold(f64::INFINITY, |a, &b| a.min(b))
    }
    
    fn calculate_total_variation(&self) -> f64 {
        let mut total = 0.0;
        for i in 1..self.prices.len() {
            total += (self.prices[i] - self.prices[i - 1]).abs();
        }
        total
    }
    
    fn calculate_volatility(&self) -> f64 {
        if self.prices.len() < 2 {
            return 0.5;
        }
        
        let mean = self.prices.iter().sum::<f64>() / self.prices.len() as f64;
        let variance: f64 = self.prices.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / self.prices.len() as f64;
        
        let std_dev = variance.sqrt();
        let relative_volatility = std_dev / mean.abs().max(1e-12);
        
        // Нормализуем в диапазон 0-1
        (relative_volatility * 100.0).clamp(0.0, 1.0)
    }
    
    /// Вычисление дополнительных метрик
    fn calculate_additional_metrics(&mut self) {
        // Эффективность движения
        if self.prices.len() >= 2 {
            let net_change = (self.prices.last().unwrap() - self.prices[0]).abs();
            let total_movement = self.calculate_total_variation();
            
            self.current_result.efficiency = if total_movement > 1e-12 {
                net_change / total_movement
            } else {
                0.0
            };
        }
        
        // Уровень шума (обратно пропорционален эффективности)
        self.current_result.noise_level = 1.0 - self.current_result.efficiency;
        
        // Сила тренда на основе размерности
        self.current_result.trend_strength = (2.0 - self.current_result.fractal_dimension) / 1.0;
        
        // Поправка на волатильность
        let volatility = self.calculate_volatility();
        self.current_result.volatility_adjustment = if volatility > 0.7 {
            1.2  // Увеличиваем чувствительность в волатильные периоды  
        } else if volatility < 0.3 {
            0.8  // Уменьшаем чувствительность в спокойные периоды
        } else {
            1.0
        };
        
        // Статистика периодов
        if self.current_result.trend_strength > 0.6 {
            self.trend_periods += 1;
        } else if self.current_result.trend_strength < 0.3 {
            self.ranging_periods += 1;
        }
        
        if self.current_result.noise_level > 0.7 {
            self.high_noise_periods += 1;
        }
    }
    
    /// Сохранение истории
    fn save_history(&mut self) {
        // Размерность
        if self.dimension_history.len() >= 100 {
            self.dimension_history.remove(0);
        }
        if !self.dimension_history.is_full() {
            self.dimension_history.push(self.current_result.fractal_dimension);
        }
        
        // Альфа
        if self.alpha_history.len() >= 100 {
            self.alpha_history.remove(0);
        }
        if !self.alpha_history.is_full() {
            self.alpha_history.push(self.current_result.alpha);
        }
        
        // Эффективность
        if self.efficiency_history.len() >= 100 {
            self.efficiency_history.remove(0);
        }
        if !self.efficiency_history.is_full() {
            self.efficiency_history.push(self.current_result.efficiency);
        }
    }
    
    // Публичные методы

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_result.value)
    }
    
    pub fn fractal_dimension(&self) -> f64 {
        self.current_result.fractal_dimension
    }
    
    pub fn efficiency(&self) -> f64 {
        self.current_result.efficiency
    }
    
    pub fn alpha(&self) -> f64 {
        self.current_result.alpha
    }
    
    pub fn noise_level(&self) -> f64 {
        self.current_result.noise_level
    }
    
    pub fn trend_strength(&self) -> f64 {
        self.current_result.trend_strength
    }
    
    pub fn volatility_adjustment(&self) -> f64 {
        self.current_result.volatility_adjustment
    }
    
    pub fn period(&self) -> usize {
        self.period
    }
    
    pub fn method(&self) -> FractalMethod {
        self.method
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_initialized && self.prices.len() == self.period
    }
    
    pub fn trend_periods(&self) -> usize {
        self.trend_periods
    }
    
    pub fn ranging_periods(&self) -> usize {
        self.ranging_periods
    }
    
    pub fn high_noise_periods(&self) -> usize {
        self.high_noise_periods
    }
    
    pub fn dimension_history(&self) -> &[f64] {
        &self.dimension_history
    }
    
    pub fn alpha_history(&self) -> &[f64] {
        &self.alpha_history
    }
    
    pub fn efficiency_history(&self) -> &[f64] {
        &self.efficiency_history
    }
    
    /// Установка границ адаптации
    pub fn set_alpha_bounds(&mut self, min_alpha: f64, max_alpha: f64) {
        self.min_alpha = min_alpha.clamp(0.001, 0.5);
        self.max_alpha = max_alpha.min(1.0).max(self.min_alpha);
    }
    
    pub fn alpha_bounds(&self) -> (f64, f64) {
        (self.min_alpha, self.max_alpha)
    }
    
    /// Установка параметров сглаживания размерности
    pub fn set_dimension_smoothing(&mut self, alpha: f64) {
        self.dimension_ema_alpha = alpha.clamp(0.01, 1.0);
    }
    
    pub fn reset(&mut self) {
        self.prices.clear();
        self.high_prices.clear();
        self.low_prices.clear();
        self.current_result = FramaResult::new();
        self.dimension_history.clear();
        self.alpha_history.clear();
        self.efficiency_history.clear();
        self.smoothed_dimension = 1.5;
        self.trend_periods = 0;
        self.ranging_periods = 0;
        self.high_noise_periods = 0;
        self.is_initialized = false;
    }
    
    /// Прогнозирование на основе текущего тренда
    pub fn forecast(&self, periods: usize) -> Vec<f64> {
        if !self.is_ready() || periods == 0 {
            return vec![];
        }

        let mut forecasts = Vec::with_capacity(periods);
        let mut current_value = self.current_result.value;

        // Определяем направление тренда
        let trend_direction = if self.prices.len() >= 3 {
            let recent_change = self.prices.last().unwrap() - self.prices[self.prices.len() - 3];
            if recent_change.abs() < 1e-12 {
                0.0
            } else {
                recent_change.signum()
            }
        } else {
            0.0
        };

        let trend_magnitude = self.current_result.trend_strength * 0.01; // 1% базовое изменение

        for i in 0..periods {
            let decay = 0.9_f64.powi(i as i32); // Затухающий тренд
            let change = trend_direction * trend_magnitude * decay;
            current_value *= 1.0 + change;
            forecasts.push(current_value);
        }

        forecasts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frama_creation() {
        let ind = FractalAdaptiveMovingAverage::new(16, FractalMethod::Standard);
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_frama_warmup() {
        let mut ind = FractalAdaptiveMovingAverage::new(10, FractalMethod::Standard);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update_with_hl(price, price + 1.0, price - 1.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_frama_values_finite() {
        let mut ind = FractalAdaptiveMovingAverage::new(10, FractalMethod::Improved);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 5.0;
            ind.update_with_hl(price, price + 1.0, price - 1.0);
        }
        assert!(ind.value().main().is_finite());
        assert!(ind.fractal_dimension() >= 1.0 && ind.fractal_dimension() <= 2.0);
        assert!(ind.alpha().is_finite());
    }

    #[test]
    fn test_frama_methods() {
        let methods = [
            FractalMethod::Standard,
            FractalMethod::Improved,
            FractalMethod::Dynamic,
            FractalMethod::Robust,
        ];
        for method in methods {
            let mut ind = FractalAdaptiveMovingAverage::new(10, method);
            for i in 0..20 {
                let price = 100.0 + i as f64;
                ind.update_with_hl(price, price + 1.0, price - 1.0);
            }
            assert!(ind.is_ready());
            assert!(ind.value().main().is_finite());
        }
    }

    #[test]
    fn test_frama_reset() {
        let mut ind = FractalAdaptiveMovingAverage::new(10, FractalMethod::Standard);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            ind.update_with_hl(price, price + 1.0, price - 1.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
} 






















