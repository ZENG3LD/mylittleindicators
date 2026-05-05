// Convenience wrapper around SpectralRolloff with target 0.95

use crate::bar_indicators::signal_processing::spectral_rolloff::SpectralRolloff;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SpectralRolloff95 {
    inner: SpectralRolloff,
}

impl SpectralRolloff95 {
    pub fn new(window: usize) -> Self {
        Self {
            inner: SpectralRolloff::new(window, 0.95),
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }
    #[inline]
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        self.inner.update_bar(o, h, l, c, v)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        self.inner.value()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_rolloff_95_creation() {
        let sr = SpectralRolloff95::new(64);
        assert!(!sr.is_ready());
        assert_eq!(sr.value().main(), 0.0);
    }

    #[test]
    fn test_spectral_rolloff_95_warmup() {
        let mut sr = SpectralRolloff95::new(64);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sr.is_ready());
    }

    #[test]
    fn test_spectral_rolloff_95_finite() {
        let mut sr = SpectralRolloff95::new(64);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Rolloff95 should be finite");
        }
    }

    #[test]
    fn test_spectral_rolloff_95_reset() {
        let mut sr = SpectralRolloff95::new(64);
        for i in 0..70 {
            sr.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        sr.reset();
        assert!(!sr.is_ready());
        assert_eq!(sr.value().main(), 0.0);
    }
}
