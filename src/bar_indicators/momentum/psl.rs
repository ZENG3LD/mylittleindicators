// High-performance Psychological Line (PSL)
// (c) 2024

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct Psl {
    period: usize,
    buffer: ArrayVec<u8, 512>,
    index: usize,
    filled: bool,
    prev_close: f64,
    value: f64,
}

impl Psl {
    pub fn new(period: usize) -> Self {
        assert!(period <= 512, "PSL period must be <= 512");
        Self {
            period,
            buffer: ArrayVec::from([0u8; 512]),
            index: 0,
            filled: false,
            prev_close: 0.0,
            value: 0.0,
        }
    }
    /// Обновить PSL новым баром (используется close)
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> f64 {
        if self.index == 0 && !self.filled && self.prev_close == 0.0 {
            self.prev_close = close;
            self.index = 1;
            return self.value;
        }
        let up = if close > self.prev_close { 1 } else { 0 };
        self.buffer[self.index % self.period] = up;
        self.prev_close = close;
        self.index += 1;
        if self.index >= self.period {
            self.filled = true;
        }
        let len = if self.filled { self.period } else { self.index };
        if len < self.period {
            self.value = 0.0;
            return self.value;
        }
        let sum_up = self.buffer.iter().take(self.period).map(|&v| v as usize).sum::<usize>();
        self.value = 100.0 * (sum_up as f64) / (self.period as f64);
        self.value
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn reset(&mut self) {
        self.buffer.fill(0);
        self.index = 0;
        self.filled = false;
        self.prev_close = 0.0;
        self.value = 0.0;
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_psl_creation() {
        let psl = Psl::new(12);
        assert!(!psl.is_ready());
        assert_eq!(psl.value().main(), 0.0);
        assert_eq!(psl.period(), 12);
    }

    #[test]
    fn test_psl_uptrend() {
        let mut psl = Psl::new(10);
        for i in 1..=20 {
            let price = 100.0 + i as f64 * 2.0; // always up
            psl.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(psl.is_ready());
        // All bars up = PSL should be 100
        assert!(psl.value().main() > 80.0, "PSL should be high in uptrend, got {}", psl.value().main());
    }

    #[test]
    fn test_psl_downtrend() {
        let mut psl = Psl::new(10);
        for i in 1..=20 {
            let price = 200.0 - i as f64 * 2.0; // always down
            psl.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(psl.is_ready());
        // All bars down = PSL should be 0
        assert!(psl.value().main() < 20.0, "PSL should be low in downtrend, got {}", psl.value().main());
    }

    #[test]
    fn test_psl_range() {
        let mut psl = Psl::new(10);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = psl.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            if psl.is_ready() {
                assert!(value >= 0.0 && value <= 100.0, "PSL should be in [0, 100], got {}", value);
            }
        }
    }

    #[test]
    fn test_psl_reset() {
        let mut psl = Psl::new(10);
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            psl.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(psl.is_ready());
        psl.reset();
        assert!(!psl.is_ready());
        assert_eq!(psl.value().main(), 0.0);
    }
}






















