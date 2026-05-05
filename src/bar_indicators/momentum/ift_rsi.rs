// Inverse Fisher Transform of RSI

use crate::bar_indicators::momentum::rsi::Rsi;
use crate::bar_indicators::indicator_value::IndicatorValue;


#[derive(Clone)]
pub struct IftRsi {
    rsi: Rsi,
    value: f64,
}

impl IftRsi {
    pub fn new(period: usize) -> Self {
        Self {
            rsi: Rsi::new(period.max(1)),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.rsi.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rsi.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let r = self.rsi.update_bar(o, h, l, c, v);
        // RSI returns 0.0-1.0, normalize to [-1, 1]
        let normalized_rsi = (r - 0.5) * 2.0; // [0, 1] -> [-1, 1]
        // Apply tanh for smoother Fisher Transform
        self.value = normalized_rsi.tanh();
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ift_rsi_creation() {
        let ift = IftRsi::new(14);
        assert!(!ift.is_ready());
        assert_eq!(ift.value().main(), 0.0);
    }

    #[test]
    fn test_ift_rsi_uptrend() {
        let mut ift = IftRsi::new(14);
        for i in 1..=40 {
            let price = 100.0 + i as f64 * 2.0;
            ift.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ift.is_ready());
        // In uptrend, RSI > 50, so IFT RSI > 0
        assert!(ift.value().main() > 0.0, "IFT RSI should be positive in uptrend, got {}", ift.value().main());
    }

    #[test]
    fn test_ift_rsi_downtrend() {
        let mut ift = IftRsi::new(14);
        for i in 1..=40 {
            let price = 200.0 - i as f64 * 2.0;
            ift.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ift.is_ready());
        // In downtrend, RSI < 50, so IFT RSI < 0
        assert!(ift.value().main() < 0.0, "IFT RSI should be negative in downtrend, got {}", ift.value().main());
    }

    #[test]
    fn test_ift_rsi_range() {
        let mut ift = IftRsi::new(14);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = ift.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            // tanh output is in (-1, 1)
            assert!(value >= -1.0 && value <= 1.0, "IFT RSI should be in [-1, 1], got {}", value);
        }
    }

    #[test]
    fn test_ift_rsi_reset() {
        let mut ift = IftRsi::new(14);
        for i in 1..=40 {
            let price = 100.0 + i as f64;
            ift.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ift.is_ready());
        ift.reset();
        assert!(!ift.is_ready());
        assert_eq!(ift.value().main(), 0.0);
    }
}
