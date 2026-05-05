// Percentile rank of Spectral Flatness over window

use crate::bar_indicators::signal_processing::spectral_flatness::SpectralFlatness;
use crate::bar_indicators::indicator_value::IndicatorValue;

pub struct SpectralFlatnessPercentile {
    inner: SpectralFlatness,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl SpectralFlatnessPercentile {
    pub fn new(fft_window: usize, pct_window: usize) -> Self {
        let w = pct_window.max(10);
        Self {
            inner: SpectralFlatness::new(fft_window),
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
        let x = self.inner.update_bar(o, h, l, c, v);
        self.buf[self.idx] = x;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            let mut le = 0usize;
            for i in 0..len {
                if self.buf[i] <= x {
                    le += 1;
                }
            }
            self.value = (le as f64) / (len as f64);
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
    fn test_spectral_flatness_pct_creation() {
        let sfp = SpectralFlatnessPercentile::new(64, 30);
        assert!(!sfp.is_ready());
        assert_eq!(sfp.value().main(), 0.0);
        assert_eq!(sfp.window(), 30);
    }

    #[test]
    fn test_spectral_flatness_pct_warmup() {
        let mut sfp = SpectralFlatnessPercentile::new(64, 30);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sfp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sfp.is_ready());
    }

    #[test]
    fn test_spectral_flatness_pct_range() {
        let mut sfp = SpectralFlatnessPercentile::new(64, 30);
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sfp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 1.0, "Percentile should be in [0, 1], got {}", value);
        }
    }

    #[test]
    fn test_spectral_flatness_pct_reset() {
        let mut sfp = SpectralFlatnessPercentile::new(64, 30);
        for i in 0..100 {
            sfp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        sfp.reset();
        assert!(!sfp.is_ready());
        assert_eq!(sfp.value().main(), 0.0);
    }
}
