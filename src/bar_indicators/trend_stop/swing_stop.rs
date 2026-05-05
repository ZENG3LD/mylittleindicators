//! Swing Stop - индикатор динамических уровней на основе свинг хаев и лоуз
//! 
//! Вычисляет уровни на основе значимых свинг экстремумов:
//! - Swing High - локальный максимум, окруженный более низкими барами
//! - Swing Low - локальный минимум, окруженный более высокими барами
//! 
//! Может использовать:
//! - Последние свинг уровни как стопы
//! - Свинги с отступом (swing ± offset) для большей безопасности
//! - Фильтрацию по значимости свинга (минимальное расстояние)
//! 
//! Индикатор НЕ содержит логику стопов - только возвращает свинг уровни.
//! Логика остановки позиций реализуется в стратегиях на основе этих уровней.

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Swing Stop индикатор - уровни на основе свинг экстремумов
#[derive(Debug, Clone)]
pub struct SwingStop {
    lookback: usize,        // Количество баров для поиска свинга с каждой стороны
    min_swing_size: f64,    // Минимальный размер свинга для фильтрации
    offset: f64,            // Отступ от свинга в пунктах/процентах
    use_percentage: bool,   // Использовать процентный отступ
    
    // Буферы цен
    highs: ArrayVec<f64, 1024>,
    lows: ArrayVec<f64, 1024>,
    
    // Текущие свинг уровни
    last_swing_high: f64,
    last_swing_low: f64,
    
    // Уровни с отступами
    long_stop: f64,     // Последний swing low с отступом
    short_stop: f64,    // Последний swing high с отступом
    
    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl SwingStop {
    /// Создать Swing Stop с параметрами по умолчанию
    /// (lookback=5, min_swing_size=0.0, offset=0.0, absolute)
    pub fn new() -> Self {
        Self::with_params(5, 0.0, 0.0, false)
    }
    
    /// Создать Swing Stop с настраиваемыми параметрами
    /// * `lookback` - количество баров для поиска свинга с каждой стороны
    /// * `min_swing_size` - минимальный размер свинга для фильтрации  
    /// * `offset` - отступ от свинга (пункты или %)
    /// * `use_percentage` - использовать процентный отступ
    pub fn with_params(lookback: usize, min_swing_size: f64, offset: f64, use_percentage: bool) -> Self {
        assert!(lookback > 0, "Lookback must be greater than 0");
        
        Self {
            lookback,
            min_swing_size,
            offset,
            use_percentage,
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            last_swing_high: 0.0,
            last_swing_low: f64::MAX,
            long_stop: 0.0,
            short_stop: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Создать для оптимизации с широким диапазоном параметров
    pub fn for_optimization(lookback: usize, min_swing_size: f64, offset: f64, use_percentage: bool) -> Self {
        Self::with_params(lookback, min_swing_size, offset, use_percentage)
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, _close: f64, _volume: f64) -> (f64, f64) {
        self.bars_count += 1;
        
        // Добавляем цены в буферы
        let buffer_size = self.lookback * 2 + 10; // Дополнительный размер для стабильности
        if self.highs.len() >= buffer_size {
            self.highs.remove(0);
            self.lows.remove(0);
        }
        self.highs.push(high);
        self.lows.push(low);
        
        // Ищем свинги только если есть достаточно данных
        if self.highs.len() >= (self.lookback * 2 + 1) {
            self.find_swings();
        }
        
        // Вычисляем уровни стопов с отступами
        self.calculate_stop_levels();
        
        // Готов когда найдены первые свинги
        self.is_ready = self.bars_count >= (self.lookback * 2 + 1) && 
                       (self.last_swing_high > 0.0 || self.last_swing_low < f64::MAX);
        
        (self.long_stop, self.short_stop)
    }
    
    /// Найти свинг хаи и лоузы
    fn find_swings(&mut self) {
        let len = self.highs.len();
        if len < (self.lookback * 2 + 1) {
            return;
        }
        
        // Проверяем возможный свинг в центре окна (lookback bars back from current)
        let swing_index = len - self.lookback - 1;
        
        // Проверяем swing high
        let candidate_high = self.highs[swing_index];
        let mut is_swing_high = true;
        
        // Проверяем что это локальный максимум
        for i in (swing_index.saturating_sub(self.lookback))..swing_index {
            if self.highs[i] >= candidate_high {
                is_swing_high = false;
                break;
            }
        }
        
        if is_swing_high {
            for i in (swing_index + 1)..=(swing_index + self.lookback).min(len - 1) {
                if self.highs[i] >= candidate_high {
                    is_swing_high = false;
                    break;
                }
            }
        }
        
        // Проверяем минимальный размер свинга
        if is_swing_high && self.min_swing_size > 0.0
            && self.last_swing_high > 0.0 {
                let swing_size = (candidate_high - self.last_swing_high).abs();
                if swing_size < self.min_swing_size {
                    is_swing_high = false;
                }
            }
        
        if is_swing_high {
            self.last_swing_high = candidate_high;
        }
        
        // Проверяем swing low
        let candidate_low = self.lows[swing_index];
        let mut is_swing_low = true;
        
        // Проверяем что это локальный минимум
        for i in (swing_index.saturating_sub(self.lookback))..swing_index {
            if self.lows[i] <= candidate_low {
                is_swing_low = false;
                break;
            }
        }
        
        if is_swing_low {
            for i in (swing_index + 1)..=(swing_index + self.lookback).min(len - 1) {
                if self.lows[i] <= candidate_low {
                    is_swing_low = false;
                    break;
                }
            }
        }
        
        // Проверяем минимальный размер свинга
        if is_swing_low && self.min_swing_size > 0.0
            && self.last_swing_low < f64::MAX {
                let swing_size = (self.last_swing_low - candidate_low).abs();
                if swing_size < self.min_swing_size {
                    is_swing_low = false;
                }
            }
        
        if is_swing_low {
            self.last_swing_low = candidate_low;
        }
    }
    
    /// Вычислить уровни стопов с отступами
    fn calculate_stop_levels(&mut self) {
        // Лонг стоп = последний swing low с отступом вниз
        if self.last_swing_low < f64::MAX {
            if self.use_percentage {
                self.long_stop = self.last_swing_low * (1.0 - self.offset / 100.0);
            } else {
                self.long_stop = self.last_swing_low - self.offset;
            }
        }
        
        // Шорт стоп = последний swing high с отступом вверх
        if self.last_swing_high > 0.0 {
            if self.use_percentage {
                self.short_stop = self.last_swing_high * (1.0 + self.offset / 100.0);
            } else {
                self.short_stop = self.last_swing_high + self.offset;
            }
        }
    }
    
    /// Получить уровень для лонг позиций (последний swing low с отступом)
    pub fn long_stop(&self) -> f64 {
        self.long_stop
    }
    
    /// Получить уровень для шорт позиций (последний swing high с отступом)  
    pub fn short_stop(&self) -> f64 {
        self.short_stop
    }
    
    /// Получить основной уровень (лонг по умолчанию)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.long_stop)
    }
    
    /// Получить оба уровня как кортеж (long, short)
    pub fn levels(&self) -> (f64, f64) {
        (self.long_stop, self.short_stop)
    }
    
    /// Получить последний swing high (без отступа)
    pub fn last_swing_high(&self) -> f64 {
        self.last_swing_high
    }
    
    /// Получить последний swing low (без отступа)
    pub fn last_swing_low(&self) -> f64 {
        if self.last_swing_low == f64::MAX { 0.0 } else { self.last_swing_low }
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.last_swing_high = 0.0;
        self.last_swing_low = f64::MAX;
        self.long_stop = 0.0;
        self.short_stop = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
    
    /// Получить параметры индикатора
    pub fn params(&self) -> (usize, f64, f64, bool) {
        (self.lookback, self.min_swing_size, self.offset, self.use_percentage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swing_stop_creation() {
        let ind = SwingStop::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_swing_stop_with_params() {
        let ind = SwingStop::with_params(5, 0.0, 0.5, false);
        assert!(!ind.is_ready());
        assert_eq!(ind.params(), (5, 0.0, 0.5, false));
    }

    #[test]
    fn test_swing_stop_warmup() {
        let mut ind = SwingStop::with_params(3, 0.0, 0.0, false);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 5.0;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_swing_stop_values_finite() {
        let mut ind = SwingStop::with_params(3, 0.0, 0.0, false);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let (long, short) = ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(long.is_finite());
            assert!(short.is_finite());
        }
    }

    #[test]
    fn test_swing_stop_reset() {
        let mut ind = SwingStop::with_params(3, 0.0, 0.0, false);
        for i in 0..20 {
            ind.update_bar(100.0 + (i as f64 * 0.3).sin() * 5.0, 105.0, 95.0, 101.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
} 