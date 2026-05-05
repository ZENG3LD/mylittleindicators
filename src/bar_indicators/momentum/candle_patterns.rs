// High-performance Candle Patterns detector
// Распознавание паттернов свечей: engulfing, hammer, shooting star
// (c) 2024

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CandlePattern {
    None,
    BullishEngulfing,
    BearishEngulfing,
    Hammer,
    ShootingStar,
}

#[derive(Clone)]
pub struct CandlePatterns {
    prev_bar: Option<(f64, f64, f64, f64)>, // (open, high, low, close)
    current_pattern: CandlePattern,
    min_candle_size_pct: f64, // Минимальный размер свечи в %
}

impl CandlePatterns {
    pub fn new(min_candle_size_pct: f64) -> Self {
        Self {
            prev_bar: None,
            current_pattern: CandlePattern::None,
            min_candle_size_pct,
        }
    }
    
    /// Обновить паттерны новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, _volume: f64) -> CandlePattern {
        let current_bar = (open, high, low, close);
        
        // Проверяем минимальный размер свечи
        let candle_size_pct = ((close - open).abs() / open) * 100.0;
        if candle_size_pct < self.min_candle_size_pct {
            self.current_pattern = CandlePattern::None;
            self.prev_bar = Some(current_bar);
            return self.current_pattern;
        }
        
        self.current_pattern = CandlePattern::None;
        
        // Проверяем паттерны только если есть предыдущий бар
        if let Some((prev_open, prev_high, prev_low, prev_close)) = self.prev_bar {
            // Bullish Engulfing
            if self.is_bullish_engulfing(prev_open, prev_high, prev_low, prev_close, open, high, low, close) {
                self.current_pattern = CandlePattern::BullishEngulfing;
            }
            // Bearish Engulfing
            else if self.is_bearish_engulfing(prev_open, prev_high, prev_low, prev_close, open, high, low, close) {
                self.current_pattern = CandlePattern::BearishEngulfing;
            }
        }
        
        // Hammer (не требует предыдущий бар)
        if self.current_pattern == CandlePattern::None && self.is_hammer(open, high, low, close) {
            self.current_pattern = CandlePattern::Hammer;
        }
        
        // Shooting Star (не требует предыдущий бар)
        if self.current_pattern == CandlePattern::None && self.is_shooting_star(open, high, low, close) {
            self.current_pattern = CandlePattern::ShootingStar;
        }
        
        self.prev_bar = Some(current_bar);
        self.current_pattern
    }
    
    /// Проверка Bullish Engulfing
    fn is_bullish_engulfing(&self, prev_open: f64, _prev_high: f64, _prev_low: f64, prev_close: f64,
                           open: f64, _high: f64, _low: f64, close: f64) -> bool {
        // Текущая свеча бычья и предыдущая медвежья
        let current_bullish = close > open;
        let prev_bearish = prev_close < prev_open;
        
        // Текущая свеча поглощает предыдущую
        let engulfs = open < prev_close && close > prev_open;
        
        current_bullish && prev_bearish && engulfs
    }
    
    /// Проверка Bearish Engulfing
    fn is_bearish_engulfing(&self, prev_open: f64, _prev_high: f64, _prev_low: f64, prev_close: f64,
                           open: f64, _high: f64, _low: f64, close: f64) -> bool {
        // Текущая свеча медвежья и предыдущая бычья
        let current_bearish = close < open;
        let prev_bullish = prev_close > prev_open;
        
        // Текущая свеча поглощает предыдущую
        let engulfs = open > prev_close && close < prev_open;
        
        current_bearish && prev_bullish && engulfs
    }
    
    /// Проверка Hammer
    fn is_hammer(&self, open: f64, high: f64, low: f64, close: f64) -> bool {
        let body_size = (close - open).abs();
        let total_range = high - low;
        let lower_shadow = open.min(close) - low;
        let upper_shadow = high - open.max(close);
        
        // Условия для Hammer:
        // 1. Длинная нижняя тень (минимум в 2 раза больше тела)
        // 2. Короткая или отсутствующая верхняя тень
        // 3. Тело в верхней части диапазона
        if total_range == 0.0 { return false; }
        
        let long_lower_shadow = lower_shadow >= 2.0 * body_size;
        let short_upper_shadow = upper_shadow <= body_size * 0.5;
        let body_in_upper_part = (close.min(open) - low) / total_range >= 0.6;
        let bullish_close = close > open;
        
        long_lower_shadow && short_upper_shadow && body_in_upper_part && bullish_close
    }
    
    /// Проверка Shooting Star
    fn is_shooting_star(&self, open: f64, high: f64, low: f64, close: f64) -> bool {
        let body_size = (close - open).abs();
        let total_range = high - low;
        let lower_shadow = open.min(close) - low;
        let upper_shadow = high - open.max(close);
        
        // Условия для Shooting Star:
        // 1. Длинная верхняя тень (минимум в 2 раза больше тела)
        // 2. Короткая или отсутствующая нижняя тень
        // 3. Тело в нижней части диапазона
        if total_range == 0.0 { return false; }
        
        let long_upper_shadow = upper_shadow >= 2.0 * body_size;
        let short_lower_shadow = lower_shadow <= body_size * 0.5;
        let body_in_lower_part = (high - close.max(open)) / total_range >= 0.6;
        let bearish_close = close < open;
        
        long_upper_shadow && short_lower_shadow && body_in_lower_part && bearish_close
    }
    
    pub fn value(&self) -> CandlePattern {
        self.current_pattern
    }
    
    pub fn is_bullish_pattern(&self) -> bool {
        matches!(self.current_pattern, CandlePattern::BullishEngulfing | CandlePattern::Hammer)
    }
    
    pub fn is_bearish_pattern(&self) -> bool {
        matches!(self.current_pattern, CandlePattern::BearishEngulfing | CandlePattern::ShootingStar)
    }
    
    pub fn is_ready(&self) -> bool {
        self.prev_bar.is_some()
    }
    
    pub fn reset(&mut self) {
        self.prev_bar = None;
        self.current_pattern = CandlePattern::None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candle_patterns_creation() {
        let cp = CandlePatterns::new(0.1);
        assert!(!cp.is_ready());
        assert_eq!(cp.value(), CandlePattern::None);
    }

    #[test]
    fn test_candle_patterns_bullish_engulfing() {
        let mut cp = CandlePatterns::new(0.1);
        // Previous bar: bearish (open > close)
        cp.update_bar(105.0, 106.0, 99.0, 100.0, 1000.0);
        // Current bar: bullish engulfing (close > prev open, open < prev close)
        let pattern = cp.update_bar(99.0, 110.0, 98.0, 108.0, 1000.0);
        assert_eq!(pattern, CandlePattern::BullishEngulfing);
        assert!(cp.is_bullish_pattern());
    }

    #[test]
    fn test_candle_patterns_bearish_engulfing() {
        let mut cp = CandlePatterns::new(0.1);
        // Previous bar: bullish
        cp.update_bar(100.0, 106.0, 99.0, 105.0, 1000.0);
        // Current bar: bearish engulfing
        let pattern = cp.update_bar(106.0, 107.0, 98.0, 99.0, 1000.0);
        assert_eq!(pattern, CandlePattern::BearishEngulfing);
        assert!(cp.is_bearish_pattern());
    }

    #[test]
    fn test_candle_patterns_reset() {
        let mut cp = CandlePatterns::new(0.1);
        cp.update_bar(100.0, 105.0, 98.0, 103.0, 1000.0);
        assert!(cp.is_ready());
        cp.reset();
        assert!(!cp.is_ready());
        assert_eq!(cp.value(), CandlePattern::None);
    }
} 






















