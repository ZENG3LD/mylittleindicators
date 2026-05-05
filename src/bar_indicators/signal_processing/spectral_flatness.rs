// Spectral Flatness (Wiener entropy): geometric mean / arithmetic mean of power spectrum

use crate::bar_indicators::signal_processing::fft::FastFourierTransform;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SpectralFlatness {
    window: usize,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralFlatness {
    pub fn new(window: usize) -> Self {
        let w = window.clamp(16, 256);
        Self {
            window: w,
            fft: FastFourierTransform::new(w, 1.0),
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.buf.fill(0.0);
        self.value = 0.0;
        self.fft.reset();
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.fft.is_ready()
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let n = self.window;
        self.buf[self.idx] = c;
        self.idx = (self.idx + 1) % n;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            // demean and feed
            let mut mean = 0.0;
            for i in 0..n {
                mean += self.buf[i];
            }
            mean /= n as f64;
            for i in 0..n {
                let x = self.buf[(self.idx + i) % n] - mean;
                self.fft.update(x);
            }
            let fd = self.fft.frequency_domain();
            let k = fd.power_spectrum.len().max(1);
            let mut am = 0.0;
            let mut lg = 0.0;
            for i in 0..fd.power_spectrum.len() {
                let p = fd.power_spectrum[i].max(1e-18);
                am += p;
                lg += p.ln();
            }
            let amean = am / k as f64;
            let gmean = (lg / k as f64).exp();
            self.value = if amean > 0.0 {
                (gmean / amean).clamp(0.0, 1.0)
            } else {
                0.0
            };
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
    fn test_spectral_flatness_creation() {
        let sf = SpectralFlatness::new(64);
        assert!(!sf.is_ready());
        assert_eq!(sf.value().main(), 0.0);
        assert_eq!(sf.window(), 64);
    }

    #[test]
    fn test_spectral_flatness_warmup() {
        let mut sf = SpectralFlatness::new(64);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sf.is_ready());
    }

    #[test]
    fn test_spectral_flatness_range() {
        let mut sf = SpectralFlatness::new(64);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if sf.is_ready() {
                assert!(value >= 0.0 && value <= 1.0, "Flatness should be in [0, 1], got {}", value);
            }
        }
    }

    #[test]
    fn test_spectral_flatness_reset() {
        let mut sf = SpectralFlatness::new(64);
        for i in 0..70 {
            sf.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        sf.reset();
        assert!(!sf.is_ready());
        assert_eq!(sf.value().main(), 0.0);
    }
}
