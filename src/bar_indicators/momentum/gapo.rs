// GAPO (Gopalakrishnan Range Index)

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct Gapo {
    window: usize,
    value: f64,
}

impl Gapo {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
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
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, _c: f64, _v: f64) -> f64 {
        let range = (h - l).max(1e-9);
        self.value = (range.ln()) / ((self.window as f64).ln());
        self.value
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gapo_creation() {
        let gapo = Gapo::new(14);
        assert!(gapo.is_ready()); // GAPO is always ready
        assert_eq!(gapo.value().main(), 0.0);
        assert_eq!(gapo.window(), 14);
    }

    #[test]
    fn test_gapo_min_window() {
        let gapo = Gapo::new(1);
        assert_eq!(gapo.window(), 2); // min window is 2
    }

    #[test]
    fn test_gapo_basic() {
        let mut gapo = Gapo::new(14);
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            gapo.update_bar(price, price + 5.0, price - 5.0, price, 1000.0);
        }
        // GAPO is ln(range) / ln(window), should be finite and positive for typical ranges
        assert!(gapo.value().main().is_finite());
    }

    #[test]
    fn test_gapo_reset() {
        let mut gapo = Gapo::new(14);
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            gapo.update_bar(price, price + 5.0, price - 5.0, price, 1000.0);
        }
        gapo.reset();
        assert_eq!(gapo.value().main(), 0.0);
    }

    #[test]
    fn test_gapo_finite_values() {
        let mut gapo = Gapo::new(14);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = gapo.update_bar(price, price + 3.0, price - 3.0, price, 1000.0);
            assert!(value.is_finite(), "GAPO should always be finite");
        }
    }
}
