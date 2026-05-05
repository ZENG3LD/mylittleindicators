// Z-score of Spectral Slope over rolling window

use crate::bar_indicators::signal_processing::spectral_slope::SpectralSlope;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SpectralSlopeZscore {
    inner: SpectralSlope,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralSlopeZscore {
    pub fn new(fft_window: usize, z_window: usize) -> Self {
        let w = z_window.clamp(20, 2048);
        Self {
            inner: SpectralSlope::new(fft_window),
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.inner.is_ready()
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let slope = self.inner.update_bar(o, h, l, c, v);
        let n = self.window;
        self.buf[self.idx] = slope;
        self.idx = (self.idx + 1) % n;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut mean = 0.0;
            for i in 0..n {
                mean += self.buf[i];
            }
            mean /= n as f64;
            let mut var = 0.0;
            for i in 0..n {
                let d = self.buf[i] - mean;
                var += d * d;
            }
            let std = (var / (n as f64)).sqrt().max(1e-9);
            self.value = (slope - mean) / std;
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spectral_slope_zscore_creation() {
        let ssz = SpectralSlopeZscore::new(64, 30);
        assert!(!ssz.is_ready());
        assert_eq!(ssz.value().main(), 0.0);
        assert_eq!(ssz.window(), 30);
    }

    #[test]
    fn test_spectral_slope_zscore_warmup() {
        let mut ssz = SpectralSlopeZscore::new(64, 30);
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ssz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ssz.is_ready());
    }

    #[test]
    fn test_spectral_slope_zscore_finite() {
        let mut ssz = SpectralSlopeZscore::new(64, 30);
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ssz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Z-score should be finite");
        }
    }

    #[test]
    fn test_spectral_slope_zscore_reset() {
        let mut ssz = SpectralSlopeZscore::new(64, 30);
        for i in 0..200 {
            ssz.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        ssz.reset();
        assert!(!ssz.is_ready());
        assert_eq!(ssz.value().main(), 0.0);
    }
}
