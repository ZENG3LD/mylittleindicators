//! Обертка для расширенной FRAMA (FractalAdaptiveMovingAverage)
//! Обеспечивает совместимость с MovingAverage интерфейсом

use crate::bar_indicators::adaptive::frama::{FractalAdaptiveMovingAverage, FractalMethod};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Обертка для расширенной FRAMA с конфигурируемыми параметрами
#[derive(Debug, Clone)]
pub struct FramaAdvanced {
    inner: FractalAdaptiveMovingAverage,
    period: usize,
}

impl FramaAdvanced {
    /// Создает новую расширенную FRAMA с настройками по умолчанию
    pub fn new(period: usize) -> Self {
        let period = period.clamp(1, 512); // Разрешаем период от 1
        Self {
            inner: FractalAdaptiveMovingAverage::new(period, FractalMethod::Standard),
            period,
        }
    }
    
    /// Создает расширенную FRAMA с выбранным методом
    pub fn new_with_method(period: usize, method: FractalMethod) -> Self {
        let period = period.clamp(1, 512);
        Self {
            inner: FractalAdaptiveMovingAverage::new(period, method),
            period,
        }
    }
    
    /// Создает расширенную FRAMA с настройками alpha bounds
    pub fn new_with_alpha_bounds(period: usize, min_alpha: f64, max_alpha: f64) -> Self {
        let period = period.clamp(1, 512);
        let mut frama = FractalAdaptiveMovingAverage::new(period, FractalMethod::Standard);
        frama.set_alpha_bounds(min_alpha, max_alpha);
        Self {
            inner: frama,
            period,
        }
    }
    
    /// Создает полностью настраиваемую расширенную FRAMA
    pub fn new_custom(
        period: usize, 
        method: FractalMethod, 
        min_alpha: f64, 
        max_alpha: f64,
        dimension_smoothing: f64
    ) -> Self {
        let period = period.clamp(1, 512);
        let mut frama = FractalAdaptiveMovingAverage::new(period, method);
        frama.set_alpha_bounds(min_alpha, max_alpha);
        frama.set_dimension_smoothing(dimension_smoothing);
        Self {
            inner: frama,
            period,
        }
    }
    
    pub fn period(&self) -> usize {
        self.period
    }
    
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }
    
    pub fn reset(&mut self) {
        self.inner.reset();
    }
    
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        // Используем OHLC данные для расширенной FRAMA
        let result = self.inner.update_ohlc(open, high, low, close);
        result.value
    }
    
    pub fn value(&self) -> IndicatorValue {
        self.inner.value()
    }
    
    pub fn is_initialized(&self) -> bool {
        self.inner.is_ready()
    }
    
    // Дополнительные методы расширенной FRAMA
    
    /// Получить фрактальную размерность
    pub fn fractal_dimension(&self) -> f64 {
        self.inner.fractal_dimension()
    }
    
    /// Получить текущий коэффициент сглаживания (alpha)
    pub fn alpha(&self) -> f64 {
        self.inner.alpha()
    }
    
    /// Получить эффективность движения
    pub fn efficiency(&self) -> f64 {
        self.inner.efficiency()
    }
    
    /// Получить уровень шума
    pub fn noise_level(&self) -> f64 {
        self.inner.noise_level()
    }
    
    /// Получить силу тренда
    pub fn trend_strength(&self) -> f64 {
        self.inner.trend_strength()
    }
    
    /// Получить метод расчета
    pub fn method(&self) -> FractalMethod {
        self.inner.method()
    }
    
    /// Получить границы alpha
    pub fn alpha_bounds(&self) -> (f64, f64) {
        self.inner.alpha_bounds()
    }
    
    /// Установить границы alpha
    pub fn set_alpha_bounds(&mut self, min_alpha: f64, max_alpha: f64) {
        self.inner.set_alpha_bounds(min_alpha, max_alpha);
    }
    
    /// Установить сглаживание размерности
    pub fn set_dimension_smoothing(&mut self, alpha: f64) {
        self.inner.set_dimension_smoothing(alpha);
    }
    
    /// Получить статистику периодов
    pub fn trend_periods(&self) -> usize {
        self.inner.trend_periods()
    }
    
    pub fn ranging_periods(&self) -> usize {
        self.inner.ranging_periods()
    }
    
    pub fn high_noise_periods(&self) -> usize {
        self.inner.high_noise_periods()
    }
    
    /// Получить историю размерности
    pub fn dimension_history(&self) -> &[f64] {
        self.inner.dimension_history()
    }
    
    /// Получить историю alpha
    pub fn alpha_history(&self) -> &[f64] {
        self.inner.alpha_history()
    }
    
    /// Получить историю эффективности
    pub fn efficiency_history(&self) -> &[f64] {
        self.inner.efficiency_history()
    }
    
    /// Прогноз на несколько периодов вперед
    pub fn forecast(&self, periods: usize) -> Vec<f64> {
        self.inner.forecast(periods)
    }
} 






















