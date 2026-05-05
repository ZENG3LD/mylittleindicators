//! Mass Index - индикатор массового индекса Дональда Дорси
//! Mass Index = Сумма(EMA(High-Low, 9) / EMA(EMA(High-Low, 9), 9)) за 25 периодов
//! Используется для определения потенциальных разворотов тренда
//! Значения выше 27 указывают на возможный разворот, ниже 26.5 - на продолжение тренда

use arrayvec::ArrayVec;
use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Mass Index индикатор
#[derive(Clone)]
pub struct MassIndex {
    ema_period: usize,
    sum_period: usize,
    
    // Буферы для расчетов
    high_low_values: ArrayVec<f64, 512>,
    mass_ratio_values: ArrayVec<f64, 512>,
    mass_index_values: ArrayVec<f64, 512>,
    
    // EMA для первого и второго сглаживания
    first_ema: MovingAverageProvider,
    second_ema: MovingAverageProvider,
    
    // Текущие значения
    mass_index_value: f64,
    
    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl MassIndex {
    /// Создать новый Mass Index с параметрами по умолчанию (9, 25)
    pub fn new() -> Self {
        Self::with_params(9, 25)
    }
    
    /// Создать новый Mass Index с настраиваемыми параметрами
    pub fn with_params(ema_period: usize, sum_period: usize) -> Self {
        assert!(ema_period > 0, "EMA period must be greater than 0");
        assert!(sum_period > 0, "Sum period must be greater than 0");
        
        Self {
            ema_period,
            sum_period,
            high_low_values: ArrayVec::new(),
            mass_ratio_values: ArrayVec::new(),
            mass_index_values: ArrayVec::new(),
            first_ema: MovingAverageProvider::new(MovingAverageType::EMA, ema_period),
            second_ema: MovingAverageProvider::new(MovingAverageType::EMA, ema_period),
            mass_index_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, _close: f64, _volume: f64) -> f64 {
        self.bars_count += 1;
        
        // Рассчитываем High - Low
        let high_low = high - low;
        
        // Добавляем в буфер
        if self.high_low_values.len() >= 512 {
            self.high_low_values.remove(0);
        }
        self.high_low_values.push(high_low);
        
        // Первое сглаживание EMA(High-Low)
        let first_ema_value = self.first_ema.update_bar(high_low, high_low, high_low, high_low, 1.0);
        
        // Второе сглаживание EMA(EMA(High-Low))
        let second_ema_value = self.second_ema.update_bar(first_ema_value, first_ema_value, first_ema_value, first_ema_value, 1.0);
        
        // Рассчитываем отношение
        let mass_ratio = if second_ema_value.abs() > 1e-12 {
            first_ema_value / second_ema_value
        } else {
            1.0
        };
        
        // Добавляем в буфер отношений
        if self.mass_ratio_values.len() >= 512 {
            self.mass_ratio_values.remove(0);
        }
        self.mass_ratio_values.push(mass_ratio);
        
        // Рассчитываем Mass Index как сумму отношений за период
        if self.mass_ratio_values.len() >= self.sum_period {
            let start_idx = self.mass_ratio_values.len() - self.sum_period;
            self.mass_index_value = self.mass_ratio_values[start_idx..].iter().sum();
        }
        
        // Добавляем в буфер значений Mass Index
        if self.mass_index_values.len() >= 512 {
            self.mass_index_values.remove(0);
        }
        self.mass_index_values.push(self.mass_index_value);
        
        // Проверяем готовность
        if self.bars_count >= self.ema_period * 2 + self.sum_period {
            self.is_ready = true;
        }
        
        self.mass_index_value
    }
    
    /// Получить значение Mass Index
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.mass_index_value)
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить параметры индикатора
    pub fn parameters(&self) -> (usize, usize) {
        (self.ema_period, self.sum_period)
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.high_low_values.clear();
        self.mass_ratio_values.clear();
        self.mass_index_values.clear();
        self.first_ema.reset();
        self.second_ema.reset();
        self.mass_index_value = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
    
    /// Определить состояние рынка
    pub fn market_condition(&self) -> &'static str {
        match self.mass_index_value {
            v if v > 27.0 => "Potential Reversal Zone",
            v if v > 26.5 => "High Volatility",
            v if v < 26.5 => "Trend Continuation",
            _ => "Normal"
        }
    }
    
    /// Получить сигнал разворота
    /// 1 = потенциальный разворот вверх, -1 = потенциальный разворот вниз, 0 = нет сигнала
    pub fn reversal_signal(&self) -> i8 {
        if !self.is_ready() || self.mass_index_values.len() < 3 {
            return 0;
        }
        
        let len = self.mass_index_values.len();
        let current = self.mass_index_value;
        let prev_1 = if len >= 2 { self.mass_index_values[len - 2] } else { 0.0 };
        let prev_2 = if len >= 3 { self.mass_index_values[len - 3] } else { 0.0 };
        
        // Сигнал разворота: Mass Index поднимается выше 27 и затем опускается ниже 26.5
        if prev_2 <= 27.0 && prev_1 > 27.0 && current < 26.5 {
            // Направление зависит от предыдущего тренда
            // Для простоты возвращаем общий сигнал разворота
            1
        } else {
            0
        }
    }
    
    /// Получить продвинутый сигнал разворота с дополнительными условиями
    pub fn advanced_reversal_signal(&self, price_trend: i8) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        let basic_signal = self.reversal_signal();
        
        if basic_signal != 0 {
            // Если есть базовый сигнал, учитываем направление предыдущего тренда
            match price_trend {
                1 => -1,  // Восходящий тренд -> сигнал разворота вниз
                -1 => 1,  // Нисходящий тренд -> сигнал разворота вверх
                _ => 0    // Неопределенный тренд -> нет сигнала
            }
        } else {
            0
        }
    }
    
    /// Получить уровень волатильности
    pub fn volatility_level(&self) -> &'static str {
        match self.mass_index_value {
            v if v > 28.0 => "Extremely High",
            v if v > 27.0 => "Very High",
            v if v > 26.0 => "High",
            v if v > 25.0 => "Moderate",
            _ => "Low"
        }
    }
    
    /// Получить силу сигнала разворота (от 0 до 100)
    pub fn reversal_strength(&self) -> f64 {
        if !self.is_ready() {
            return 0.0;
        }
        
        // Сила зависит от того, насколько высоко поднялся Mass Index
        let max_value = if self.mass_index_values.len() >= 10 {
            let start_idx = self.mass_index_values.len() - 10;
            self.mass_index_values[start_idx..].iter()
                .fold(f64::NEG_INFINITY, |a, &b| a.max(b))
        } else {
            self.mass_index_value
        };
        
        if max_value > 27.0 {
            ((max_value - 27.0) / 3.0 * 100.0).min(100.0)
        } else {
            0.0
        }
    }
    
    /// Получить тренд Mass Index
    pub fn trend_direction(&self, lookback: usize) -> i8 {
        if !self.is_ready() || self.mass_index_values.len() < lookback + 1 {
            return 0;
        }
        
        let current = self.mass_index_value;
        let past = self.mass_index_values[self.mass_index_values.len() - lookback - 1];
        
        if current > past {
            1  // Растущая волатильность
        } else if current < past {
            -1 // Падающая волатильность
        } else {
            0  // Стабильная волатильность
        }
    }
    
    /// Получить скорость изменения Mass Index
    pub fn rate_of_change(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.mass_index_values.len() < periods + 1 {
            return 0.0;
        }
        
        let current = self.mass_index_value;
        let past = self.mass_index_values[self.mass_index_values.len() - periods - 1];
        
        if past.abs() > 1e-12 {
            (current - past) / past * 100.0
        } else {
            0.0
        }
    }
    
    /// Получить среднее значение Mass Index за период
    pub fn average_value(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.mass_index_values.len() < periods {
            return 0.0;
        }
        
        let start_idx = self.mass_index_values.len() - periods;
        let slice = &self.mass_index_values[start_idx..];
        
        slice.iter().sum::<f64>() / slice.len() as f64
    }
    
    /// Получить экстремумы Mass Index за период
    pub fn extremes(&self, periods: usize) -> (f64, f64) {
        if !self.is_ready() || self.mass_index_values.len() < periods {
            return (0.0, 0.0);
        }
        
        let start_idx = self.mass_index_values.len() - periods;
        let slice = &self.mass_index_values[start_idx..];
        
        let max_val = slice.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min_val = slice.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        
        (min_val, max_val)
    }
    
    /// Проверить, находится ли Mass Index в зоне разворота
    pub fn in_reversal_zone(&self) -> bool {
        self.is_ready() && self.mass_index_value > 27.0
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        let strength = self.reversal_strength();
        let trend_dir = match self.trend_direction(5) {
            1 => "Rising",
            -1 => "Falling",
            _ => "Stable"
        };

        format!(
            "Mass Index: {:.2}, Condition: {}, Volatility: {}, Strength: {:.1}%, Trend: {}",
            self.mass_index_value,
            self.market_condition(),
            self.volatility_level(),
            strength,
            trend_dir
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mass_index_creation() {
        let mi = MassIndex::new();
        assert!(!mi.is_ready());
        assert_eq!(mi.value().main(), 0.0);
    }

    #[test]
    fn test_mass_index_warmup() {
        let mut mi = MassIndex::with_params(9, 25);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            mi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(mi.is_ready());
    }

    #[test]
    fn test_mass_index_values() {
        let mut mi = MassIndex::new();
        for i in 0..50 {
            let price = 100.0 + i as f64;
            let value = mi.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_mass_index_market_condition() {
        let mut mi = MassIndex::new();
        for i in 0..50 {
            let price = 100.0 + i as f64;
            mi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        let condition = mi.market_condition();
        assert!(!condition.is_empty());
    }

    #[test]
    fn test_mass_index_reset() {
        let mut mi = MassIndex::new();
        for i in 0..50 {
            mi.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        mi.reset();
        assert!(!mi.is_ready());
        assert_eq!(mi.value().main(), 0.0);
    }
} 






















