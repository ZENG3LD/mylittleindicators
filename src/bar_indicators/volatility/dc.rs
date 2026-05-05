// High-performance Donchian Channel (DC)
// (c) 2024

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct Dc {
    period: usize,
    highs: arrayvec::ArrayVec<f64, 512>,
    lows: arrayvec::ArrayVec<f64, 512>,
    index: usize,
    filled: bool,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl Dc {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            highs: arrayvec::ArrayVec::new(),
            lows: arrayvec::ArrayVec::new(),
            index: 0,
            filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }
    /// Обновить Donchian Channel новым баром (используются high, low)
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, _close: f64, _volume: f64) -> (f64, f64, f64) {
        if self.highs.len() < self.period {
            self.highs.push(high);
            self.lows.push(low);
        } else {
            self.highs.remove(0);
            self.lows.remove(0);
            self.highs.push(high);
            self.lows.push(low);
            self.filled = true;
        }
        let len = self.highs.len();
        if len < self.period {
            self.upper = 0.0;
            self.middle = 0.0;
            self.lower = 0.0;
            return (self.upper, self.middle, self.lower);
        }
        let (lower, upper) = self.highs.iter()
            .zip(self.lows.iter())
            .fold((f64::INFINITY, f64::NEG_INFINITY),
                  |(min, max), (&h, &l)| (min.min(l), max.max(h)));
        self.upper = upper;
        self.lower = lower;
        self.middle = 0.5 * (self.upper + self.lower);
        (self.upper, self.middle, self.lower)
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.upper, self.middle, self.lower)
    }
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.index = 0;
        self.filled = false;
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dc_creation() {
        let dc = Dc::new(20);
        assert!(!dc.is_ready());
        assert_eq!(dc.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }

    #[test]
    fn test_dc_warmup() {
        let mut dc = Dc::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            dc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(dc.is_ready());
    }

    #[test]
    fn test_dc_values() {
        let mut dc = Dc::new(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            dc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        if let IndicatorValue::Triple(upper, middle, lower) = dc.value() {
            assert!(upper >= middle);
            assert!(middle >= lower);
        } else {
            panic!("Expected Triple");
        }
    }

    #[test]
    fn test_dc_reset() {
        let mut dc = Dc::new(20);
        for i in 0..25 {
            dc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        dc.reset();
        assert!(!dc.is_ready());
        assert_eq!(dc.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }
}
