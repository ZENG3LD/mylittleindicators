// Robust percentile of Spectral Slope using winsorized window

use crate::bar_indicators::signal_processing::spectral_slope::SpectralSlope;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::utils::math::percentile::quickselect_nth;

#[derive(Clone)]
pub struct SpectralSlopeRobustPercentile {
    inner: SpectralSlope,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralSlopeRobustPercentile {
    pub fn new(fft_window: usize, pct_window: usize) -> Self {
        let w = pct_window.max(50);
        Self {
            inner: SpectralSlope::new(fft_window),
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
            let mut w: Vec<f64> = self.buf.clone();
            let k = (0.05 * (self.window as f64)) as usize;

            // Get 5th percentile using quickselect
            let low = quickselect_nth(&mut w.clone(), k);
            // Get 95th percentile using quickselect
            let high = quickselect_nth(&mut w, self.window - 1 - k);

            let mut count = 0usize;
            let mut less = 0usize;
            for &t in &self.buf {
                let tt = t.max(low).min(high);
                if tt <= x {
                    less += 1;
                }
                count += 1;
            }
            self.value = 100.0 * (less as f64) / (count as f64);
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
    fn test_spectral_slope_robust_pct_creation() {
        let ssr = SpectralSlopeRobustPercentile::new(64, 60);
        assert!(!ssr.is_ready());
        assert_eq!(ssr.value().main(), 50.0);
        assert_eq!(ssr.window(), 60);
    }

    #[test]
    fn test_spectral_slope_robust_pct_warmup() {
        let mut ssr = SpectralSlopeRobustPercentile::new(64, 60);
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ssr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ssr.is_ready());
    }

    #[test]
    fn test_spectral_slope_robust_pct_range() {
        let mut ssr = SpectralSlopeRobustPercentile::new(64, 60);
        for i in 0..200 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ssr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 100.0, "Percentile should be in [0, 100], got {}", value);
        }
    }

    #[test]
    fn test_spectral_slope_robust_pct_reset() {
        let mut ssr = SpectralSlopeRobustPercentile::new(64, 60);
        for i in 0..200 {
            ssr.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        ssr.reset();
        assert!(!ssr.is_ready());
        assert_eq!(ssr.value().main(), 50.0);
    }
}
