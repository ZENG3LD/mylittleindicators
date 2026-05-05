// Spectral Centroid feature from FFT

use crate::bar_indicators::signal_processing::fft::FastFourierTransform;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SpectralCentroidFeature {
    fft: FastFourierTransform,
    pub value: f64,
}

impl SpectralCentroidFeature {
    pub fn new(window: usize) -> Self {
        Self {
            fft: FastFourierTransform::new(window, 1.0),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.fft.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.fft.is_ready()
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let fd = self.fft.update(c);
        self.value = fd.spectral_centroid;
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_centroid_creation() {
        let sc = SpectralCentroidFeature::new(64);
        assert!(!sc.is_ready());
        assert_eq!(sc.value().main(), 0.0);
    }

    #[test]
    fn test_spectral_centroid_warmup() {
        let mut sc = SpectralCentroidFeature::new(64);
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sc.is_ready());
    }

    #[test]
    fn test_spectral_centroid_finite() {
        let mut sc = SpectralCentroidFeature::new(64);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Centroid should be finite");
        }
    }

    #[test]
    fn test_spectral_centroid_reset() {
        let mut sc = SpectralCentroidFeature::new(64);
        for i in 0..70 {
            sc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        sc.reset();
        assert!(!sc.is_ready());
        assert_eq!(sc.value().main(), 0.0);
    }
}
