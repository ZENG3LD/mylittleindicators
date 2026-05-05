// Decycler (Ehlers) - remove cyclic components (placeholder first-order HP)

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct Decycler {
    alpha: f64,
    prev: f64,
    value: f64,
}

impl Decycler {
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
            prev: 0.0,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.prev = 0.0;
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
        // simple HP filter
        let hp = self.alpha * (self.prev + c - self.value);
        self.prev = c;
        self.value = hp;
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
    fn test_decycler_creation() {
        let dc = Decycler::new(0.5);
        assert!(dc.is_ready());
        assert_eq!(dc.value().main(), 0.0);
        assert!((dc.alpha() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_decycler_finite() {
        let mut dc = Decycler::new(0.5);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = dc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Decycler should always be finite");
        }
    }

    #[test]
    fn test_decycler_reset() {
        let mut dc = Decycler::new(0.5);
        for i in 1..=20 {
            dc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        dc.reset();
        assert_eq!(dc.value().main(), 0.0);
    }
}
