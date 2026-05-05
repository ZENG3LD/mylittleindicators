// Percentile of Spectral Rolloff over rolling window

use crate::bar_indicators::signal_processing::spectral_rolloff::SpectralRolloff;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SpectralRolloffPercentile {
    inner: SpectralRolloff,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralRolloffPercentile {
    pub fn new(fft_window: usize, pct_window: usize, rolloff_percent: f64) -> Self {
        let w = pct_window.clamp(30, 4096);
        Self {
            inner: SpectralRolloff::new(fft_window, rolloff_percent),
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 50.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 50.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.inner.is_ready()
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let r = self.inner.update_bar(o, h, l, c, v);
        self.buf[self.idx] = r;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut less = 0usize;
            for &x in &self.buf {
                if x <= r {
                    less += 1;
                }
            }
            self.value = 100.0 * (less as f64) / (self.window as f64);
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
    fn test_spectral_rolloff_pct_creation() {
        let srp = SpectralRolloffPercentile::new(64, 50, 0.85);
        assert!(!srp.is_ready());
        assert_eq!(srp.value().main(), 50.0);
        assert_eq!(srp.window(), 50);
    }

    #[test]
    fn test_spectral_rolloff_pct_warmup() {
        let mut srp = SpectralRolloffPercentile::new(64, 50, 0.85);
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            srp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(srp.is_ready());
    }

    #[test]
    fn test_spectral_rolloff_pct_range() {
        let mut srp = SpectralRolloffPercentile::new(64, 50, 0.85);
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = srp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 100.0, "Percentile should be in [0, 100], got {}", value);
        }
    }

    #[test]
    fn test_spectral_rolloff_pct_reset() {
        let mut srp = SpectralRolloffPercentile::new(64, 50, 0.85);
        for i in 0..120 {
            srp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        srp.reset();
        assert!(!srp.is_ready());
        assert_eq!(srp.value().main(), 50.0);
    }
}
