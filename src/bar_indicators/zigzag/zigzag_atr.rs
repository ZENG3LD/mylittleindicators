// ZigZag by ATR-multiplier threshold
// (c) 2024

use arrayvec::ArrayVec;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct ZigZagAtr {
    pub zigzag_period: usize,     // Период для расчета ZigZag
    pub atr_mult: f64,           // Множитель ATR
    pub swings: ArrayVec<(usize, f64), 512>,
    pub last_extreme: f64,
    pub last_extreme_idx: usize,
    pub direction: i8,
    pub atr: Atr,               // ATR с собственным периодом
    
    // Оптимизированные циклические буферы как в SMA
    high_buffer: ArrayVec<f64, 1024>,
    low_buffer: ArrayVec<f64, 1024>,
    buffer_count: usize,
    buffer_idx: usize,
    
    // Текущие экстремумы в окне (обновляются при добавлении элементов)
    current_max: f64,
    current_min: f64,
    bar_counter: usize, // internal bar counter for update_bar
}

impl ZigZagAtr {
    /// Создает ZigZag ATR с независимыми периодами
    /// - zigzag_period: период для поиска экстремумов ZigZag (рекомендуется 5-100)
    /// - atr_period: период для расчета ATR (рекомендуется 14-50)
    /// - atr_mult: множитель ATR для определения минимального движения (рекомендуется 0.5-3.0)
    pub fn new(zigzag_period: usize, atr_period: usize, atr_mult: f64) -> Self {
        // Валидация параметров
        assert!(zigzag_period > 0 && zigzag_period <= 1000, "zigzag_period должен быть 1-1000");
        assert!(atr_period > 0 && atr_period <= 500, "atr_period должен быть 1-500");
        assert!(atr_mult > 0.0 && atr_mult <= 10.0, "atr_mult должен быть 0.0-10.0");
        
        let atr = Atr::new(atr_period, crate::bar_indicators::average::MovingAverageType::RMA);
        
        Self {
            zigzag_period,
            atr_mult,
            swings: ArrayVec::new(),
            last_extreme: 0.0,
            last_extreme_idx: 0,
            direction: 0,
            atr,
            high_buffer: ArrayVec::new(),
            low_buffer: ArrayVec::new(),
            buffer_count: 0,
            buffer_idx: 0,
            current_max: f64::NEG_INFINITY,
            current_min: f64::INFINITY,
            bar_counter: 0,
        }
    }

    /// Совместимость со старым API - период ZigZag = период ATR
    pub fn new_compatible(period: usize, atr_mult: f64, atr: Atr) -> Self {
        Self {
            zigzag_period: period,
            atr_mult,
            swings: ArrayVec::new(),
            last_extreme: 0.0,
            last_extreme_idx: 0,
            direction: 0,
            atr,
            high_buffer: ArrayVec::new(),
            low_buffer: ArrayVec::new(),
            buffer_count: 0,
            buffer_idx: 0,
            current_max: f64::NEG_INFINITY,
            current_min: f64::INFINITY,
            bar_counter: 0,
        }
    }

    /// Update with OHLCV bar
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        self.update(high, low, close, self.bar_counter);
        self.bar_counter += 1;
        self.last_swing().map(|(_, price)| price).unwrap_or(close)
    }

    /// Get current indicator value
    pub fn value(&self) -> IndicatorValue {
        let price = self.last_swing().map(|(_, price)| price).unwrap_or(0.0);
        IndicatorValue::Single(price)
    }
    
    pub fn update(&mut self, high: f64, low: f64, close: f64, idx: usize) {
        // Обновляем ATR
        self.atr.update_bar(0.0, high, low, close, 0.0);
        
        // Циклический буфер как в SMA - намного быстрее VecDeque
        if self.buffer_count < self.zigzag_period {
            self.high_buffer.push(high);
            self.low_buffer.push(low);
            self.buffer_count += 1;
            self.buffer_idx = self.buffer_count % self.zigzag_period;
            
            // Обновляем экстремумы при добавлении
            if high > self.current_max { self.current_max = high; }
            if low < self.current_min { self.current_min = low; }
        } else {
            // Перезаписываем старые значения циклически
            if self.buffer_idx >= self.high_buffer.len() {
                self.buffer_idx = 0;
            }
            
            let old_high = self.high_buffer[self.buffer_idx];
            let old_low = self.low_buffer[self.buffer_idx];
            
            self.high_buffer[self.buffer_idx] = high;
            self.low_buffer[self.buffer_idx] = low;
            self.buffer_idx = (self.buffer_idx + 1) % self.zigzag_period;
            
            // Обновляем экстремумы
            if high > self.current_max { 
                self.current_max = high; 
            } else if old_high == self.current_max {
                // Если выпал старый максимум - пересчитываем
                self.recalculate_max();
            }
            
            if low < self.current_min { 
                self.current_min = low; 
            } else if old_low == self.current_min {
                // Если выпал старый минимум - пересчитываем
                self.recalculate_min();
            }
        }
        
        // Не начинаем ZigZag пока не накопится достаточно данных
        if self.buffer_count < self.zigzag_period {
            return;
        }
        
        let threshold = self.atr.value().main() * self.atr_mult;
        
        if self.direction == 0 {
            // Первая инициализация - начинаем с close
            self.last_extreme = close;
            self.last_extreme_idx = idx;
            self.direction = 1; // Начинаем поиск с восходящего тренда
            
            // Добавляем в буфер swings с проверкой переполнения
            if self.swings.len() >= 512 {
                self.swings.remove(0);
            }
            self.swings.push((idx, self.last_extreme));
            return;
        }
        
        // Ищем новые экстремумы в окне
        let current_high = self.find_highest_in_period();
        let current_low = self.find_lowest_in_period();
        
        if self.direction > 0 {
            // Восходящий тренд - ищем новый максимум или разворот вниз
            if current_high > self.last_extreme {
                // Новый максимум - обновляем экстремум
                self.last_extreme = current_high;
                self.last_extreme_idx = idx;
                // Обновляем последний свинг
                if let Some(last) = self.swings.last_mut() {
                    *last = (idx, current_high);
                }
            } else if self.last_extreme - current_low >= threshold {
                // Разворот вниз - новый low свинг
                self.direction = -1;
                self.last_extreme = current_low;
                self.last_extreme_idx = idx;
                
                if self.swings.len() >= 512 {
                    self.swings.remove(0);
                }
                self.swings.push((idx, current_low));
            }
        } else {
            // Нисходящий тренд - ищем новый минимум или разворот вверх
            if current_low < self.last_extreme {
                // Новый минимум - обновляем экстремум
                self.last_extreme = current_low;
                self.last_extreme_idx = idx;
                // Обновляем последний свинг
                if let Some(last) = self.swings.last_mut() {
                    *last = (idx, current_low);
                }
            } else if current_high - self.last_extreme >= threshold {
                // Разворот вверх - новый high свинг  
                self.direction = 1;
                self.last_extreme = current_high;
                self.last_extreme_idx = idx;
                
                if self.swings.len() >= 512 {
                    self.swings.remove(0);
                }
                self.swings.push((idx, current_high));
            }
        }
    }
    
    /// Получить текущий максимум в окне
    fn find_highest_in_period(&self) -> f64 {
        self.current_max
    }
    
    /// Получить текущий минимум в окне
    fn find_lowest_in_period(&self) -> f64 {
        self.current_min
    }
    
    /// Пересчитать максимум в буфере (вызывается когда старый max выпадает)
    fn recalculate_max(&mut self) {
        self.current_max = f64::NEG_INFINITY;
        let count = self.buffer_count.min(self.zigzag_period);
        for i in 0..count {
            if self.high_buffer[i] > self.current_max {
                self.current_max = self.high_buffer[i];
            }
        }
    }
    
    /// Пересчитать минимум в буфере (вызывается когда старый min выпадает)
    fn recalculate_min(&mut self) {
        self.current_min = f64::INFINITY;
        let count = self.buffer_count.min(self.zigzag_period);
        for i in 0..count {
            if self.low_buffer[i] < self.current_min {
                self.current_min = self.low_buffer[i];
            }
        }
    }
    
    pub fn last_swing(&self) -> Option<(usize, f64)> {
        self.swings.last().copied()
    }
    
    pub fn swings(&self) -> &ArrayVec<(usize, f64), 512> {
        &self.swings
    }
    
    /// Получить параметры ZigZag
    pub fn get_zigzag_period(&self) -> usize {
        self.zigzag_period
    }
    
    /// Получить период ATR
    pub fn get_atr_period(&self) -> usize {
        self.atr.period()
    }
    
    /// Получить множитель ATR
    pub fn get_atr_multiplier(&self) -> f64 {
        self.atr_mult
    }
    
    /// Сброс всех данных индикатора
    pub fn reset(&mut self) {
        self.swings.clear();
        self.high_buffer.clear();
        self.low_buffer.clear();
        self.buffer_count = 0;
        self.buffer_idx = 0;
        self.last_extreme = 0.0;
        self.last_extreme_idx = 0;
        self.direction = 0;
        self.atr.reset();
        self.current_max = f64::NEG_INFINITY;
        self.current_min = f64::INFINITY;
        self.bar_counter = 0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.atr.is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zigzag_atr_creation() {
        let ind = ZigZagAtr::new(10, 14, 2.0);
        assert!(!ind.is_ready());
        assert_eq!(ind.get_zigzag_period(), 10);
    }

    #[test]
    fn test_zigzag_atr_warmup() {
        let mut ind = ZigZagAtr::new(10, 14, 2.0);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ind.update(price + 1.0, price - 1.0, price, i);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_zigzag_atr_swings() {
        let mut ind = ZigZagAtr::new(5, 10, 1.0);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            ind.update(price + 2.0, price - 2.0, price, i);
        }
        assert!(ind.swings().len() >= 1);
    }

    #[test]
    fn test_zigzag_atr_reset() {
        let mut ind = ZigZagAtr::new(10, 14, 2.0);
        for i in 0..30 {
            ind.update(100.0 + i as f64, 95.0, 101.0, i);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert!(ind.swings().is_empty());
    }
}






















