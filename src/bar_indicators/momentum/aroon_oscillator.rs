// Aroon Oscillator wrapper over Aroon: Up - Down

use crate::bar_indicators::momentum::aroon::Aroon;
use crate::bar_indicators::indicator_value::IndicatorValue;


#[derive(Clone)]
pub struct AroonOscillator {
    aroon: Aroon,
    value: f64,
}

impl AroonOscillator {
    pub fn new(period: usize) -> Self {
        Self {
            aroon: Aroon::new(period.max(2)),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.aroon.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.aroon.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let (_up, _down, osc) = self.aroon.update_bar(o, h, l, c, v);
        self.value = osc;
        self.value
    }

    pub fn period(&self) -> usize {
        self.aroon.period()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aroon_oscillator_creation() {
        let aroon = AroonOscillator::new(14);
        assert!(!aroon.is_ready());
        assert_eq!(aroon.value().main(), 0.0);
    }

    #[test]
    fn test_aroon_oscillator_uptrend() {
        let mut aroon = AroonOscillator::new(14);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            aroon.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(aroon.is_ready());
        // In uptrend, Oscillator = Up - Down should be positive
        assert!(aroon.value().main() > 0.0, "Aroon Oscillator should be positive in uptrend, got {}", aroon.value().main());
    }

    #[test]
    fn test_aroon_oscillator_downtrend() {
        let mut aroon = AroonOscillator::new(14);
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            aroon.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(aroon.is_ready());
        // In downtrend, Oscillator = Up - Down should be negative
        assert!(aroon.value().main() < 0.0, "Aroon Oscillator should be negative in downtrend, got {}", aroon.value().main());
    }

    #[test]
    fn test_aroon_oscillator_range() {
        let mut aroon = AroonOscillator::new(14);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = aroon.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            if aroon.is_ready() {
                // Oscillator = Up - Down, range is [-100, 100]
                assert!(value >= -100.0 && value <= 100.0, "Aroon Oscillator should be in [-100, 100], got {}", value);
            }
        }
    }

    #[test]
    fn test_aroon_oscillator_reset() {
        let mut aroon = AroonOscillator::new(14);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            aroon.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(aroon.is_ready());
        aroon.reset();
        assert!(!aroon.is_ready());
        assert_eq!(aroon.value().main(), 0.0);
    }
}
