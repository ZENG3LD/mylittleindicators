//! Choppiness Index - индикатор изменчивости рынка
//! Choppiness Index = 100 * log10(Σ(ATR(1)) / (Highest High - Lowest Low)) / log10(n)
//! где n - период, ATR - Average True Range
//! Значения близкие к 100 указывают на боковой рынок, близкие к 0 - на трендовый

use arrayvec::ArrayVec;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Choppiness Index индикатор
#[derive(Clone)]
pub struct ChoppinessIndex {
    period: usize,
    
    // Буферы для расчетов
    high_prices: ArrayVec<f64, 512>,
    low_prices: ArrayVec<f64, 512>,
    atr_values: ArrayVec<f64, 512>,
    choppiness_values: ArrayVec<f64, 512>,
    
    // ATR для расчета True Range
    atr: Atr,
    
    // Текущие значения
    choppiness_value: f64,
    
    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl ChoppinessIndex {
    /// Создать новый Choppiness Index с периодом по умолчанию (14)
    pub fn new() -> Self {
        Self::with_period(14)
    }
    
    /// Создать новый Choppiness Index с заданным периодом
    pub fn with_period(period: usize) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        
        Self {
            period,
            high_prices: ArrayVec::new(),
            low_prices: ArrayVec::new(),
            atr_values: ArrayVec::new(),
            choppiness_values: ArrayVec::new(),
            atr: Atr::new(1, MovingAverageType::RMA), // ATR с периодом 1 для получения True Range
            choppiness_value: 50.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        self.bars_count += 1;
        
        // Добавляем цены в буферы
        if self.high_prices.len() >= 512 {
            self.high_prices.remove(0);
        }
        if self.low_prices.len() >= 512 {
            self.low_prices.remove(0);
        }
        
        self.high_prices.push(high);
        self.low_prices.push(low);
        
        // Получаем ATR (True Range для текущего бара)
        let atr_value = self.atr.update_bar(open, high, low, close, volume);
        
        // Добавляем ATR в буфер
        if self.atr_values.len() >= 512 {
            self.atr_values.remove(0);
        }
        self.atr_values.push(atr_value);
        
        // Проверяем, можем ли рассчитать Choppiness Index
        if self.high_prices.len() >= self.period && self.low_prices.len() >= self.period && self.atr_values.len() >= self.period {
            // Находим максимум и минимум за период
            let start_idx = self.high_prices.len() - self.period;
            let highest_high = self.high_prices[start_idx..].iter()
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let lowest_low = self.low_prices[start_idx..].iter()
                .fold(f64::INFINITY, |a, &b| a.min(b));
            
            // Сумма ATR за период
            let atr_start_idx = self.atr_values.len() - self.period;
            let atr_sum: f64 = self.atr_values[atr_start_idx..].iter().sum();
            
            // Рассчитываем Choppiness Index
            let range = highest_high - lowest_low;
            if range > 1e-12 && atr_sum > 1e-12 {
                let ratio = atr_sum / range;
                let log_ratio = ratio.log10();
                let log_period = (self.period as f64).log10();
                
                if log_period.abs() > 1e-12 {
                    self.choppiness_value = 100.0 * log_ratio / log_period;
                    // Ограничиваем значение от 0 до 100
                    self.choppiness_value = self.choppiness_value.clamp(0.0, 100.0);
                }
            }
        }
        
        // Добавляем в буфер значений
        if self.choppiness_values.len() >= 512 {
            self.choppiness_values.remove(0);
        }
        self.choppiness_values.push(self.choppiness_value);
        
        // Проверяем готовность
        if self.bars_count >= self.period + 5 {
            self.is_ready = true;
        }
        
        self.choppiness_value
    }
    
    /// Получить значение Choppiness Index
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.choppiness_value)
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
        self.high_prices.clear();
        self.low_prices.clear();
        self.atr_values.clear();
        self.choppiness_values.clear();
        self.atr.reset();
        self.choppiness_value = 50.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
    
    /// Определить состояние рынка
    pub fn market_condition(&self) -> &'static str {
        match self.choppiness_value {
            v if v > 61.8 => "Choppy/Sideways",      // Боковой рынок
            v if v > 50.0 => "Moderately Choppy",    // Умеренно боковой
            v if v > 38.2 => "Moderately Trending",  // Умеренно трендовый
            _ => "Trending"                          // Трендовый рынок
        }
    }
    
    /// Получить торговый сигнал
    /// 1 = трендовый рынок (подходит для трендовых стратегий)
    /// -1 = боковой рынок (подходит для осцилляторных стратегий)
    /// 0 = неопределенное состояние
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        match self.choppiness_value {
            v if v < 38.2 => 1,   // Трендовый рынок
            v if v > 61.8 => -1,  // Боковой рынок
            _ => 0                // Переходное состояние
        }
    }
    
    /// Получить продвинутый сигнал с подтверждением
    pub fn advanced_signal(&self) -> i8 {
        if !self.is_ready() || self.choppiness_values.len() < 3 {
            return 0;
        }
        
        let len = self.choppiness_values.len();
        let current = self.choppiness_value;
        let prev_1 = if len >= 2 { self.choppiness_values[len - 2] } else { 50.0 };
        let prev_2 = if len >= 3 { self.choppiness_values[len - 3] } else { 50.0 };
        
        // Переход к трендовому рынку
        if prev_2 > 50.0 && prev_1 > 40.0 && current < 38.2 {
            return 1;
        }
        
        // Переход к боковому рынку
        if prev_2 < 50.0 && prev_1 < 60.0 && current > 61.8 {
            return -1;
        }
        
        0
    }
    
    /// Получить силу тренда (инвертированная изменчивость)
    pub fn trend_strength(&self) -> f64 {
        if !self.is_ready() {
            return 0.0;
        }
        
        // Чем меньше Choppiness Index, тем сильнее тренд
        100.0 - self.choppiness_value
    }
    
    /// Получить силу боковика
    pub fn sideways_strength(&self) -> f64 {
        if !self.is_ready() {
            return 0.0;
        }
        
        // Чем больше Choppiness Index, тем сильнее боковое движение
        self.choppiness_value
    }
    
    /// Получить нормализованное значение (от 0 до 1)
    pub fn normalized_value(&self) -> f64 {
        if !self.is_ready() {
            return 0.5;
        }
        
        self.choppiness_value / 100.0
    }
    
    /// Получить тренд Choppiness Index
    pub fn trend_direction(&self, lookback: usize) -> i8 {
        if !self.is_ready() || self.choppiness_values.len() < lookback + 1 {
            return 0;
        }
        
        let current = self.choppiness_value;
        let past = self.choppiness_values[self.choppiness_values.len() - lookback - 1];
        
        if current > past {
            1  // Становится более боковым
        } else if current < past {
            -1 // Становится более трендовым
        } else {
            0  // Стабильное состояние
        }
    }
    
    /// Получить скорость изменения Choppiness Index
    pub fn rate_of_change(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.choppiness_values.len() < periods + 1 {
            return 0.0;
        }
        
        let current = self.choppiness_value;
        let past = self.choppiness_values[self.choppiness_values.len() - periods - 1];
        
        if past.abs() > 1e-12 {
            (current - past) / past * 100.0
        } else {
            0.0
        }
    }
    
    /// Получить волатильность Choppiness Index
    pub fn volatility(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.choppiness_values.len() < periods {
            return 0.0;
        }
        
        let start_idx = self.choppiness_values.len() - periods;
        let slice = &self.choppiness_values[start_idx..];
        
        // Рассчитываем стандартное отклонение
        let mean = slice.iter().sum::<f64>() / slice.len() as f64;
        let variance = slice.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / slice.len() as f64;
        
        variance.sqrt()
    }
    
    /// Получить экстремумы Choppiness Index за период
    pub fn extremes(&self, periods: usize) -> (f64, f64) {
        if !self.is_ready() || self.choppiness_values.len() < periods {
            return (0.0, 100.0);
        }
        
        let start_idx = self.choppiness_values.len() - periods;
        let slice = &self.choppiness_values[start_idx..];
        
        let max_val = slice.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min_val = slice.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        
        (min_val, max_val)
    }
    
    /// Получить среднее значение Choppiness Index за период
    pub fn average_value(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.choppiness_values.len() < periods {
            return 50.0;
        }
        
        let start_idx = self.choppiness_values.len() - periods;
        let slice = &self.choppiness_values[start_idx..];
        
        slice.iter().sum::<f64>() / slice.len() as f64
    }
    
    /// Определить рыночный режим для выбора стратегии
    pub fn market_regime(&self) -> &'static str {
        match self.choppiness_value {
            v if v < 25.0 => "Strong Trend - Use Trend Following",
            v if v < 38.2 => "Trend - Use Trend Following",
            v if v < 50.0 => "Weak Trend - Use Hybrid Strategies",
            v if v < 61.8 => "Weak Chop - Use Hybrid Strategies",
            v if v < 75.0 => "Chop - Use Mean Reversion",
            _ => "Strong Chop - Use Mean Reversion"
        }
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        let trend_str = self.trend_strength();
        let sideways_str = self.sideways_strength();
        let trend_dir = match self.trend_direction(5) {
            1 => "More Choppy",
            -1 => "More Trending",
            _ => "Stable"
        };

        format!(
            "Choppiness Index: {:.1}, Condition: {}, Trend Strength: {:.1}, Sideways Strength: {:.1}, Direction: {}",
            self.choppiness_value,
            self.market_condition(),
            trend_str,
            sideways_str,
            trend_dir
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_choppiness_index_creation() {
        let ci = ChoppinessIndex::new();
        assert!(!ci.is_ready());
        assert_eq!(ci.value().main(), 50.0);
    }

    #[test]
    fn test_choppiness_index_warmup() {
        let mut ci = ChoppinessIndex::with_period(14);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ci.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ci.is_ready());
    }

    #[test]
    fn test_choppiness_index_range() {
        let mut ci = ChoppinessIndex::with_period(14);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let value = ci.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 100.0, "Choppiness should be in [0, 100]");
        }
    }

    #[test]
    fn test_choppiness_index_market_condition() {
        let mut ci = ChoppinessIndex::with_period(14);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            ci.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        let condition = ci.market_condition();
        assert!(!condition.is_empty());
    }

    #[test]
    fn test_choppiness_index_reset() {
        let mut ci = ChoppinessIndex::with_period(14);
        for i in 0..25 {
            ci.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        ci.reset();
        assert!(!ci.is_ready());
        assert_eq!(ci.value().main(), 50.0);
    }
} 






















