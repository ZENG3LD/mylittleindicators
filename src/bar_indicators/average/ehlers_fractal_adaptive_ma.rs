//! Ehlers Fractal Adaptive Moving Average (FAMA) - фрактальная адаптивная скользящая средняя
//!
//! FAMA адаптируется к изменчивости рынка, используя фрактальную размерность
//! для автоматической настройки периода сглаживания.
//! Чем выше волатильность, тем быстрее реагирует индикатор.
//!
//! Переиспользует существующие компоненты EMA

use crate::bar_indicators::average::ema::Ema;
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;
use super::super::ohlcv_field::OhlcvField;

/// Результат Fractal Adaptive MA
#[derive(Debug, Clone, Copy)]
pub struct FractalAdaptiveResult {
    pub fama: f64,                  // Значение FAMA
    pub fractal_dimension: f64,     // Фрактальная размерность
    pub efficiency_ratio: f64,      // Коэффициент эффективности
    pub adaptive_period: f64,       // Адаптивный период
    pub trend_strength: f64,        // Сила тренда
}

impl FractalAdaptiveResult {
    pub fn empty() -> Self {
        Self {
            fama: 0.0,
            fractal_dimension: 0.0,
            efficiency_ratio: 0.0,
            adaptive_period: 0.0,
            trend_strength: 0.0,
        }
    }
}

/// Ehlers Fractal Adaptive Moving Average
#[derive(Debug, Clone)]
pub struct EhlersFractalAdaptiveMa {
    // Переиспользуем существующие компоненты
    adaptive_ema: Ema,               // Адаптивная EMA с изменяющимся периодом
    base_ema: Ema,                   // Базовая EMA для сравнения

    // Параметры
    period: usize,
    source: OhlcvField,              // Источник данных
    min_period: usize,
    max_period: usize,

    // Буферы для расчетов
    prices: ArrayVec<f64, 512>,

    // Результат
    last_result: FractalAdaptiveResult,

    // Состояние
    initialized: bool,
}

impl EhlersFractalAdaptiveMa {
    pub fn new(period: usize) -> Self {
        Self::with_source(period, OhlcvField::HLC3)
    }

    pub fn with_source(period: usize, source: OhlcvField) -> Self {
        let min_period = (period / 4).max(2);
        let max_period = period * 2;

        Self {
            adaptive_ema: Ema::with_source(period, source),
            base_ema: Ema::with_source(period, source),
            period,
            source,
            min_period,
            max_period,
            prices: ArrayVec::new(),
            last_result: FractalAdaptiveResult::empty(),
            initialized: false,
        }
    }
    
    pub fn period(&self) -> usize {
        self.period
    }
    
    pub fn is_initialized(&self) -> bool {
        self.initialized && self.prices.len() >= self.period
    }
    
    /// Обновление с одним значением цены
    pub fn update(&mut self, price: f64) -> f64 {
        self.update_bar(price, price, price, price, 1.0)
    }
    
    /// Обновление с OHLCV данными (использует настроенный source)
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let price = self.source.extract(open, high, low, close, volume);

        // Добавляем новую цену
        if self.prices.len() >= 512 {
            self.prices.remove(0);
        }
        self.prices.push(price);

        // Обновляем базовую EMA
        let _base_value = self.base_ema.update_bar(open, high, low, close, volume);
        
        if self.prices.len() < self.period {
            let adaptive_value = self.adaptive_ema.update_bar(open, high, low, close, volume);
            return adaptive_value;
        }
        
        // Вычисляем фрактальную размерность
        let fractal_dim = self.calculate_fractal_dimension();
        
        // Вычисляем коэффициент эффективности  
        let efficiency = self.calculate_efficiency_ratio();
        
        // Адаптивный период на основе фрактальной размерности
        let adaptive_period = self.calculate_adaptive_period(fractal_dim, efficiency);
        
        // Создаем новую адаптивную EMA с рассчитанным периодом
        let new_period = adaptive_period.round() as usize;
        if new_period != self.adaptive_ema.period() {
            self.adaptive_ema = Ema::with_source(new_period, self.source);
        }

        // Обновляем адаптивную EMA
        let adaptive_value = self.adaptive_ema.update_bar(open, high, low, close, volume);
        
        // Обновляем результат
        self.last_result = FractalAdaptiveResult {
            fama: adaptive_value,
            fractal_dimension: fractal_dim,
            efficiency_ratio: efficiency,
            adaptive_period,
            trend_strength: efficiency * (2.0 - fractal_dim), // Комбинированная метрика
        };
        
        self.initialized = true;
        adaptive_value
    }
    
    /// Вычисление фрактальной размерности
    fn calculate_fractal_dimension(&self) -> f64 {
        if self.prices.len() < self.period {
            return 1.5; // Средняя фрактальная размерность
        }
        
        let mut total_length = 0.0;

        let start_idx = self.prices.len() - self.period;

        // Вычисляем общую длину пути
        for i in 1..self.period {
            let prev = self.prices[start_idx + i - 1];
            let curr = self.prices[start_idx + i];
            total_length += (curr - prev).abs();
        }

        // Прямое расстояние
        let direct_distance = (self.prices[start_idx + self.period - 1] - self.prices[start_idx]).abs();
        
        if direct_distance == 0.0 || total_length == 0.0 {
            return 1.5;
        }
        
        // Фрактальная размерность
        let fractal_dim = (total_length / direct_distance).ln() / (self.period as f64).ln();
        
        // Ограничиваем значения от 1.0 до 2.0
        fractal_dim.clamp(1.0, 2.0)
    }
    
    /// Вычисление коэффициента эффективности (как в AMA)
    fn calculate_efficiency_ratio(&self) -> f64 {
        if self.prices.len() < self.period {
            return 0.0;
        }
        
        let start_idx = self.prices.len() - self.period;
        
        // Направленное движение
        let direction = (self.prices[start_idx + self.period - 1] - self.prices[start_idx]).abs();
        
        // Волатильность (сумма абсолютных изменений)
        let mut volatility = 0.0;
        for i in 1..self.period {
            volatility += (self.prices[start_idx + i] - self.prices[start_idx + i - 1]).abs();
        }
        
        if volatility == 0.0 {
            return 0.0;
        }
        
        direction / volatility
    }
    
    /// Вычисление адаптивного периода
    fn calculate_adaptive_period(&self, fractal_dim: f64, efficiency: f64) -> f64 {
        // Комбинируем фрактальную размерность и эффективность
        let complexity_factor = fractal_dim; // 1.0-2.0
        let efficiency_factor = efficiency;   // 0.0-1.0
        
        // Высокая сложность (fractal_dim близко к 2.0) = медленная адаптация
        // Высокая эффективность = быстрая адаптация  
        let adaptation_speed = efficiency_factor / complexity_factor;
        
        // Интерполируем между min_period и max_period
        let adaptive_period = self.max_period as f64 - 
            adaptation_speed * (self.max_period - self.min_period) as f64;
            
        adaptive_period.max(self.min_period as f64).min(self.max_period as f64)
    }
    
    /// Получить последний результат
    pub fn result(&self) -> FractalAdaptiveResult {
        self.last_result
    }
    
    /// Получить значение FAMA
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_result.fama)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.prices.len() >= self.period
    }

    /// Сброс индикатора
    pub fn reset(&mut self) {
        self.prices.clear();
        self.adaptive_ema = Ema::new(self.period);
        self.base_ema = Ema::new(self.period);
        self.last_result = FractalAdaptiveResult::empty();
        self.initialized = false;
    }
}