// Spectral Rolloff: frequency where cumulative power reaches target fraction (e.g., 85%)

use crate::bar_indicators::signal_processing::fft::FastFourierTransform;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SpectralRolloff {
    window: usize,
    target: f64,
    fft: FastFourierTransform,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64, // frequency in [0, Nyquist]
}

impl SpectralRolloff {
    pub fn new(window: usize, target_fraction: f64) -> Self {
        let w = window.clamp(16, 256);
        let t = target_fraction.clamp(0.01, 0.99);
        Self {
            window: w,
            target: t,
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
            let mut total = 0.0;
            for i in 0..fd.power_spectrum.len() {
                total += fd.power_spectrum[i];
            }
            if total > 0.0 {
                let mut cum = 0.0;
                let mut f = 0.0;
                for i in 0..fd.power_spectrum.len() {
                    cum += fd.power_spectrum[i];
                    if cum / total >= self.target {
                        f = if i < fd.frequencies.len() {
                            fd.frequencies[i]
                        } else {
                            0.0
                        };
                        break;
                    }
                }
                self.value = f;
            } else {
                self.value = 0.0;
            }
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
    fn test_spectral_rolloff_creation() {
        let sr = SpectralRolloff::new(64, 0.85);
        assert!(!sr.is_ready());
        assert_eq!(sr.value().main(), 0.0);
        assert_eq!(sr.window(), 64);
    }

    #[test]
    fn test_spectral_rolloff_warmup() {
        let mut sr = SpectralRolloff::new(64, 0.85);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sr.is_ready());
    }

    #[test]
    fn test_spectral_rolloff_finite() {
        let mut sr = SpectralRolloff::new(64, 0.85);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Rolloff should be finite");
        }
    }

    #[test]
    fn test_spectral_rolloff_reset() {
        let mut sr = SpectralRolloff::new(64, 0.85);
        for i in 0..70 {
            sr.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        sr.reset();
        assert!(!sr.is_ready());
        assert_eq!(sr.value().main(), 0.0);
    }
}
