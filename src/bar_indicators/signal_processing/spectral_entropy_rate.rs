// Spectral Entropy Rate: smoothed first difference of spectral entropy

use crate::bar_indicators::signal_processing::spectral_entropy::SpectralEntropy;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SpectralEntropyRate {
    inner: SpectralEntropy,
    alpha: f64, // EMA smoothing
    prev_entropy: f64,
    is_prev_set: bool,
    pub value: f64, // smoothed dH
}

impl SpectralEntropyRate {
    pub fn new(window: usize, smoothing_alpha: f64) -> Self {
        Self {
            inner: SpectralEntropy::new(window),
            alpha: smoothing_alpha.clamp(0.01, 1.0),
            prev_entropy: 0.0,
            is_prev_set: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.prev_entropy = 0.0;
        self.is_prev_set = false;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready() && self.is_prev_set
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let h_now = self.inner.update_bar(o, h, l, c, v);
        if !self.is_prev_set {
            self.prev_entropy = h_now;
            self.is_prev_set = true;
            return self.value;
        }
        let diff = h_now - self.prev_entropy;
        self.prev_entropy = h_now;
        // EMA smoothing of rate
        self.value = self.alpha * diff + (1.0 - self.alpha) * self.value;
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_entropy_rate_creation() {
        let ser = SpectralEntropyRate::new(64, 0.2);
        assert!(!ser.is_ready());
        assert_eq!(ser.value().main(), 0.0);
        assert!((ser.alpha() - 0.2).abs() < 1e-9);
    }

    #[test]
    fn test_spectral_entropy_rate_warmup() {
        let mut ser = SpectralEntropyRate::new(64, 0.2);
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ser.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ser.is_ready());
    }

    #[test]
    fn test_spectral_entropy_rate_finite() {
        let mut ser = SpectralEntropyRate::new(64, 0.2);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ser.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Entropy rate should be finite");
        }
    }

    #[test]
    fn test_spectral_entropy_rate_reset() {
        let mut ser = SpectralEntropyRate::new(64, 0.2);
        for i in 0..80 {
            ser.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        ser.reset();
        assert!(!ser.is_ready());
        assert_eq!(ser.value().main(), 0.0);
    }
}
