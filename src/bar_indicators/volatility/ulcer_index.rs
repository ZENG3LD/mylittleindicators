//! Ulcer Index - индикатор язвенного индекса Питера Мартина
//! Ulcer Index = sqrt(Σ(R²) / n), где R = (Close - Highest High over n periods) / Highest High * 100
//! Измеряет глубину и продолжительность просадок цены
//! Чем выше значение, тем больше стресс от просадок

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Ulcer Index индикатор
#[derive(Clone)]
pub struct UlcerIndex {
    period: usize,
    
    // Буферы для расчетов
    close_prices: ArrayVec<f64, 512>,
    r_squared_values: ArrayVec<f64, 512>,
    ulcer_values: ArrayVec<f64, 512>,
    
    // Текущие значения
    ulcer_index_value: f64,
    
    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl UlcerIndex {
    /// Создать новый Ulcer Index с периодом по умолчанию (14)
    pub fn new() -> Self {
        Self::with_period(14)
    }
    
    /// Создать новый Ulcer Index с заданным периодом
    pub fn with_period(period: usize) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        
        Self {
            period,
            close_prices: ArrayVec::new(),
            r_squared_values: ArrayVec::new(),
            ulcer_values: ArrayVec::new(),
            ulcer_index_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> f64 {
        self.bars_count += 1;
        
        // Добавляем цену закрытия в буфер
        if self.close_prices.len() >= 512 {
            self.close_prices.remove(0);
        }
        self.close_prices.push(close);
        
        // Проверяем, можем ли рассчитать Ulcer Index
        if self.close_prices.len() >= self.period {
            // Находим максимальную цену за период
            let start_idx = self.close_prices.len() - self.period;
            let highest_high = self.close_prices[start_idx..].iter()
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            
            // Рассчитываем R (процент просадки от максимума)
            let r = if highest_high > 1e-12 {
                (close - highest_high) / highest_high * 100.0
            } else {
                0.0
            };
            
            // R² (квадрат просадки)
            let r_squared = r * r;
            
            // Добавляем в буфер R²
            if self.r_squared_values.len() >= self.period {
                self.r_squared_values.remove(0);
            }
            self.r_squared_values.push(r_squared);
            
            // Рассчитываем Ulcer Index
            if self.r_squared_values.len() == self.period {
                let sum_r_squared: f64 = self.r_squared_values.iter().sum();
                self.ulcer_index_value = (sum_r_squared / self.period as f64).sqrt();
            }
        }
        
        // Добавляем в буфер значений Ulcer Index
        if self.ulcer_values.len() >= 512 {
            self.ulcer_values.remove(0);
        }
        self.ulcer_values.push(self.ulcer_index_value);
        
        // Проверяем готовность
        if self.bars_count >= self.period + 5 {
            self.is_ready = true;
        }
        
        self.ulcer_index_value
    }
    
    /// Получить значение Ulcer Index
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.ulcer_index_value)
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить период индикатора
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.close_prices.clear();
        self.r_squared_values.clear();
        self.ulcer_values.clear();
        self.ulcer_index_value = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
    
    /// Определить уровень стресса от просадок
    pub fn stress_level(&self) -> &'static str {
        match self.ulcer_index_value {
            v if v > 10.0 => "Extreme Stress",
            v if v > 7.0 => "High Stress",
            v if v > 5.0 => "Moderate Stress",
            v if v > 3.0 => "Low Stress",
            _ => "Minimal Stress"
        }
    }
    
    /// Получить сигнал риска
    /// 1 = высокий риск, 0 = нормальный риск, -1 = низкий риск
    pub fn risk_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        match self.ulcer_index_value {
            v if v > 8.0 => 1,   // Высокий риск
            v if v < 2.0 => -1,  // Низкий риск
            _ => 0               // Нормальный риск
        }
    }
    
    /// Получить продвинутый сигнал с учетом тренда
    pub fn advanced_risk_signal(&self) -> i8 {
        if !self.is_ready() || self.ulcer_values.len() < 3 {
            return 0;
        }
        
        let len = self.ulcer_values.len();
        let current = self.ulcer_index_value;
        let prev_1 = if len >= 2 { self.ulcer_values[len - 2] } else { 0.0 };
        let prev_2 = if len >= 3 { self.ulcer_values[len - 3] } else { 0.0 };
        
        // Растущий стресс
        if current > prev_1 && prev_1 > prev_2 && current > 5.0 {
            return 1;  // Увеличивающийся риск
        }
        
        // Падающий стресс
        if current < prev_1 && prev_1 < prev_2 && current < 3.0 {
            return -1; // Уменьшающийся риск
        }
        
        0
    }
    
    /// Получить текущую просадку от максимума
    pub fn current_drawdown(&self) -> f64 {
        if !self.is_ready() || self.close_prices.len() < self.period {
            return 0.0;
        }
        
        let current_close = *self.close_prices.last().unwrap();
        let start_idx = self.close_prices.len() - self.period;
        let highest_high = self.close_prices[start_idx..].iter()
            .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        
        if highest_high > 1e-12 {
            (current_close - highest_high) / highest_high * 100.0
        } else {
            0.0
        }
    }
    
    /// Получить максимальную просадку за период
    pub fn max_drawdown(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.close_prices.len() < periods {
            return 0.0;
        }
        
        let start_idx = self.close_prices.len() - periods;
        let slice = &self.close_prices[start_idx..];
        
        let mut max_drawdown = 0.0;
        let mut peak = slice[0];
        
        for &price in slice.iter().skip(1) {
            if price > peak {
                peak = price;
            }
            
            let drawdown = (price - peak) / peak * 100.0;
            if drawdown < max_drawdown {
                max_drawdown = drawdown;
            }
        }
        
        max_drawdown
    }
    
    /// Получить среднюю просадку за период
    pub fn average_drawdown(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.close_prices.len() < periods {
            return 0.0;
        }
        
        let start_idx = self.close_prices.len() - periods;
        let slice = &self.close_prices[start_idx..];
        
        let mut total_drawdown = 0.0;
        let mut count = 0;
        let mut peak = slice[0];
        
        for &price in slice.iter().skip(1) {
            if price > peak {
                peak = price;
            }
            
            let drawdown = (price - peak) / peak * 100.0;
            if drawdown < 0.0 {
                total_drawdown += drawdown.abs();
                count += 1;
            }
        }
        
        if count > 0 {
            total_drawdown / count as f64
        } else {
            0.0
        }
    }
    
    /// Получить тренд Ulcer Index
    pub fn trend_direction(&self, lookback: usize) -> i8 {
        if !self.is_ready() || self.ulcer_values.len() < lookback + 1 {
            return 0;
        }
        
        let current = self.ulcer_index_value;
        let past = self.ulcer_values[self.ulcer_values.len() - lookback - 1];
        
        if current > past {
            1  // Растущий стресс
        } else if current < past {
            -1 // Падающий стресс
        } else {
            0  // Стабильный стресс
        }
    }
    
    /// Получить скорость изменения Ulcer Index
    pub fn rate_of_change(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.ulcer_values.len() < periods + 1 {
            return 0.0;
        }
        
        let current = self.ulcer_index_value;
        let past = self.ulcer_values[self.ulcer_values.len() - periods - 1];
        
        if past.abs() > 1e-12 {
            (current - past) / past * 100.0
        } else {
            0.0
        }
    }
    
    /// Получить волатильность Ulcer Index
    pub fn volatility(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.ulcer_values.len() < periods {
            return 0.0;
        }
        
        let start_idx = self.ulcer_values.len() - periods;
        let slice = &self.ulcer_values[start_idx..];
        
        // Рассчитываем стандартное отклонение
        let mean = slice.iter().sum::<f64>() / slice.len() as f64;
        let variance = slice.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / slice.len() as f64;
        
        variance.sqrt()
    }
    
    /// Получить экстремумы Ulcer Index за период
    pub fn extremes(&self, periods: usize) -> (f64, f64) {
        if !self.is_ready() || self.ulcer_values.len() < periods {
            return (0.0, 0.0);
        }
        
        let start_idx = self.ulcer_values.len() - periods;
        let slice = &self.ulcer_values[start_idx..];
        
        let max_val = slice.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min_val = slice.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        
        (min_val, max_val)
    }
    
    /// Получить нормализованное значение Ulcer Index (от 0 до 1)
    pub fn normalized_value(&self, periods: usize) -> f64 {
        if !self.is_ready() {
            return 0.0;
        }
        
        let (min_val, max_val) = self.extremes(periods);
        let range = max_val - min_val;
        
        if range.abs() < 1e-12 {
            0.0
        } else {
            (self.ulcer_index_value - min_val) / range
        }
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        let current_dd = self.current_drawdown();
        let max_dd = self.max_drawdown(self.period);
        let trend_dir = match self.trend_direction(5) {
            1 => "Rising",
            -1 => "Falling",
            _ => "Stable"
        };

        format!(
            "Ulcer Index: {:.2}, Stress: {}, Current DD: {:.2}%, Max DD: {:.2}%, Trend: {}",
            self.ulcer_index_value,
            self.stress_level(),
            current_dd,
            max_dd,
            trend_dir
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ulcer_index_creation() {
        let ui = UlcerIndex::new();
        assert!(!ui.is_ready());
        assert_eq!(ui.value().main(), 0.0);
        assert_eq!(ui.period(), 14);
    }

    #[test]
    fn test_ulcer_index_with_period() {
        let ui = UlcerIndex::with_period(20);
        assert_eq!(ui.period(), 20);
    }

    #[test]
    fn test_ulcer_index_warmup() {
        let mut ui = UlcerIndex::new();
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ui.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ui.is_ready());
    }

    #[test]
    fn test_ulcer_index_non_negative() {
        let mut ui = UlcerIndex::new();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ui.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "Ulcer Index should be non-negative");
        }
    }

    #[test]
    fn test_ulcer_index_stress_levels() {
        let mut ui = UlcerIndex::new();
        for i in 0..25 {
            ui.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        let stress = ui.stress_level();
        assert!(!stress.is_empty());
    }

    #[test]
    fn test_ulcer_index_reset() {
        let mut ui = UlcerIndex::new();
        for i in 0..25 {
            ui.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        ui.reset();
        assert!(!ui.is_ready());
        assert_eq!(ui.value().main(), 0.0);
    }
} 






















