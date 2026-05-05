// Ehlers Sinewave - simplified streaming proxy

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct EhlersSinewave {
    alpha: f64,
    prev: f64,
    phase: f64,
    value: f64,
}

impl EhlersSinewave {
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
            prev: 0.0,
            phase: 0.0,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.prev = 0.0;
        self.phase = 0.0;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let diff = c - self.prev;
        self.prev = c;
        // crude phase increment proportional to normalized diff
        self.phase += (diff.tanh()) * self.alpha;
        self.value = self.phase.sin();
        self.value
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ehlers_sinewave_creation() {
        let sw = EhlersSinewave::new(0.5);
        assert!(sw.is_ready());
        assert_eq!(sw.value().main(), 0.0);
        assert!((sw.alpha() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_ehlers_sinewave_range() {
        let mut sw = EhlersSinewave::new(0.5);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = sw.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= -1.0 && value <= 1.0, "Sinewave should be in [-1, 1], got {}", value);
        }
    }

    #[test]
    fn test_ehlers_sinewave_reset() {
        let mut sw = EhlersSinewave::new(0.5);
        for i in 1..=20 {
            sw.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        sw.reset();
        assert_eq!(sw.value().main(), 0.0);
    }
}
