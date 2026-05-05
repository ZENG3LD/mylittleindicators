//! Sample Entropy (SampEn) - выборочная энтропия для оценки сложности временных рядов
//! Усовершенствованная версия Approximate Entropy, не подверженная самосовпадениям
//! Значения: 0.0-3.0+, где 0.0 = максимально регулярно, выше = более сложно

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Sample Entropy индикатор
#[derive(Clone)]
pub struct SampleEntropy {
    period: usize,              // Период анализа
    m: usize,                   // Длина паттерна (обычно 2)
    r: f64,                     // Толерантность
    
    // Буферы
    data: ArrayVec<f64, 512>,   // Буфер цен
    
    // Результаты
    sampen: f64,                // Sample Entropy
    complexity_score: f64,      // Оценка сложности (0-1)
    
    // Состояние
    count: usize,
    initialized: bool,
    
    // Кэш для стандартного отклонения
    std_dev: f64,
}

impl SampleEntropy {
    pub fn new(period: usize, m: usize, r: f64) -> Self {
        Self {
            period: period.min(512),
            m: m.clamp(1, 5), // Ограничиваем разумными пределами
            r: r.clamp(0.01, 1.0), // Толерантность не может быть слишком маленькой или большой
            data: ArrayVec::new(),
            sampen: 0.0,
            complexity_score: 0.5,
            count: 0,
            initialized: false,
            std_dev: 0.0,
        }
    }
    
    /// Создать SampEn с параметрами по умолчанию
    pub fn new_default(period: usize) -> Self {
        Self::new(period, 2, 0.0) // r будет вычислено автоматически
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> f64 {
        // Добавляем цену в буфер
        if self.data.len() >= self.period {
            self.data.remove(0);
        }
        self.data.push(close);
        self.count += 1;
        
        // Рассчитываем SampEn если достаточно данных
        // Minimum is m+1 for pattern matching, but also respect period
        let min_required = (self.m + 1).max(3).min(self.period);
        if self.data.len() >= min_required {
            self.calculate_std_dev();
            self.calculate_sampen();
            self.initialized = true;
        }
        
        self.sampen
    }
    
    /// Рассчитать стандартное отклонение для автоматической настройки r
    fn calculate_std_dev(&mut self) {
        if self.data.len() < 2 {
            return;
        }
        
        let mean = self.data.iter().sum::<f64>() / self.data.len() as f64;
        let variance = self.data.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / self.data.len() as f64;
        
        self.std_dev = variance.sqrt();
        
        // Автоматически устанавливаем r если он не задан
        if self.r < 0.01 {
            self.r = 0.15 * self.std_dev; // 15% от стандартного отклонения
        }
    }
    
    /// Рассчитать выборочную энтропию
    fn calculate_sampen(&mut self) {
        if self.data.len() < self.m + 1 {
            return;
        }
        
        // Подсчитываем совпадения для паттернов длины m и m+1
        let matches_m = self.count_matches(self.m);
        let matches_m_plus_1 = self.count_matches(self.m + 1);
        
        // SampEn = -ln(A/B) где A = matches_m+1, B = matches_m
        if matches_m > 0 && matches_m_plus_1 > 0 {
            self.sampen = -((matches_m_plus_1 as f64) / (matches_m as f64)).ln();
        } else if matches_m > 0 {
            // Если нет совпадений для m+1, но есть для m - высокая энтропия
            self.sampen = 3.0; // Максимальное значение
        } else {
            // Если нет совпадений вообще - средняя энтропия
            self.sampen = 1.5;
        }
        
        // Нормализуем для получения complexity score (0-1)
        // Максимальное значение SampEn примерно 3.0 для случайных данных
        self.complexity_score = (self.sampen / 3.0).clamp(0.0, 1.0);
    }
    
    /// Подсчитать совпадения для паттернов заданной длины
    fn count_matches(&self, pattern_length: usize) -> usize {
        if pattern_length >= self.data.len() {
            return 0;
        }
        
        let n = self.data.len();
        let mut matches = 0;
        
        // Для каждого возможного паттерна
        for i in 0..=(n - pattern_length) {
            // Сравниваем с другими паттернами (исключая самосовпадения)
            for j in 0..=(n - pattern_length) {
                if i != j && self.patterns_match(i, j, pattern_length) {
                    matches += 1;
                }
            }
        }
        
        matches
    }
    
    /// Проверить совпадение паттернов с учетом толерантности r
    fn patterns_match(&self, i: usize, j: usize, length: usize) -> bool {
        for k in 0..length {
            if i + k >= self.data.len() || j + k >= self.data.len() {
                return false;
            }
            
            if (self.data[i + k] - self.data[j + k]).abs() > self.r {
                return false;
            }
        }
        true
    }
    
    /// Получить текущую выборочную энтропию
    pub fn sampen(&self) -> f64 {
        self.sampen
    }
    
    /// Получить оценку сложности (0-1, где 1 = максимально сложно)
    pub fn complexity_score(&self) -> f64 {
        self.complexity_score
    }
    
    /// Определить состояние рынка
    pub fn market_state(&self) -> &'static str {
        match self.sampen {
            s if s < 0.5 => "Highly Predictable",
            s if s < 1.0 => "Moderately Complex",
            s if s < 2.0 => "Complex",
            _ => "Highly Complex",
        }
    }
    
    /// Получить торговый сигнал на основе сложности
    pub fn trading_signal(&self) -> i8 {
        match self.sampen {
            s if s < 0.3 => 1,  // Низкая сложность - следуем паттерну
            s if s > 2.0 => -1, // Высокая сложность - контртренд
            _ => 0,             // Нейтрально
        }
    }
    
    /// Получить значение для использования в других индикаторах
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.sampen)
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Получить длину паттерна
    pub fn pattern_length(&self) -> usize {
        self.m
    }
    
    /// Получить толерантность
    pub fn tolerance(&self) -> f64 {
        self.r
    }
    
    /// Получить стандартное отклонение
    pub fn std_dev(&self) -> f64 {
        self.std_dev
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.data.clear();
        self.sampen = 0.0;
        self.complexity_score = 0.5;
        self.count = 0;
        self.initialized = false;
        self.std_dev = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_entropy_creation() {
        let sampen = SampleEntropy::new(50, 2, 0.15);
        assert!(!sampen.is_ready());
        assert_eq!(sampen.value().main(), 0.0);
        assert_eq!(sampen.period(), 50);
        assert_eq!(sampen.pattern_length(), 2);
    }

    #[test]
    fn test_sample_entropy_default() {
        let sampen = SampleEntropy::new_default(30);
        assert!(!sampen.is_ready());
        assert_eq!(sampen.period(), 30);
    }

    #[test]
    fn test_sample_entropy_warmup() {
        let mut sampen = SampleEntropy::new_default(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sampen.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sampen.is_ready());
    }

    #[test]
    fn test_sample_entropy_values_finite() {
        let mut sampen = SampleEntropy::new_default(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sampen.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_sample_entropy_reset() {
        let mut sampen = SampleEntropy::new_default(20);
        for i in 0..25 {
            sampen.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        sampen.reset();
        assert!(!sampen.is_ready());
        assert_eq!(sampen.value().main(), 0.0);
    }

    #[test]
    fn test_sample_entropy_complexity_score() {
        let mut sampen = SampleEntropy::new_default(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sampen.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        let score = sampen.complexity_score();
        assert!(score >= 0.0 && score <= 1.0);
    }
} 






















