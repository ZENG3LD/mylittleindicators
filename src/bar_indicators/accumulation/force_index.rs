//! Force Index (FI) - индекс силы Александра Элдера
//! Измеряет силу быков и медведей с учетом объема
//! Force Index = Volume × (Close - Previous Close)
//! Положительные значения указывают на силу быков, отрицательные - на силу медведей

use arrayvec::ArrayVec;
use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Force Index индикатор
#[derive(Clone)]
pub struct ForceIndex {
    smoothing_period: usize,
    ma_type: MovingAverageType,

    // Буферы для расчетов
    force_values: ArrayVec<f64, 512>,

    // Предыдущая цена закрытия
    prev_close: f64,

    // Сглаживание
    smoothed_force: MovingAverageProvider,

    // Текущие значения
    force_raw: f64,
    force_smooth: f64,

    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl ForceIndex {
    /// Создать новый Force Index без сглаживания
    pub fn new() -> Self {
        Self::with_smoothing(1, MovingAverageType::EMA) // Период 1 = без сглаживания
    }

    /// Создать новый Force Index с заданным периодом сглаживания (backward compatibility)
    pub fn with_smoothing_default(smoothing_period: usize) -> Self {
        Self::with_smoothing(smoothing_period, MovingAverageType::EMA)
    }

    /// Создать новый Force Index с заданным периодом сглаживания
    pub fn with_smoothing(smoothing_period: usize, ma_type: MovingAverageType) -> Self {
        assert!(smoothing_period > 0, "Smoothing period must be greater than 0");

        Self {
            smoothing_period,
            ma_type,
            force_values: ArrayVec::new(),
            prev_close: 0.0,
            smoothed_force: MovingAverageProvider::new(ma_type, smoothing_period),
            force_raw: 0.0,
            force_smooth: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Создать новый Force Index с EMA сглаживанием (популярные периоды: 2, 13)
    pub fn with_ema(ema_period: usize) -> Self {
        Self::with_smoothing(ema_period, MovingAverageType::EMA)
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, volume: f64) -> f64 {
        self.bars_count += 1;
        
        if self.bars_count == 1 {
            // Первый бар - инициализируем предыдущую цену
            self.prev_close = close;
            return self.force_smooth;
        }
        
        // Рассчитываем Force Index
        let price_change = close - self.prev_close;
        self.force_raw = volume * price_change;
        
        // Добавляем в буфер
        if self.force_values.len() >= 512 {
            self.force_values.remove(0);
        }
        self.force_values.push(self.force_raw);
        
        // Сглаживаем Force Index
        if self.smoothing_period == 1 {
            // Без сглаживания
            self.force_smooth = self.force_raw;
        } else {
            // С EMA сглаживанием
            self.force_smooth = self.smoothed_force.update_bar(self.force_raw, self.force_raw, self.force_raw, self.force_raw, 1.0);
        }
        
        // Обновляем предыдущую цену
        self.prev_close = close;
        
        // Проверяем готовность
        if self.bars_count >= 2 {
            self.is_ready = true;
        }
        
        self.force_smooth
    }
    
    /// Получить сглаженное значение Force Index
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.force_smooth)
    }
    
    /// Получить сырое (несглаженное) значение Force Index
    pub fn raw_value(&self) -> f64 {
        self.force_raw
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить период сглаживания
    pub fn smoothing_period(&self) -> usize {
        self.smoothing_period
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.force_values.clear();
        self.prev_close = 0.0;
        self.smoothed_force = MovingAverageProvider::new(self.ma_type, self.smoothing_period);
        self.force_raw = 0.0;
        self.force_smooth = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    /// Установить новый тип скользящей средней
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }
    
    /// Определить состояние рынка
    pub fn market_condition(&self) -> &'static str {
        match self.force_smooth {
            v if v > 1000.0 => "Strong Bull Power",      // Сильная сила быков
            v if v > 0.0 => "Bull Power",                // Сила быков
            v if v < -1000.0 => "Strong Bear Power",     // Сильная сила медведей
            v if v < 0.0 => "Bear Power",                // Сила медведей
            _ => "Neutral"                               // Нейтрально
        }
    }
    
    /// Получить торговый сигнал
    /// 1 = покупка, -1 = продажа, 0 = нейтрально
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        // Простой сигнал на основе знака Force Index
        if self.force_smooth > 0.0 {
            1  // Покупка - сила быков
        } else if self.force_smooth < 0.0 {
            -1 // Продажа - сила медведей
        } else {
            0  // Нейтрально
        }
    }
    
    /// Получить усовершенствованный сигнал с подтверждением
    pub fn advanced_signal(&self, threshold: f64) -> i8 {
        if !self.is_ready() || self.force_values.len() < 3 {
            return 0;
        }
        
        let len = self.force_values.len();
        let current = self.force_smooth;
        let prev_1 = if len >= 2 { self.force_values[len - 2] } else { 0.0 };
        let prev_2 = if len >= 3 { self.force_values[len - 3] } else { 0.0 };
        
        // Сигнал покупки: пересечение порога снизу вверх с подтверждением
        if prev_2 <= threshold && prev_1 <= threshold && current > threshold {
            return 1;
        }
        
        // Сигнал продажи: пересечение порога сверху вниз с подтверждением  
        if prev_2 >= -threshold && prev_1 >= -threshold && current < -threshold {
            return -1;
        }
        
        0
    }
    
    /// Получить силу давления (абсолютное значение Force Index)
    pub fn pressure_strength(&self) -> f64 {
        self.force_smooth.abs()
    }
    
    /// Проверить дивергенцию между ценой и Force Index
    pub fn check_divergence(&self, price_history: &[f64], lookback: usize) -> i8 {
        if !self.is_ready() || price_history.len() < lookback + 1 || self.force_values.len() < lookback + 1 {
            return 0;
        }
        
        let current_price = price_history[price_history.len() - 1];
        let past_price = price_history[price_history.len() - lookback - 1];
        let current_force = self.force_smooth;
        let past_force = self.force_values[self.force_values.len() - lookback - 1];
        
        let price_change = current_price - past_price;
        let force_change = current_force - past_force;
        
        // Бычья дивергенция: цена делает новый минимум, но Force Index растет
        if price_change < 0.0 && force_change > 0.0 {
            return 1;
        }
        
        // Медвежья дивергенция: цена делает новый максимум, но Force Index падает
        if price_change > 0.0 && force_change < 0.0 {
            return -1;
        }
        
        0
    }
    
    /// Получить тренд Force Index
    pub fn trend_direction(&self, lookback: usize) -> i8 {
        if !self.is_ready() || self.force_values.len() < lookback + 1 {
            return 0;
        }
        
        let current = self.force_smooth;
        let past = self.force_values[self.force_values.len() - lookback - 1];
        
        if current > past {
            1  // Восходящий тренд (усиление быков)
        } else if current < past {
            -1 // Нисходящий тренд (усиление медведей)
        } else {
            0  // Боковой тренд
        }
    }
    
    /// Получить нормализованное значение Force Index
    pub fn normalized_value(&self, normalization_period: usize) -> f64 {
        if !self.is_ready() || self.force_values.len() < normalization_period {
            return 0.0;
        }
        
        let len = self.force_values.len();
        let start_idx = len - normalization_period;
        
        // Находим максимум и минимум за период
        let max_force = self.force_values[start_idx..].iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let min_force = self.force_values[start_idx..].iter().fold(f64::INFINITY, |a, &b| a.min(b));
        
        let range = max_force - min_force;
        
        if range.abs() < 1e-12 {
            0.0
        } else {
            (self.force_smooth - min_force) / range * 2.0 - 1.0 // Нормализация к диапазону [-1, 1]
        }
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        format!(
            "FI: {:.2}, Raw: {:.2}, Condition: {}, Strength: {:.2}",
            self.force_smooth,
            self.force_raw,
            self.market_condition(),
            self.pressure_strength()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_force_index_creation() {
        let fi = ForceIndex::new();
        assert!(!fi.is_ready());
        assert_eq!(fi.value().main(), 0.0);
        assert_eq!(fi.smoothing_period(), 1);
    }

    #[test]
    fn test_force_index_with_ema() {
        let fi = ForceIndex::with_ema(13);
        assert!(!fi.is_ready());
        assert_eq!(fi.smoothing_period(), 13);
    }

    #[test]
    fn test_force_index_warmup() {
        let mut fi = ForceIndex::new();
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            fi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(fi.is_ready());
    }

    #[test]
    fn test_force_index_values_finite() {
        let mut fi = ForceIndex::with_ema(13);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = fi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_force_index_reset() {
        let mut fi = ForceIndex::new();
        for i in 0..10 {
            fi.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        fi.reset();
        assert!(!fi.is_ready());
        assert_eq!(fi.value().main(), 0.0);
    }

    #[test]
    fn test_force_index_trading_signal() {
        let mut fi = ForceIndex::new();
        for i in 0..10 {
            let price = 100.0 + i as f64;
            fi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        let signal = fi.trading_signal();
        assert!(signal >= -1 && signal <= 1);
    }
} 






















