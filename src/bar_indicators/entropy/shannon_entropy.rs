//! Shannon Entropy - энтропия Шеннона для оценки предсказуемости рынка
//! Измеряет неопределенность в распределении доходностей
//! Значения: 0.0-1.0 (нормализованная), где 0.0 = полностью предсказуемо, 1.0 = максимально случайно

use arrayvec::ArrayVec;
use std::collections::HashMap;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Состояние рынка на основе энтропии Шеннона
#[derive(Debug, Clone, PartialEq)]
pub enum MarketEntropyState {
    HighlyPredictable,  // entropy < 0.3
    Moderate,           // 0.3 <= entropy < 0.7
    Random,             // 0.7 <= entropy < 0.9
    Chaotic,            // entropy >= 0.9
}

/// Shannon Entropy индикатор
#[derive(Clone)]
pub struct ShannonEntropy {
    period: usize,              // Период анализа
    bins: usize,                // Количество корзин для гистограммы
    
    // Буферы данных
    returns: ArrayVec<f64, 512>, // Буфер логарифмических доходностей
    prev_close: Option<f64>,     // Предыдущая цена для расчета доходности
    
    // Результаты
    entropy: f64,               // Энтропия Шеннона
    normalized_entropy: f64,    // Нормализованная энтропия (0-1)
    predictability_score: f64,  // 1 - normalized_entropy
    
    // Состояние
    count: usize,
    initialized: bool,
}

impl ShannonEntropy {
    pub fn new(period: usize, bins: usize) -> Self {
        Self {
            period: period.min(512),
            bins: bins.clamp(5, 50), // Ограничиваем разумными пределами
            returns: ArrayVec::new(),
            prev_close: None,
            entropy: 0.0,
            normalized_entropy: 0.0,
            predictability_score: 1.0,
            count: 0,
            initialized: false,
        }
    }
    
    /// Создать Shannon Entropy с параметрами по умолчанию
    pub fn new_default(period: usize) -> Self {
        Self::new(period, 20) // 20 bins по умолчанию
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> f64 {
        // Рассчитываем логарифмическую доходность
        if let Some(prev_close) = self.prev_close {
            let log_return = (close / prev_close).ln();
            
            // Фильтруем экстремальные значения
            if log_return.is_finite() && log_return.abs() < 1.0 {
                // Добавляем в буфер
                if self.returns.len() >= self.period {
                    self.returns.remove(0);
                }
                self.returns.push(log_return);
                self.count += 1;
                
                // Рассчитываем энтропию если достаточно данных
                if self.returns.len() >= self.period.min(10) {
                    self.calculate_entropy();
                    self.initialized = true;
                }
            }
        }
        
        self.prev_close = Some(close);
        self.normalized_entropy
    }
    
    /// Рассчитать энтропию Шеннона
    fn calculate_entropy(&mut self) {
        if self.returns.is_empty() {
            return;
        }
        
        // Находим диапазон доходностей
        let min_return = self.returns.iter().copied().fold(f64::INFINITY, f64::min);
        let max_return = self.returns.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        
        if (max_return - min_return).abs() < 1e-10 {
            // Все доходности одинаковые - минимальная энтропия
            self.entropy = 0.0;
            self.normalized_entropy = 0.0;
            self.predictability_score = 1.0;
            return;
        }
        
        // Создаем гистограмму
        let bin_width = (max_return - min_return) / self.bins as f64;
        let mut histogram = HashMap::new();
        
        for &return_val in &self.returns {
            let bin_index = ((return_val - min_return) / bin_width).floor() as usize;
            let bin_index = bin_index.min(self.bins - 1);
            *histogram.entry(bin_index).or_insert(0) += 1;
        }
        
        // Рассчитываем энтропию Шеннона: H = -Σ p(xi) * log2(p(xi))
        let total_count = self.returns.len() as f64;
        let mut entropy = 0.0;
        
        for &count in histogram.values() {
            if count > 0 {
                let probability = count as f64 / total_count;
                entropy -= probability * probability.log2();
            }
        }
        
        // Нормализуем энтропию (максимальная энтропия = log2(bins))
        let max_entropy = (self.bins as f64).log2();
        
        self.entropy = entropy;
        self.normalized_entropy = if max_entropy > 0.0 {
            (entropy / max_entropy).clamp(0.0, 1.0)
        } else {
            0.0
        };
        self.predictability_score = 1.0 - self.normalized_entropy;
    }
    
    /// Получить текущую энтропию Шеннона
    pub fn entropy(&self) -> f64 {
        self.entropy
    }
    
    /// Получить нормализованную энтропию (0-1)
    pub fn normalized_entropy(&self) -> f64 {
        self.normalized_entropy
    }
    
    /// Получить оценку предсказуемости (0-1, где 1 = максимально предсказуемо)
    pub fn predictability_score(&self) -> f64 {
        self.predictability_score
    }
    
    /// Определить состояние рынка
    pub fn market_state(&self) -> MarketEntropyState {
        match self.normalized_entropy {
            e if e < 0.3 => MarketEntropyState::HighlyPredictable,
            e if e < 0.7 => MarketEntropyState::Moderate,
            e if e < 0.9 => MarketEntropyState::Random,
            _ => MarketEntropyState::Chaotic,
        }
    }
    
    /// Получить торговый сигнал на основе энтропии
    pub fn trading_signal(&self) -> i8 {
        match self.market_state() {
            MarketEntropyState::HighlyPredictable => 1,  // Высокая предсказуемость - следуем тренду
            MarketEntropyState::Moderate => 0,           // Умеренная - нейтрально
            MarketEntropyState::Random => -1,            // Случайность - контртренд
            MarketEntropyState::Chaotic => 0,            // Хаос - ждем
        }
    }
    
    /// Получить значение для использования в других индикаторах
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.normalized_entropy)
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Получить количество корзин
    pub fn bins(&self) -> usize {
        self.bins
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.returns.clear();
        self.prev_close = None;
        self.entropy = 0.0;
        self.normalized_entropy = 0.0;
        self.predictability_score = 1.0;
        self.count = 0;
        self.initialized = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shannon_entropy_creation() {
        let se = ShannonEntropy::new(50, 20);
        assert!(!se.is_ready());
        assert_eq!(se.value().main(), 0.0);
        assert_eq!(se.period(), 50);
        assert_eq!(se.bins(), 20);
    }

    #[test]
    fn test_shannon_entropy_default() {
        let se = ShannonEntropy::new_default(30);
        assert!(!se.is_ready());
        assert_eq!(se.period(), 30);
        assert_eq!(se.bins(), 20);
    }

    #[test]
    fn test_shannon_entropy_warmup() {
        let mut se = ShannonEntropy::new_default(15);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            se.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(se.is_ready());
    }

    #[test]
    fn test_shannon_entropy_values_range() {
        let mut se = ShannonEntropy::new_default(15);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = se.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 1.0);
        }
    }

    #[test]
    fn test_shannon_entropy_reset() {
        let mut se = ShannonEntropy::new_default(15);
        for i in 0..25 {
            se.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        se.reset();
        assert!(!se.is_ready());
        assert_eq!(se.value().main(), 0.0);
    }

    #[test]
    fn test_shannon_entropy_predictability() {
        let mut se = ShannonEntropy::new_default(15);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            se.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        let score = se.predictability_score();
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_shannon_entropy_market_state() {
        let mut se = ShannonEntropy::new_default(15);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            se.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        let state = se.market_state();
        assert!(matches!(state, MarketEntropyState::HighlyPredictable | MarketEntropyState::Moderate | MarketEntropyState::Random | MarketEntropyState::Chaotic));
    }
} 






















