//! Approximate Entropy (ApEn) - приблизительная энтропия для оценки регулярности временных рядов
//! Измеряет регулярность и предсказуемость в паттернах цен
//! Значения: 0.0-2.0+, где 0.0 = максимально регулярно, выше = менее регулярно

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Approximate Entropy индикатор
#[derive(Clone)]
pub struct ApproximateEntropy {
    period: usize,              // Период анализа
    m: usize,                   // Длина паттерна (обычно 2)
    r: f64,                     // Толерантность (0.1-0.2 * std_dev)
    
    // Буферы
    data: ArrayVec<f64, 512>,   // Буфер цен
    
    // Результаты
    apen: f64,                  // Approximate Entropy
    regularity_score: f64,      // Оценка регулярности (0-1)
    
    // Состояние
    count: usize,
    initialized: bool,
    
    // Кэш для стандартного отклонения
    std_dev: f64,
}

impl ApproximateEntropy {
    pub fn new(period: usize, m: usize, r: f64) -> Self {
        Self {
            period: period.min(512),
            m: m.clamp(1, 5), // Ограничиваем разумными пределами
            r: r.clamp(0.01, 1.0), // Толерантность не может быть слишком маленькой или большой
            data: ArrayVec::new(),
            apen: 0.0,
            regularity_score: 0.5,
            count: 0,
            initialized: false,
            std_dev: 0.0,
        }
    }
    
    /// Создать ApEn с параметрами по умолчанию
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
        
        // Рассчитываем ApEn если достаточно данных
        // Minimum is m+1 for pattern matching, but also respect period
        let min_required = (self.m + 1).max(3).min(self.period);
        if self.data.len() >= min_required {
            self.calculate_std_dev();
            self.calculate_apen();
            self.initialized = true;
        }
        
        self.apen
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
    
    /// Рассчитать приблизительную энтропию
    fn calculate_apen(&mut self) {
        if self.data.len() < self.m + 1 {
            return;
        }
        
        let _n = self.data.len();
        
        // Рассчитываем φ(m) и φ(m+1)
        let phi_m = self.calculate_phi(self.m);
        let phi_m_plus_1 = self.calculate_phi(self.m + 1);
        
        // ApEn = φ(m) - φ(m+1)
        self.apen = phi_m - phi_m_plus_1;
        
        // Нормализуем для получения регулярности (0-1)
        // Максимальное значение ApEn примерно 2.0 для случайных данных
        self.regularity_score = 1.0 - (self.apen / 2.0).clamp(0.0, 1.0);
    }
    
    /// Рассчитать φ(m) для заданной длины паттерна
    fn calculate_phi(&self, pattern_length: usize) -> f64 {
        if pattern_length >= self.data.len() {
            return 0.0;
        }
        
        let n = self.data.len();
        let mut sum = 0.0;
        let mut valid_patterns = 0;
        
        // Для каждого возможного паттерна
        for i in 0..=(n - pattern_length) {
            let mut matches = 0;
            
            // Подсчитываем совпадения с другими паттернами
            for j in 0..=(n - pattern_length) {
                if self.patterns_match(i, j, pattern_length) {
                    matches += 1;
                }
            }
            
            if matches > 0 {
                let probability = matches as f64 / (n - pattern_length + 1) as f64;
                sum += probability.ln();
                valid_patterns += 1;
            }
        }
        
        if valid_patterns > 0 {
            sum / valid_patterns as f64
        } else {
            0.0
        }
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
    
    /// Получить текущую приблизительную энтропию
    pub fn apen(&self) -> f64 {
        self.apen
    }
    
    /// Получить оценку регулярности (0-1, где 1 = максимально регулярно)
    pub fn regularity_score(&self) -> f64 {
        self.regularity_score
    }
    
    /// Определить состояние рынка
    pub fn market_state(&self) -> &'static str {
        match self.apen {
            a if a < 0.3 => "Highly Regular",
            a if a < 0.7 => "Moderately Regular",
            a if a < 1.2 => "Irregular",
            _ => "Highly Irregular",
        }
    }
    
    /// Получить торговый сигнал на основе регулярности
    pub fn trading_signal(&self) -> i8 {
        match self.apen {
            a if a < 0.5 => 1,  // Высокая регулярность - следуем паттерну
            a if a > 1.5 => -1, // Низкая регулярность - контртренд
            _ => 0,             // Нейтрально
        }
    }
    
    /// Получить значение для использования в других индикаторах
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.apen)
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
        self.apen = 0.0;
        self.regularity_score = 0.5;
        self.count = 0;
        self.initialized = false;
        self.std_dev = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approximate_entropy_creation() {
        let apen = ApproximateEntropy::new(50, 2, 0.15);
        assert!(!apen.is_ready());
        assert_eq!(apen.value().main(), 0.0);
        assert_eq!(apen.period(), 50);
        assert_eq!(apen.pattern_length(), 2);
    }

    #[test]
    fn test_approximate_entropy_default() {
        let apen = ApproximateEntropy::new_default(30);
        assert!(!apen.is_ready());
        assert_eq!(apen.period(), 30);
    }

    #[test]
    fn test_approximate_entropy_warmup() {
        let mut apen = ApproximateEntropy::new_default(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            apen.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(apen.is_ready());
    }

    #[test]
    fn test_approximate_entropy_values_finite() {
        let mut apen = ApproximateEntropy::new_default(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = apen.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_approximate_entropy_reset() {
        let mut apen = ApproximateEntropy::new_default(20);
        for i in 0..25 {
            apen.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        apen.reset();
        assert!(!apen.is_ready());
        assert_eq!(apen.value().main(), 0.0);
    }

    #[test]
    fn test_approximate_entropy_regularity_score() {
        let mut apen = ApproximateEntropy::new_default(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            apen.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        let score = apen.regularity_score();
        assert!(score >= 0.0 && score <= 1.0);
    }
} 






















