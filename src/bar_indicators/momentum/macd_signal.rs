// MACD Signal wrapper: outputs signal line of MACD

use crate::bar_indicators::momentum::macd::Macd;
use crate::bar_indicators::indicator_value::IndicatorValue;


#[derive(Clone)]
pub struct MacdSignal {
    macd: Macd,
    value: f64,
}

impl MacdSignal {
    pub fn new(fast: usize, slow: usize, signal: usize) -> Self {
        Self {
            macd: Macd::new_with_signal(fast.max(1), slow.max(1), signal.max(1)),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.macd.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.macd.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let _ = self.macd.update_bar(o, h, l, c, v);
        self.value = self.macd.value_signal();
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macd_signal_creation() {
        let sig = MacdSignal::new(12, 26, 9);
        assert!(!sig.is_ready());
        assert_eq!(sig.value().main(), 0.0);
    }

    #[test]
    fn test_macd_signal_basic() {
        let mut sig = MacdSignal::new(12, 26, 9);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            sig.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sig.is_ready());
        assert!(sig.value().main().is_finite());
    }

    #[test]
    fn test_macd_signal_reset() {
        let mut sig = MacdSignal::new(12, 26, 9);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            sig.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sig.is_ready());
        sig.reset();
        assert!(!sig.is_ready());
        assert_eq!(sig.value().main(), 0.0);
    }

    #[test]
    fn test_macd_signal_finite_values() {
        let mut sig = MacdSignal::new(12, 26, 9);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = sig.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "MACD Signal should always be finite");
        }
    }
}
