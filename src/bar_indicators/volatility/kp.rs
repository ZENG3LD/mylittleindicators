// High-performance Keltner Position (KP)
// (c) 2024

use super::kc::Kc;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct Kp {
    kc: Kc,
    value: f64,
}

impl Kp {
    pub fn new(period: usize, k_multiplier: f64) -> Self {
        Self {
            kc: Kc::new(period, k_multiplier),
            value: 0.0,
        }
    }
    /// Обновить KP новым баром (используются high, low, close)
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        let (_, middle, _) = self.kc.update_bar(_open, high, low, close, _volume);
        let k_width = (self.kc.upper - self.kc.lower) / 2.0;
        if k_width > 0.0 {
            self.value = (close - middle) / k_width;
        } else {
            self.value = 0.0;
        }
        self.value
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.kc.is_ready()
    }
    pub fn reset(&mut self) {
        self.kc.reset();
        self.value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kp_creation() {
        let kp = Kp::new(20, 2.0);
        assert!(!kp.is_ready());
        assert_eq!(kp.value().main(), 0.0);
    }

    #[test]
    fn test_kp_warmup() {
        let mut kp = Kp::new(20, 2.0);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(kp.is_ready());
    }

    #[test]
    fn test_kp_values() {
        let mut kp = Kp::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "KP should be finite");
        }
    }

    #[test]
    fn test_kp_reset() {
        let mut kp = Kp::new(20, 2.0);
        for i in 0..25 {
            kp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        kp.reset();
        assert!(!kp.is_ready());
        assert_eq!(kp.value().main(), 0.0);
    }
} 






















