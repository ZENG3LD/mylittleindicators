// MACD Histogram wrapper: outputs MACD - Signal

use crate::bar_indicators::momentum::macd::Macd;
use crate::bar_indicators::indicator_value::IndicatorValue;


#[derive(Clone)]
pub struct MacdHistogram {
    macd: Macd,
    value: f64,
}

impl MacdHistogram {
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
        self.value = self.macd.value_histogram();
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macd_histogram_creation() {
        let hist = MacdHistogram::new(12, 26, 9);
        assert!(!hist.is_ready());
        assert_eq!(hist.value().main(), 0.0);
    }

    #[test]
    fn test_macd_histogram_uptrend() {
        let mut hist = MacdHistogram::new(12, 26, 9);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            hist.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(hist.is_ready());
        // In strong uptrend, MACD > Signal, so histogram > 0
        assert!(hist.value().main() > 0.0, "MACD Histogram should be positive in uptrend, got {}", hist.value().main());
    }

    #[test]
    fn test_macd_histogram_downtrend() {
        let mut hist = MacdHistogram::new(12, 26, 9);
        for i in 1..=50 {
            let price = 200.0 - i as f64 * 2.0;
            hist.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(hist.is_ready());
        // In strong downtrend, MACD < Signal, so histogram < 0
        assert!(hist.value().main() < 0.0, "MACD Histogram should be negative in downtrend, got {}", hist.value().main());
    }

    #[test]
    fn test_macd_histogram_reset() {
        let mut hist = MacdHistogram::new(12, 26, 9);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            hist.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(hist.is_ready());
        hist.reset();
        assert!(!hist.is_ready());
        assert_eq!(hist.value().main(), 0.0);
    }

    #[test]
    fn test_macd_histogram_finite_values() {
        let mut hist = MacdHistogram::new(12, 26, 9);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = hist.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "MACD Histogram should always be finite");
        }
    }
}
