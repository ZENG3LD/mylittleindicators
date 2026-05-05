//! Supertrend - популярный трендовый индикатор
//! Supertrend = (High + Low) / 2 ± (Multiplier × ATR)
//! Показывает динамические уровни поддержки и сопротивления
//! Очень популярен среди трейдеров для определения направления тренда

use arrayvec::ArrayVec;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Supertrend индикатор
#[derive(Debug, Clone)]
pub struct Supertrend {
    period: usize,
    multiplier: f64,
    
    // Буферы для расчетов
    supertrend_values: ArrayVec<f64, 512>,
    trend_direction: ArrayVec<i8, 512>,
    
    // ATR для расчета волатильности
    atr: Atr,
    
    // Текущие значения
    supertrend_value: f64,
    current_trend: i8,  // 1 = восходящий тренд, -1 = нисходящий тренд
    
    // Предыдущие значения для расчета
    prev_supertrend: f64,
    prev_trend: i8,
    
    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl Supertrend {
    /// Создать новый Supertrend с параметрами по умолчанию (10, 3.0)
    pub fn new() -> Self {
        Self::with_params(10, 3.0)
    }
    
    /// Создать новый Supertrend с настраиваемыми параметрами
    pub fn with_params(period: usize, multiplier: f64) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        assert!(multiplier > 0.0, "Multiplier must be greater than 0");
        
        Self {
            period,
            multiplier,
            supertrend_values: ArrayVec::new(),
            trend_direction: ArrayVec::new(),
            atr: Atr::new(period, MovingAverageType::RMA),
            supertrend_value: 0.0,
            current_trend: 1,
            prev_supertrend: 0.0,
            prev_trend: 1,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        self.bars_count += 1;
        
        // Обновляем ATR
        let atr_value = self.atr.update_bar(open, high, low, close, volume);
        
        // Рассчитываем медианную цену (HL2)
        let hl2 = (high + low) / 2.0;
        
        // Рассчитываем базовые уровни
        let upper_band = hl2 + (self.multiplier * atr_value);
        let lower_band = hl2 - (self.multiplier * atr_value);
        
        // Рассчитываем финальные уровни с учетом предыдущих значений
        let final_upper_band = if self.bars_count == 1 {
            upper_band
        } else {
            let prev_final_upper = if !self.supertrend_values.is_empty() && self.prev_trend == 1 {
                self.prev_supertrend
            } else {
                upper_band
            };
            
            if upper_band < prev_final_upper || close > prev_final_upper {
                upper_band
            } else {
                prev_final_upper
            }
        };
        
        let final_lower_band = if self.bars_count == 1 {
            lower_band
        } else {
            let prev_final_lower = if !self.supertrend_values.is_empty() && self.prev_trend == -1 {
                self.prev_supertrend
            } else {
                lower_band
            };
            
            if lower_band > prev_final_lower || close < prev_final_lower {
                lower_band
            } else {
                prev_final_lower
            }
        };
        
        // Определяем направление тренда
        if self.bars_count == 1 {
            // Первый бар - определяем направление по позиции цены
            self.current_trend = if close <= final_upper_band { -1 } else { 1 };
        } else {
            // Определяем смену тренда
            if self.prev_trend == 1 && close <= final_lower_band {
                self.current_trend = -1;
            } else if self.prev_trend == -1 && close >= final_upper_band {
                self.current_trend = 1;
            } else {
                self.current_trend = self.prev_trend;
            }
        }
        
        // Устанавливаем значение Supertrend
        self.supertrend_value = if self.current_trend == 1 {
            final_lower_band
        } else {
            final_upper_band
        };
        
        // Добавляем в буферы
        if self.supertrend_values.len() >= 512 {
            self.supertrend_values.remove(0);
        }
        if self.trend_direction.len() >= 512 {
            self.trend_direction.remove(0);
        }
        
        self.supertrend_values.push(self.supertrend_value);
        self.trend_direction.push(self.current_trend);
        
        // Обновляем предыдущие значения
        self.prev_supertrend = self.supertrend_value;
        self.prev_trend = self.current_trend;
        
        // Проверяем готовность
        if self.bars_count >= self.period + 2 {
            self.is_ready = true;
        }
        
        self.supertrend_value
    }
    
    /// Получить значение Supertrend
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.supertrend_value)
    }
    
    /// Получить направление тренда
    pub fn trend_direction(&self) -> i8 {
        self.current_trend
    }
    
    /// Получить значение и направление тренда
    pub fn values(&self) -> (f64, i8) {
        (self.supertrend_value, self.current_trend)
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить параметры индикатора
    pub fn parameters(&self) -> (usize, f64) {
        (self.period, self.multiplier)
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.supertrend_values.clear();
        self.trend_direction.clear();
        self.atr.reset();
        self.supertrend_value = 0.0;
        self.current_trend = 1;
        self.prev_supertrend = 0.0;
        self.prev_trend = 1;
        self.bars_count = 0;
        self.is_ready = false;
    }
    
    /// Определить состояние тренда
    pub fn trend_condition(&self) -> &'static str {
        match self.current_trend {
            1 => "Uptrend",
            -1 => "Downtrend",
            _ => "Neutral"
        }
    }
    
    /// Получить торговый сигнал
    /// 1 = покупка, -1 = продажа, 0 = нейтрально
    pub fn trading_signal(&self, close: f64) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        // Простой сигнал на основе позиции цены относительно Supertrend
        match self.current_trend {
            1 => {
                if close > self.supertrend_value {
                    1  // Покупка - цена выше Supertrend в восходящем тренде
                } else {
                    0
                }
            },
            -1 => {
                if close < self.supertrend_value {
                    -1 // Продажа - цена ниже Supertrend в нисходящем тренде
                } else {
                    0
                }
            },
            _ => 0
        }
    }
    
    /// Получить сигнал смены тренда
    pub fn trend_change_signal(&self) -> i8 {
        if !self.is_ready() || self.trend_direction.len() < 2 {
            return 0;
        }
        
        let len = self.trend_direction.len();
        let current = self.current_trend;
        let prev = self.trend_direction[len - 2];
        
        // Сигнал смены тренда
        if prev == -1 && current == 1 {
            1  // Смена на восходящий тренд
        } else if prev == 1 && current == -1 {
            -1 // Смена на нисходящий тренд
        } else {
            0  // Нет смены тренда
        }
    }
    
    /// Получить продвинутый сигнал с подтверждением
    pub fn advanced_signal(&self, close: f64, volume: f64, avg_volume: f64) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        let basic_signal = self.trading_signal(close);
        let trend_change = self.trend_change_signal();
        
        // Подтверждение объемом
        let volume_confirmation = volume > avg_volume * 1.2;
        
        // Сильный сигнал при смене тренда с подтверждением объемом
        if trend_change != 0 && volume_confirmation {
            return trend_change;
        }
        
        // Обычный сигнал
        basic_signal
    }
    
    /// Получить расстояние до Supertrend
    pub fn distance_to_supertrend(&self, price: f64) -> f64 {
        if !self.is_ready() {
            return 0.0;
        }
        
        (price - self.supertrend_value) / self.supertrend_value * 100.0
    }
    
    /// Получить силу тренда
    pub fn trend_strength(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.trend_direction.len() < periods {
            return 0.0;
        }
        
        let start_idx = self.trend_direction.len() - periods;
        let slice = &self.trend_direction[start_idx..];
        
        // Считаем процент времени в текущем тренде
        let current_trend_count = slice.iter()
            .filter(|&&x| x == self.current_trend)
            .count();
        
        current_trend_count as f64 / periods as f64 * 100.0
    }
    
    /// Получить продолжительность текущего тренда
    pub fn trend_duration(&self) -> usize {
        if !self.is_ready() || self.trend_direction.is_empty() {
            return 0;
        }
        
        let mut duration = 1;
        let current = self.current_trend;
        
        // Идем назад от текущего значения
        for i in (0..self.trend_direction.len().saturating_sub(1)).rev() {
            if self.trend_direction[i] == current {
                duration += 1;
            } else {
                break;
            }
        }
        
        duration
    }
    
    /// Получить волатильность Supertrend
    pub fn volatility(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.supertrend_values.len() < periods {
            return 0.0;
        }
        
        let start_idx = self.supertrend_values.len() - periods;
        let slice = &self.supertrend_values[start_idx..];
        
        // Рассчитываем стандартное отклонение
        let mean = slice.iter().sum::<f64>() / slice.len() as f64;
        let variance = slice.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / slice.len() as f64;
        
        variance.sqrt()
    }
    
    /// Получить скорость изменения Supertrend
    pub fn rate_of_change(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.supertrend_values.len() < periods + 1 {
            return 0.0;
        }
        
        let current = self.supertrend_value;
        let past = self.supertrend_values[self.supertrend_values.len() - periods - 1];
        
        if past.abs() > 1e-12 {
            (current - past) / past * 100.0
        } else {
            0.0
        }
    }
    
    /// Получить уровень поддержки/сопротивления
    pub fn support_resistance_level(&self) -> (&'static str, f64) {
        if !self.is_ready() {
            return ("Unknown", 0.0);
        }
        
        match self.current_trend {
            1 => ("Support", self.supertrend_value),
            -1 => ("Resistance", self.supertrend_value),
            _ => ("Neutral", self.supertrend_value)
        }
    }
    
    /// Получить статистику по трендам
    pub fn trend_statistics(&self, periods: usize) -> (f64, f64, usize) {
        if !self.is_ready() || self.trend_direction.len() < periods {
            return (0.0, 0.0, 0);
        }
        
        let start_idx = self.trend_direction.len() - periods;
        let slice = &self.trend_direction[start_idx..];
        
        let uptrend_count = slice.iter().filter(|&&x| x == 1).count();
        let downtrend_count = slice.iter().filter(|&&x| x == -1).count();
        
        let uptrend_percentage = uptrend_count as f64 / periods as f64 * 100.0;
        let downtrend_percentage = downtrend_count as f64 / periods as f64 * 100.0;
        
        // Количество смен тренда
        let mut trend_changes = 0;
        for i in 1..slice.len() {
            if slice[i] != slice[i - 1] {
                trend_changes += 1;
            }
        }
        
        (uptrend_percentage, downtrend_percentage, trend_changes)
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        let duration = self.trend_duration();
        let strength = self.trend_strength(20);
        let (level_type, level_value) = self.support_resistance_level();

        format!(
            "Supertrend: {:.2}, Trend: {}, Duration: {} bars, Strength: {:.1}%, {} Level: {:.2}",
            self.supertrend_value,
            self.trend_condition(),
            duration,
            strength,
            level_type,
            level_value
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supertrend_creation() {
        let st = Supertrend::new();
        assert!(!st.is_ready());
        assert_eq!(st.parameters(), (10, 3.0));
    }

    #[test]
    fn test_supertrend_with_params() {
        let st = Supertrend::with_params(14, 2.5);
        assert!(!st.is_ready());
        assert_eq!(st.parameters(), (14, 2.5));
    }

    #[test]
    fn test_supertrend_warmup() {
        let mut st = Supertrend::new();
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            st.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(st.is_ready());
    }

    #[test]
    fn test_supertrend_values() {
        let mut st = Supertrend::new();
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let value = st.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite());
        }
        let dir = st.trend_direction();
        assert!(dir == 1 || dir == -1);
    }

    #[test]
    fn test_supertrend_reset() {
        let mut st = Supertrend::new();
        for i in 0..30 {
            st.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        st.reset();
        assert!(!st.is_ready());
    }
} 






















