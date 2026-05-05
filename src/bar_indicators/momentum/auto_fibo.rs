//! Auto Fibonacci Retracement Indicator
//! High-performance indicator that detects swings and automatically calculates Fibonacci levels
//! Uses ArrayVec for efficient ring buffer operations

// ArrayVec больше не используется
use crate::bar_indicators::zigzag::zigzag_atr::ZigZagAtr;
use crate::bar_indicators::indicator_value::IndicatorValue;

use arrayvec::ArrayVec;

/// Fibonacci levels calculated from swing points
#[derive(Debug, Clone, Copy)]
pub struct FiboLevels {
    pub level_0: f64,     // 0.0% (swing start)
    pub level_236: f64,   // 23.6%
    pub level_382: f64,   // 38.2%
    pub level_500: f64,   // 50.0%
    pub level_618: f64,   // 61.8%
    pub level_786: f64,   // 78.6%
    pub level_1: f64,     // 100.0% (swing end)
    pub level_1236: f64,  // 123.6% (extension)
    pub level_1618: f64,  // 161.8% (extension)
    pub level_2618: f64,  // 261.8% (extension)
    pub level_4236: f64,  // 423.6% (extension)
}

impl FiboLevels {
    /// Create fibonacci levels from swing high/low points
    pub fn from_swing(swing_high: f64, swing_low: f64, is_uptrend: bool) -> Self {
        let range = swing_high - swing_low;
        
        if is_uptrend {
            // Uptrend: retracement from swing_low (0%) to swing_high (100%)
            Self {
                level_0: swing_low,     // 0% (start)
                level_236: swing_low + range * 0.236,
                level_382: swing_low + range * 0.382,
                level_500: swing_low + range * 0.500,
                level_618: swing_low + range * 0.618,
                level_786: swing_low + range * 0.786,
                level_1: swing_high,    // 100% (end)
                level_1236: swing_low + range * 1.236, // Extension
                level_1618: swing_low + range * 1.618,
                level_2618: swing_low + range * 2.618,
                level_4236: swing_low + range * 4.236,
            }
        } else {
            // Downtrend: retracement from swing_high (0%) to swing_low (100%) 
            Self {
                level_0: swing_high,    // 0% (start)
                level_236: swing_high - range * 0.236,
                level_382: swing_high - range * 0.382,
                level_500: swing_high - range * 0.500,
                level_618: swing_high - range * 0.618,
                level_786: swing_high - range * 0.786,
                level_1: swing_low,     // 100% (end)
                level_1236: swing_high - range * 1.236, // Extension
                level_1618: swing_high - range * 1.618,
                level_2618: swing_high - range * 2.618,
                level_4236: swing_high - range * 4.236,
            }
        }
    }

    /// Get fibonacci level by index (0-10)
    pub fn get_level(&self, index: usize) -> Option<f64> {
        match index {
            0 => Some(self.level_0),
            1 => Some(self.level_236),
            2 => Some(self.level_382),
            3 => Some(self.level_500),
            4 => Some(self.level_618),
            5 => Some(self.level_786),
            6 => Some(self.level_1),
            7 => Some(self.level_1236),
            8 => Some(self.level_1618),
            9 => Some(self.level_2618),
            10 => Some(self.level_4236),
            _ => None,
        }
    }
}

/// A significant swing point in price action
#[derive(Debug, Clone, Copy)]
pub struct SwingPoint {
    pub price: f64,
    pub bar_index: usize,
    pub is_high: bool, // true for swing high, false for swing low
}

/// Updates from AutoFibo indicator
#[derive(Debug)]
pub enum AutoFiboUpdate {
    /// No significant change
    None,
    /// New swing detected (returns the new swing point)
    NewSwing(SwingPoint),
    /// Fibonacci levels updated (new levels available)
    FiboUpdated(FiboLevels),
}

/// High-performance Auto Fibonacci indicator using ZigZag ATR detection
#[derive(Debug, Clone)]
pub struct AutoFibo {
    zigzag: ZigZagAtr,
    last_swing_high: Option<SwingPoint>,
    last_swing_low: Option<SwingPoint>,
    bar_counter: usize,
}

impl AutoFibo {
    /// Create new AutoFibo indicator with ZigZag ATR detection (3 параметра)
    pub fn new(zigzag_period: usize, atr_period: usize, atr_multiplier: f64) -> Self {
        let zigzag = ZigZagAtr::new(zigzag_period, atr_period, atr_multiplier);
        Self {
            zigzag,
            last_swing_high: None,
            last_swing_low: None,
            bar_counter: 0,
        }
    }
    
    /// Совместимость со старым API
    pub fn new_compatible(atr: crate::bar_indicators::volatility::atr::Atr, atr_multiplier: f64) -> Self {
        let atr_period = atr.period();
        let zigzag = ZigZagAtr::new_compatible(atr_period, atr_multiplier, atr);
        Self {
            zigzag,
            last_swing_high: None,
            last_swing_low: None,
            bar_counter: 0,
        }
    }
    
    /// Получить старые параметры ATR для совместимости
    pub fn get_atr_params(&self) -> (usize, f64) {
        (self.zigzag.get_atr_period(), self.zigzag.get_atr_multiplier())
    }
    
    /// Получить все 3 параметра
    pub fn get_params(&self) -> (usize, usize, f64) {
        (self.zigzag.get_zigzag_period(), self.zigzag.get_atr_period(), self.zigzag.get_atr_multiplier())
    }

    /// Проверить готовность индикатора (минимум 2 свинга для построения фибо)
    pub fn is_ready(&self) -> bool {
        self.last_swing_high.is_some() && self.last_swing_low.is_some()
    }

    /// Получить последние свинги
    pub fn get_swings(&self) -> (Option<SwingPoint>, Option<SwingPoint>) {
        (self.last_swing_high, self.last_swing_low)
    }
    
    /// Получить все свинги от ZigZag
    pub fn get_all_swings(&self) -> &ArrayVec<(usize, f64), 512> {
        self.zigzag.swings()
    }
    
    /// Обновить индикатор новым баром и получить новый свинг если есть
    pub fn update(&mut self, high: f64, low: f64, close: f64, bar_index: usize) -> Option<SwingPoint> {
        // Update ZigZag and check for new swing
        self.zigzag.update(high, low, close, bar_index);
        
        // Check if we have a new swing point
        if let Some((swing_idx, swing_price)) = self.zigzag.last_swing() {
            // Проверяем что это новый свинг (не дублируется)
            let current_swings_count = self.zigzag.swings().len();
            if current_swings_count <= 1 {
                return None; // Недостаточно свингов
            }
            
            // Берем последние 2 свинга чтобы определить тип
            let swings = self.zigzag.swings();
            let prev_swing = swings[current_swings_count - 2];
            let curr_swing = swings[current_swings_count - 1];
            
            // Определяем тип текущего свинга по отношению к предыдущему
            let is_high = curr_swing.1 > prev_swing.1;
            
            let swing_point = SwingPoint {
                price: swing_price,
                bar_index: swing_idx,
                is_high,
            };

            // Обновляем соответствующий тип свинга и возвращаем только новые
            if is_high {
                if self.last_swing_high.is_none_or(|last| last.bar_index != swing_idx) {
                    self.last_swing_high = Some(swing_point);
                    return Some(swing_point);
                }
            } else if self.last_swing_low.is_none_or(|last| last.bar_index != swing_idx) {
                self.last_swing_low = Some(swing_point);
                return Some(swing_point);
            }
        }

        None
    }
    
    /// Сброс состояния индикатора
    pub fn reset(&mut self) {
        self.zigzag.reset();
        self.last_swing_high = None;
        self.last_swing_low = None;
        self.bar_counter = 0;
    }

    /// Update with OHLCV bar
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        self.update(high, low, close, self.bar_counter);
        self.bar_counter += 1;

        // Calculate 50% Fibonacci level from swings
        if let (Some(swing_high), Some(swing_low)) = self.get_swings() {
            let range = swing_high.price - swing_low.price;
            let mid_price = swing_low.price + range * 0.5;
            return mid_price;
        }
        close
    }

    /// Get current indicator value (50% Fibonacci level)
    pub fn value(&self) -> IndicatorValue {
        if let (Some(swing_high), Some(swing_low)) = self.get_swings() {
            let range = swing_high.price - swing_low.price;
            let mid_price = swing_low.price + range * 0.5;
            return IndicatorValue::Single(mid_price);
        }
        IndicatorValue::Single(0.0)
    }
    
    /// Получить уровни коррекции (retracement) между последними свингами
    /// Возвращает уровни от high к low
    pub fn get_retracement_levels(&self) -> Option<FiboLevels> {
        match (self.last_swing_high, self.last_swing_low) {
            (Some(high), Some(low)) => {
                let (start_price, end_price) = if high.bar_index > low.bar_index {
                    // Недавний high после low - коррекция вниз
                    (high.price, low.price)
                } else {
                    // Недавний low после high - коррекция вверх  
                    (low.price, high.price)
                };
                
                Some(FiboLevels {
                    level_0: start_price,
                    level_236: start_price + (end_price - start_price) * 0.236,
                    level_382: start_price + (end_price - start_price) * 0.382,
                    level_500: start_price + (end_price - start_price) * 0.500,
                    level_618: start_price + (end_price - start_price) * 0.618,
                    level_786: start_price + (end_price - start_price) * 0.786,
                    level_1: end_price,
                    level_1236: start_price + (end_price - start_price) * 1.236,
                    level_1618: start_price + (end_price - start_price) * 1.618,
                    level_2618: start_price + (end_price - start_price) * 2.618,
                    level_4236: start_price + (end_price - start_price) * 4.236,
                })
            }
            _ => None,
        }
    }

    /// Получить уровни расширения (extension) между последними свингами
    /// Возвращает проекцию движения за пределы последнего экстремума
    pub fn get_extension_levels(&self) -> Option<FiboLevels> {
        match (self.last_swing_high, self.last_swing_low) {
            (Some(high), Some(low)) => {
                let (start_price, end_price) = if high.bar_index > low.bar_index {
                    // Последний high после low - экстеншн вверх от high
                    (low.price, high.price)
                } else {
                    // Последний low после high - экстеншн вниз от low
                    (high.price, low.price)
                };

                Some(FiboLevels {
                    level_0: start_price,
                    level_236: start_price + (end_price - start_price) * 0.236,
                    level_382: start_price + (end_price - start_price) * 0.382,
                    level_500: start_price + (end_price - start_price) * 0.500,
                    level_618: start_price + (end_price - start_price) * 0.618,
                    level_786: start_price + (end_price - start_price) * 0.786,
                    level_1: end_price,
                    level_1236: start_price + (end_price - start_price) * 1.236,
                    level_1618: start_price + (end_price - start_price) * 1.618,
                    level_2618: start_price + (end_price - start_price) * 2.618,
                    level_4236: start_price + (end_price - start_price) * 4.236,
                })
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fibo_levels_from_swing_uptrend() {
        let levels = FiboLevels::from_swing(120.0, 100.0, true);
        assert_eq!(levels.level_0, 100.0);
        assert_eq!(levels.level_1, 120.0);
        assert!((levels.level_500 - 110.0).abs() < 0.01);
    }

    #[test]
    fn test_fibo_levels_from_swing_downtrend() {
        let levels = FiboLevels::from_swing(120.0, 100.0, false);
        assert_eq!(levels.level_0, 120.0);
        assert_eq!(levels.level_1, 100.0);
    }

    #[test]
    fn test_fibo_levels_get_level() {
        let levels = FiboLevels::from_swing(120.0, 100.0, true);
        assert!(levels.get_level(0).is_some());
        assert!(levels.get_level(10).is_some());
        assert!(levels.get_level(11).is_none());
    }

    #[test]
    fn test_auto_fibo_creation() {
        let auto_fibo = AutoFibo::new(10, 14, 2.0);
        assert!(!auto_fibo.is_ready());
        assert_eq!(auto_fibo.get_params(), (10, 14, 2.0));
    }

    #[test]
    fn test_auto_fibo_reset() {
        let mut auto_fibo = AutoFibo::new(10, 14, 2.0);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            auto_fibo.update(price + 1.0, price - 1.0, price, i);
        }
        auto_fibo.reset();
        assert!(!auto_fibo.is_ready());
    }
} 






















