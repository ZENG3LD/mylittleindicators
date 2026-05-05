// High-performance Volatility Ratio (VR)
// (c) 2024

use super::atr::Atr;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct Vr {
    atr_fast: Atr,
    atr_slow: Atr,
    value: f64,
}

impl Vr {
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self {
            atr_fast: Atr::new(fast_period, crate::bar_indicators::average::moving_average::MovingAverageType::RMA),
            atr_slow: Atr::new(slow_period, crate::bar_indicators::average::moving_average::MovingAverageType::RMA),
            value: 0.0,
        }
    }
    /// Обновить VR новым баром (используются high, low, close)
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        let fast = self.atr_fast.update_bar(_open, high, low, close, _volume);
        let slow = self.atr_slow.update_bar(_open, high, low, close, _volume);
        if fast > 0.0 {
            self.value = slow / fast;
        } else {
            self.value = 0.0;
        }
        self.value
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.atr_fast.is_ready() && self.atr_slow.is_ready()
    }
    pub fn reset(&mut self) {
        self.atr_fast.reset();
        self.atr_slow.reset();
        self.value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vr_creation() {
        let vr = Vr::new(7, 14);
        assert!(!vr.is_ready());
        assert_eq!(vr.value().main(), 0.0);
    }

    #[test]
    fn test_vr_warmup() {
        let mut vr = Vr::new(7, 14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vr.is_ready());
    }

    #[test]
    fn test_vr_positive() {
        let mut vr = Vr::new(7, 14);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "VR should be non-negative");
        }
    }

    #[test]
    fn test_vr_reset() {
        let mut vr = Vr::new(7, 14);
        for i in 0..20 {
            vr.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        vr.reset();
        assert!(!vr.is_ready());
        assert_eq!(vr.value().main(), 0.0);
    }
} 






















