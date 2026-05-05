//! Ease of Movement (EoM) - индикатор легкости движения Ричарда Армса
//! Показывает соотношение изменения цены к объему
//! EoM = Distance Moved / Box Ratio
//! Distance Moved = (High + Low) / 2 - (Previous High + Previous Low) / 2
//! Box Ratio = Volume / Scale Factor / (High - Low)
//! Scale Factor обычно 100,000,000 для нормализации

use arrayvec::ArrayVec;
use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Ease of Movement индикатор
#[derive(Clone)]
pub struct EaseOfMovement {
    scale_factor: f64,
    period: usize,
    
    // Буферы для расчетов
    eom_values: ArrayVec<f64, 512>,
    
    // Предыдущие значения
    prev_high: f64,
    prev_low: f64,
    
    // Сглаживание
    smoothed_eom: MovingAverageProvider,
    
    // Текущее значение
    eom_raw: f64,
    eom_smooth: f64,
    
    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl EaseOfMovement {
    /// Создать новый EoM с периодом сглаживания по умолчанию (14)
    pub fn new() -> Self {
        Self::with_params(14, 100_000_000.0)
    }
    
    /// Создать новый EoM с настраиваемыми параметрами
    pub fn with_params(smoothing_period: usize, scale_factor: f64) -> Self {
        Self {
            scale_factor,
            period: smoothing_period,
            eom_values: ArrayVec::new(),
            prev_high: 0.0,
            prev_low: 0.0,
            smoothed_eom: MovingAverageProvider::new(MovingAverageType::SMA, smoothing_period),
            eom_raw: 0.0,
            eom_smooth: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, _close: f64, volume: f64) -> f64 {
        self.bars_count += 1;
        
        if self.bars_count == 1 {
            // Первый бар - инициализируем предыдущие значения
            self.prev_high = high;
            self.prev_low = low;
            return self.eom_smooth;
        }
        
        // Рассчитываем Distance Moved
        let current_midpoint = (high + low) / 2.0;
        let prev_midpoint = (self.prev_high + self.prev_low) / 2.0;
        let distance_moved = current_midpoint - prev_midpoint;
        
        // Рассчитываем Box Ratio
        let price_range = high - low;
        let box_ratio = if price_range.abs() < 1e-12 || volume.abs() < 1e-12 {
            0.0  // Избегаем деления на ноль
        } else {
            volume / self.scale_factor / price_range
        };
        
        // Рассчитываем сырой EoM
        self.eom_raw = if box_ratio.abs() < 1e-12 {
            0.0
        } else {
            distance_moved / box_ratio
        };
        
        // Добавляем в буфер
        if self.eom_values.len() >= 512 {
            self.eom_values.remove(0);
        }
        self.eom_values.push(self.eom_raw);
        
        // Сглаживаем EoM
        self.eom_smooth = self.smoothed_eom.update_bar(self.eom_raw, self.eom_raw, self.eom_raw, self.eom_raw, 1.0);
        
        // Обновляем предыдущие значения
        self.prev_high = high;
        self.prev_low = low;
        
        // Проверяем готовность
        if self.bars_count >= self.period {
            self.is_ready = true;
        }
        
        self.eom_smooth
    }
    
    /// Получить сглаженное значение EoM
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.eom_smooth)
    }
    
    /// Получить сырое (несглаженное) значение EoM
    pub fn raw_value(&self) -> f64 {
        self.eom_raw
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить период сглаживания
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.eom_values.clear();
        self.prev_high = 0.0;
        self.prev_low = 0.0;
        self.smoothed_eom.reset();
        self.eom_raw = 0.0;
        self.eom_smooth = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
    
    /// Определить состояние рынка
    pub fn market_condition(&self) -> &'static str {
        match self.eom_smooth {
            v if v > 0.0 => "Easy Upward Movement",     // Легкое движение вверх
            v if v < 0.0 => "Easy Downward Movement",   // Легкое движение вниз  
            _ => "Difficult Movement"                   // Трудное движение
        }
    }
    
    /// Получить торговый сигнал
    /// 1 = покупка, -1 = продажа, 0 = нейтрально
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        // Простой сигнал на основе пересечения нулевой линии
        if self.eom_smooth > 0.0 {
            1  // Покупка - легкое движение вверх
        } else if self.eom_smooth < 0.0 {
            -1 // Продажа - легкое движение вниз
        } else {
            0  // Нейтрально
        }
    }
    
    /// Получить усовершенствованный сигнал с подтверждением
    pub fn advanced_signal(&self) -> i8 {
        if !self.is_ready() || self.eom_values.len() < 3 {
            return 0;
        }
        
        let len = self.eom_values.len();
        let current = self.eom_smooth;
        let prev_1 = if len >= 2 { self.eom_values[len - 2] } else { 0.0 };
        let prev_2 = if len >= 3 { self.eom_values[len - 3] } else { 0.0 };
        
        // Сигнал покупки: пересечение нулевой линии снизу вверх с подтверждением
        if prev_2 < 0.0 && prev_1 < 0.0 && current > 0.0 {
            return 1;
        }
        
        // Сигнал продажи: пересечение нулевой линии сверху вниз с подтверждением
        if prev_2 > 0.0 && prev_1 > 0.0 && current < 0.0 {
            return -1;
        }
        
        0
    }
    
    /// Получить силу движения (абсолютное значение EoM)
    pub fn movement_strength(&self) -> f64 {
        self.eom_smooth.abs()
    }
    
    /// Проверить дивергенцию между ценой и EoM
    pub fn check_divergence(&self, price_history: &[f64], lookback: usize) -> i8 {
        if !self.is_ready() || price_history.len() < lookback + 1 || self.eom_values.len() < lookback + 1 {
            return 0;
        }
        
        let current_price = price_history[price_history.len() - 1];
        let past_price = price_history[price_history.len() - lookback - 1];
        let current_eom = self.eom_smooth;
        let past_eom = self.eom_values[self.eom_values.len() - lookback - 1];
        
        let price_change = current_price - past_price;
        let eom_change = current_eom - past_eom;
        
        // Бычья дивергенция: цена делает новый минимум, но EoM растет
        if price_change < 0.0 && eom_change > 0.0 {
            return 1;
        }
        
        // Медвежья дивергенция: цена делает новый максимум, но EoM падает
        if price_change > 0.0 && eom_change < 0.0 {
            return -1;
        }
        
        0
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        format!(
            "EoM: {:.6}, Raw: {:.6}, Condition: {}, Strength: {:.6}",
            self.eom_smooth,
            self.eom_raw,
            self.market_condition(),
            self.movement_strength()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ease_of_movement_creation() {
        let eom = EaseOfMovement::new();
        assert!(!eom.is_ready());
        assert_eq!(eom.value().main(), 0.0);
        assert_eq!(eom.period(), 14);
    }

    #[test]
    fn test_ease_of_movement_with_params() {
        let eom = EaseOfMovement::with_params(20, 50_000_000.0);
        assert!(!eom.is_ready());
        assert_eq!(eom.period(), 20);
    }

    #[test]
    fn test_ease_of_movement_warmup() {
        let mut eom = EaseOfMovement::new();
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            eom.update_bar(price, price + 1.0, price - 1.0, price, 1000000.0);
        }
        assert!(eom.is_ready());
    }

    #[test]
    fn test_ease_of_movement_values_finite() {
        let mut eom = EaseOfMovement::new();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = eom.update_bar(price, price + 1.0, price - 1.0, price, 1000000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_ease_of_movement_reset() {
        let mut eom = EaseOfMovement::new();
        for i in 0..20 {
            eom.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000000.0);
        }
        eom.reset();
        assert!(!eom.is_ready());
        assert_eq!(eom.value().main(), 0.0);
    }

    #[test]
    fn test_ease_of_movement_trading_signal() {
        let mut eom = EaseOfMovement::new();
        for i in 0..20 {
            let price = 100.0 + i as f64;
            eom.update_bar(price, price + 1.0, price - 1.0, price, 1000000.0);
        }
        let signal = eom.trading_signal();
        assert!(signal >= -1 && signal <= 1);
    }
} 






















