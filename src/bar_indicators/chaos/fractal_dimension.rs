//! Fractal Dimension - фрактальная размерность по методу Хигучи
//! Измеряет сложность и "изломанность" временного ряда цен
//! Значения: 1.0-2.0, где 1.0 = тренд, 1.5 = случайное блуждание, 2.0 = максимальный шум

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Фрактальная размерность по методу Хигучи
#[derive(Clone)]
pub struct FractalDimension {
    period: usize,
    max_k: usize, // Максимальное значение k для алгоритма Хигучи
    
    // Буфер цен
    prices: ArrayVec<f64, 512>,
    
    // Результат
    fractal_dimension: f64,
    complexity_score: f64, // 0.0-1.0, где 1.0 = максимальная сложность
    
    // Состояние
    is_ready: bool,
}

impl FractalDimension {
    pub fn new(period: usize, max_k: usize) -> Self {
        Self {
            period: period.min(512),
            max_k: max_k.min(period / 4).max(2), // k не должно быть больше period/4
            prices: ArrayVec::new(),
            fractal_dimension: 1.5, // Начальное значение (случайное блуждание)
            complexity_score: 0.5,
            is_ready: false,
        }
    }
    
    /// Обновить индикатор новой ценой
    pub fn update(&mut self, price: f64) -> f64 {
        // Добавляем цену в буфер
        if self.prices.len() >= self.period {
            self.prices.remove(0);
        }
        self.prices.push(price);
        
        // Рассчитываем фрактальную размерность если достаточно данных
        if self.prices.len() >= self.period {
            self.calculate_fractal_dimension();
            self.is_ready = true;
        }
        
        self.fractal_dimension
    }
    
    /// Рассчитать фрактальную размерность по методу Хигучи
    fn calculate_fractal_dimension(&mut self) {
        let _n = self.prices.len();
        let mut k_values = Vec::new();
        let mut l_k_values = Vec::new();
        
        // Рассчитываем L(k) для различных значений k
        for k in 1..=self.max_k {
            let l_k = self.calculate_l_k(k);
            if l_k > 0.0 {
                k_values.push(k as f64);
                l_k_values.push(l_k.ln());
            }
        }
        
        if k_values.len() >= 3 {
            // Линейная регрессия ln(L(k)) vs ln(k)
            let slope = self.linear_regression_slope(&k_values.iter().map(|&x| x.ln()).collect::<Vec<_>>(), &l_k_values);
            
            // Фрактальная размерность = 2 - slope
            self.fractal_dimension = (2.0 - slope).clamp(1.0, 2.0);
            
            // Complexity score: чем ближе к 2.0, тем сложнее
            self.complexity_score = (self.fractal_dimension - 1.0) / 1.0;
        }
    }
    
    /// Рассчитать L(k) для данного k
    fn calculate_l_k(&self, k: usize) -> f64 {
        let n = self.prices.len();
        if k >= n {
            return 0.0;
        }
        
        let mut total_length = 0.0;
        let m = (n - 1) / k;
        
        // Для каждой подпоследовательности i
        for i in 1..=k {
            let mut length = 0.0;
            let mut count = 0;
            
            // Рассчитываем длину кривой для подпоследовательности
            for j in 1..=m {
                let idx1 = i + (j - 1) * k - 1; // -1 для 0-based индексации
                let idx2 = i + j * k - 1;
                
                if idx1 < n && idx2 < n {
                    length += (self.prices[idx2] - self.prices[idx1]).abs();
                    count += 1;
                }
            }
            
            if count > 0 {
                // Нормализуем длину
                length = length * (n as f64 - 1.0) / (count * k) as f64;
                total_length += length;
            }
        }
        
        total_length / k as f64
    }
    
    /// Линейная регрессия для вычисления наклона
    fn linear_regression_slope(&self, x: &[f64], y: &[f64]) -> f64 {
        if x.len() != y.len() || x.is_empty() {
            return 0.0;
        }
        
        let n = x.len() as f64;
        let sum_x: f64 = x.iter().sum();
        let sum_y: f64 = y.iter().sum();
        let sum_xy: f64 = x.iter().zip(y.iter()).map(|(xi, yi)| xi * yi).sum();
        let sum_x2: f64 = x.iter().map(|xi| xi * xi).sum();
        
        let denominator = n * sum_x2 - sum_x * sum_x;
        if denominator.abs() < 1e-10 {
            return 0.0;
        }
        
        (n * sum_xy - sum_x * sum_y) / denominator
    }
    
    /// Получить текущую фрактальную размерность
    pub fn fractal_dimension(&self) -> f64 {
        self.fractal_dimension
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.fractal_dimension)
    }
    
    /// Получить оценку сложности (0.0-1.0)
    pub fn complexity_score(&self) -> f64 {
        self.complexity_score
    }
    
    /// Определить рыночное состояние на основе фрактальной размерности
    pub fn market_state(&self) -> &'static str {
        match self.fractal_dimension {
            d if d < 1.2 => "Strong Trend",
            d if d < 1.4 => "Weak Trend", 
            d if d < 1.6 => "Random Walk",
            d if d < 1.8 => "Noisy Market",
            _ => "Highly Chaotic",
        }
    }
    
    /// Получить сигнал торговли
    pub fn trading_signal(&self) -> i8 {
        match self.fractal_dimension {
            d if d < 1.3 => 1,  // Тренд - следуем направлению
            d if d > 1.7 => -1, // Хаос - контртренд или ожидание
            _ => 0,             // Нейтрально
        }
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Update with OHLCV bar - uses close price
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> IndicatorValue {
        self.update(close);
        self.value()
    }

    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.prices.clear();
        self.fractal_dimension = 1.5;
        self.complexity_score = 0.5;
        self.is_ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fractal_dimension_creation() {
        let ind = FractalDimension::new(50, 10);
        assert!(!ind.is_ready());
        assert_eq!(ind.fractal_dimension(), 1.5);
    }

    #[test]
    fn test_fractal_dimension_warmup() {
        let mut ind = FractalDimension::new(30, 6);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ind.update(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_fractal_dimension_values_range() {
        let mut ind = FractalDimension::new(30, 6);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let fd = ind.update(price);
            assert!(fd >= 1.0 && fd <= 2.0);
            assert!(ind.complexity_score() >= 0.0 && ind.complexity_score() <= 1.0);
        }
    }

    #[test]
    fn test_fractal_dimension_reset() {
        let mut ind = FractalDimension::new(30, 6);
        for i in 0..40 {
            ind.update(100.0 + i as f64);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.fractal_dimension(), 1.5);
    }
} 






















