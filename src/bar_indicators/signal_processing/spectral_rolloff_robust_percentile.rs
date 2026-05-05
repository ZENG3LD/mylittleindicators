// Robust percentile of Spectral Rolloff using winsorized window

use crate::bar_indicators::signal_processing::spectral_rolloff::SpectralRolloff;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::utils::math::percentile::quickselect_nth;

pub struct SpectralRolloffRobustPercentile {
    inner: SpectralRolloff,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralRolloffRobustPercentile {
    pub fn new(fft_window: usize, pct_window: usize, rolloff_percent: f64) -> Self {
        let w = pct_window.max(50);
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
        let x = self.inner.update_bar(o, h, l, c, v);
        self.buf[self.idx] = x;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            // Optimized: Use O(n) quickselect for 5th and 95th percentiles instead of O(n log n) full sort
            let mut w = self.buf.clone();
            let k = (0.05 * (self.window as f64)) as usize;

            // Get 5th percentile using quickselect
            let low = quickselect_nth(&mut w.clone(), k);
            // Get 95th percentile using quickselect
            let high = quickselect_nth(&mut w, self.window - 1 - k);

            let mut less = 0usize;
            for &t in &self.buf {
                let tt = t.max(low).min(high);
                if tt <= x {
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
    fn test_spectral_rolloff_robust_creation() {
        let srr = SpectralRolloffRobustPercentile::new(64, 50, 0.85);
        assert!(!srr.is_ready());
        assert_eq!(srr.value().main(), 50.0);
        assert_eq!(srr.window(), 50);
    }

    #[test]
    fn test_spectral_rolloff_robust_warmup() {
        let mut srr = SpectralRolloffRobustPercentile::new(64, 50, 0.85);
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            srr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(srr.is_ready());
    }

    #[test]
    fn test_spectral_rolloff_robust_range() {
        let mut srr = SpectralRolloffRobustPercentile::new(64, 50, 0.85);
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = srr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 100.0, "Percentile should be in [0, 100], got {}", value);
        }
    }

    #[test]
    fn test_spectral_rolloff_robust_reset() {
        let mut srr = SpectralRolloffRobustPercentile::new(64, 50, 0.85);
        for i in 0..120 {
            srr.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        srr.reset();
        assert!(!srr.is_ready());
        assert_eq!(srr.value().main(), 50.0);
    }
}
