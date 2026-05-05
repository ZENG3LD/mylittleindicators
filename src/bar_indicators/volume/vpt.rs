//! Volume Price Trend (VPT) - индикатор объемо-ценового тренда
//! VPT = Previous VPT + Volume * (Close - Previous Close) / Previous Close
//! Показывает кумулятивную связь между объемом и ценовыми изменениями
//! Используется для подтверждения трендов и поиска дивергенций

use arrayvec::ArrayVec;
use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Volume Price Trend индикатор
#[derive(Clone)]
pub struct VolumePriceTrend {
    // Период для сигнальной линии
    signal_period: usize,
    
    // Буферы для значений
    vpt_values: ArrayVec<f64, 512>,
    
    // Сигнальная линия (MA от VPT)
    signal_ma: MovingAverageProvider,
    
    // Предыдущие значения
    prev_close: f64,
    
    // Текущие значения
    vpt_value: f64,
    signal_value: f64,
    
    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl VolumePriceTrend {
    /// Создать новый VPT с периодом сигнальной линии по умолчанию (21)
    pub fn new() -> Self {
        Self::with_signal_period(21)
    }
    
    /// Создать новый VPT с настраиваемым периодом сигнальной линии
    pub fn with_signal_period(signal_period: usize) -> Self {
        assert!(signal_period > 0, "Signal period must be greater than 0");
        
        Self {
            signal_period,
            vpt_values: ArrayVec::new(),
            signal_ma: MovingAverageProvider::new(MovingAverageType::SMA, signal_period),
            prev_close: 0.0,
            vpt_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, volume: f64) -> f64 {
        self.bars_count += 1;
        
        if self.bars_count == 1 {
            // Первый бар - инициализируем предыдущую цену
            self.prev_close = close;
            self.vpt_value = 0.0;
            return self.vpt_value;
        }
        
        // Рассчитываем процентное изменение цены
        let price_change_pct = if self.prev_close.abs() > 1e-12 {
            (close - self.prev_close) / self.prev_close
        } else {
            0.0
        };
        
        // Рассчитываем VPT
        self.vpt_value += volume * price_change_pct;
        
        // Добавляем в буфер
        if self.vpt_values.len() >= 512 {
            self.vpt_values.remove(0);
        }
        self.vpt_values.push(self.vpt_value);
        
        // Рассчитываем сигнальную линию
        self.signal_value = self.signal_ma.update_bar(self.vpt_value, self.vpt_value, self.vpt_value, self.vpt_value, 1.0);
        
        // Обновляем предыдущую цену
        self.prev_close = close;
        
        // Проверяем готовность
        if self.bars_count >= self.signal_period + 5 {
            self.is_ready = true;
        }
        
        self.vpt_value
    }
    
    /// Получить значение VPT
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.vpt_value)
    }
    
    /// Получить значение сигнальной линии
    pub fn signal_value(&self) -> f64 {
        self.signal_value
    }
    
    /// Получить разность между VPT и сигнальной линией
    pub fn histogram(&self) -> f64 {
        self.vpt_value - self.signal_value
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить период сигнальной линии
    pub fn signal_period(&self) -> usize {
        self.signal_period
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.vpt_values.clear();
        self.signal_ma.reset();
        self.prev_close = 0.0;
        self.vpt_value = 0.0;
        self.signal_value = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
    
    /// Определить состояние тренда
    pub fn trend_condition(&self) -> &'static str {
        if self.vpt_value > self.signal_value && self.vpt_value > 0.0 {
            "Strong Bullish"
        } else if self.vpt_value > self.signal_value {
            "Bullish"
        } else if self.vpt_value < self.signal_value && self.vpt_value < 0.0 {
            "Strong Bearish"
        } else if self.vpt_value < self.signal_value {
            "Bearish"
        } else {
            "Neutral"
        }
    }
    
    /// Получить торговый сигнал
    /// 1 = покупка, -1 = продажа, 0 = нейтрально
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        // Сигнал на основе пересечения сигнальной линии
        if self.vpt_value > self.signal_value {
            1  // Покупка - VPT выше сигнальной линии
        } else if self.vpt_value < self.signal_value {
            -1 // Продажа - VPT ниже сигнальной линии
        } else {
            0  // Нейтрально
        }
    }
    
    /// Получить продвинутый сигнал с подтверждением
    pub fn advanced_signal(&self) -> i8 {
        if !self.is_ready() || self.vpt_values.len() < 3 {
            return 0;
        }
        
        let len = self.vpt_values.len();
        let current = self.vpt_value;
        let prev_1 = if len >= 2 { self.vpt_values[len - 2] } else { 0.0 };
        let _prev_2 = if len >= 3 { self.vpt_values[len - 3] } else { 0.0 };
        
        // Сигнал покупки: VPT пересекает сигнальную линию снизу вверх и растет
        if prev_1 <= self.signal_value && current > self.signal_value && current > prev_1 {
            return 1;
        }
        
        // Сигнал продажи: VPT пересекает сигнальную линию сверху вниз и падает
        if prev_1 >= self.signal_value && current < self.signal_value && current < prev_1 {
            return -1;
        }
        
        0
    }
    
    /// Проверить дивергенцию между ценой и VPT
    pub fn check_divergence(&self, price_history: &[f64], lookback: usize) -> i8 {
        if !self.is_ready() || price_history.len() < lookback + 1 || self.vpt_values.len() < lookback + 1 {
            return 0;
        }
        
        let current_price = price_history[price_history.len() - 1];
        let past_price = price_history[price_history.len() - lookback - 1];
        let current_vpt = self.vpt_value;
        let past_vpt = self.vpt_values[self.vpt_values.len() - lookback - 1];
        
        let price_change = current_price - past_price;
        let vpt_change = current_vpt - past_vpt;
        
        // Бычья дивергенция: цена делает новый минимум, но VPT растет
        if price_change < 0.0 && vpt_change > 0.0 {
            return 1;
        }
        
        // Медвежья дивергенция: цена делает новый максимум, но VPT падает
        if price_change > 0.0 && vpt_change < 0.0 {
            return -1;
        }
        
        0
    }
    
    /// Получить силу тренда
    pub fn trend_strength(&self) -> f64 {
        if !self.is_ready() {
            return 0.0;
        }
        
        self.vpt_value.abs()
    }
    
    /// Получить скорость изменения VPT
    pub fn rate_of_change(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.vpt_values.len() < periods + 1 {
            return 0.0;
        }
        
        let current = self.vpt_value;
        let past = self.vpt_values[self.vpt_values.len() - periods - 1];
        
        if past.abs() < 1e-12 {
            0.0
        } else {
            (current - past) / past * 100.0
        }
    }
    
    /// Получить направление тренда
    pub fn trend_direction(&self, lookback: usize) -> i8 {
        if !self.is_ready() || self.vpt_values.len() < lookback + 1 {
            return 0;
        }
        
        let current = self.vpt_value;
        let past = self.vpt_values[self.vpt_values.len() - lookback - 1];
        
        if current > past {
            1  // Восходящий тренд
        } else if current < past {
            -1 // Нисходящий тренд
        } else {
            0  // Боковой тренд
        }
    }
    
    /// Получить волатильность VPT
    pub fn volatility(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.vpt_values.len() < periods {
            return 0.0;
        }
        
        let start_idx = self.vpt_values.len() - periods;
        let slice = &self.vpt_values[start_idx..];
        
        // Рассчитываем стандартное отклонение
        let mean = slice.iter().sum::<f64>() / slice.len() as f64;
        let variance = slice.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / slice.len() as f64;
        
        variance.sqrt()
    }
    
    /// Получить среднее значение VPT за период
    pub fn average_value(&self, periods: usize) -> f64 {
        if !self.is_ready() || self.vpt_values.len() < periods {
            return 0.0;
        }
        
        let start_idx = self.vpt_values.len() - periods;
        let slice = &self.vpt_values[start_idx..];
        
        slice.iter().sum::<f64>() / slice.len() as f64
    }
    
    /// Получить экстремумы VPT за период
    pub fn extremes(&self, periods: usize) -> (f64, f64) {
        if !self.is_ready() || self.vpt_values.len() < periods {
            return (0.0, 0.0);
        }
        
        let start_idx = self.vpt_values.len() - periods;
        let slice = &self.vpt_values[start_idx..];
        
        let max_val = slice.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min_val = slice.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        
        (min_val, max_val)
    }
    
    /// Получить нормализованное значение VPT (от 0 до 1)
    pub fn normalized_value(&self, periods: usize) -> f64 {
        if !self.is_ready() {
            return 0.5;
        }
        
        let (min_val, max_val) = self.extremes(periods);
        let range = max_val - min_val;
        
        if range.abs() < 1e-12 {
            0.5
        } else {
            (self.vpt_value - min_val) / range
        }
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        let strength = self.trend_strength();
        let roc = self.rate_of_change(5);
        let direction = match self.trend_direction(5) {
            1 => "Up",
            -1 => "Down",
            _ => "Sideways"
        };

        format!(
            "VPT: {:.2}, Signal: {:.2}, Histogram: {:.2}, Trend: {}, Strength: {:.2}, ROC: {:.2}%, Direction: {}",
            self.vpt_value,
            self.signal_value,
            self.histogram(),
            self.trend_condition(),
            strength,
            roc,
            direction
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vpt_creation() {
        let vpt = VolumePriceTrend::new();
        assert!(!vpt.is_ready());
        assert_eq!(vpt.value().main(), 0.0);
    }

    #[test]
    fn test_vpt_with_signal_period() {
        let vpt = VolumePriceTrend::with_signal_period(10);
        assert!(!vpt.is_ready());
        assert_eq!(vpt.signal_period(), 10);
    }

    #[test]
    fn test_vpt_warmup() {
        let mut vpt = VolumePriceTrend::with_signal_period(10);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vpt.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vpt.is_ready());
    }

    #[test]
    fn test_vpt_values_finite() {
        let mut vpt = VolumePriceTrend::new();
        for i in 0..40 {
            let price = 100.0 + i as f64;
            let value = vpt.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_vpt_reset() {
        let mut vpt = VolumePriceTrend::new();
        for i in 0..30 {
            vpt.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        vpt.reset();
        assert!(!vpt.is_ready());
        assert_eq!(vpt.value().main(), 0.0);
    }
} 






















