// Percentile of Spectral Crest over rolling window

use crate::bar_indicators::signal_processing::spectral_crest::SpectralCrest;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SpectralCrestPercentile {
    inner: SpectralCrest,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralCrestPercentile {
    pub fn new(fft_window: usize, pct_window: usize) -> Self {
        let w = pct_window.max(30);
        Self {
            inner: SpectralCrest::new(fft_window),
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
        let x = self.inner.update_bar(o, h, l, c, v);
        self.buf[self.idx] = x;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut less = 0usize;
            for &t in &self.buf {
                if t <= x {
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
    fn test_spectral_crest_percentile_creation() {
        let scp = SpectralCrestPercentile::new(64, 50);
        assert!(!scp.is_ready());
        assert_eq!(scp.value().main(), 50.0);
        assert_eq!(scp.window(), 50);
    }

    #[test]
    fn test_spectral_crest_percentile_warmup() {
        let mut scp = SpectralCrestPercentile::new(64, 50);
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            scp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(scp.is_ready());
    }

    #[test]
    fn test_spectral_crest_percentile_range() {
        let mut scp = SpectralCrestPercentile::new(64, 50);
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = scp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 100.0, "Percentile should be in [0, 100], got {}", value);
        }
    }

    #[test]
    fn test_spectral_crest_percentile_reset() {
        let mut scp = SpectralCrestPercentile::new(64, 50);
        for i in 0..120 {
            scp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        scp.reset();
        assert!(!scp.is_ready());
        assert_eq!(scp.value().main(), 50.0);
    }
}
