// Hampel Filter - robust outlier smoother using median and MAD

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::utils::math::percentile::median;

#[derive(Debug, Clone)]
pub struct HampelFilter {
    window: usize,
    k: f64,
    buf: ArrayVec<f64, 512>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl HampelFilter {
    pub fn new(window: usize, k: f64) -> Self {
        Self {
            window: window.clamp(3, 512),
            k: if k > 0.0 { k } else { 3.0 },
            buf: ArrayVec::new(),
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        if self.buf.len() < self.window {
            self.buf.push(c);
            if self.buf.len() == self.window {
                self.filled = true;
            }
        } else {
            self.buf[self.idx] = c;
        }
        self.idx = (self.idx + 1) % self.window;
        if self.is_ready() {
            // Optimized: Use O(n) quickselect for median instead of O(n log n) full sort
            let mut v: Vec<f64> = self.buf.iter().copied().collect();
            let med = median(&mut v);

            // Optimized: Use O(n) quickselect for MAD instead of O(n log n) full sort
            let mut dev: Vec<f64> = self.buf.iter().map(|x| (x - med).abs()).collect();
            let mad = median(&mut dev);

            let sigma = 1.4826 * mad;
            let x = c;
            let z = (x - med) / sigma.max(1e-9);
            self.value = if z.abs() > self.k {
                med + self.k * sigma * z.signum()
            } else {
                x
            };
        }
        self.value
    }

    pub fn window(&self) -> usize {
        self.window
    }

    pub fn k(&self) -> f64 {
        self.k
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hampel_creation() {
        let hf = HampelFilter::new(10, 3.0);
        assert!(!hf.is_ready());
        assert_eq!(hf.window(), 10);
        assert!((hf.k() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_hampel_outlier_handling() {
        let mut hf = HampelFilter::new(10, 3.0);
        // Fill with normal values
        for i in 0..10 {
            hf.update_bar(100.0, 101.0, 99.0, 100.0 + (i as f64 * 0.1), 1000.0);
        }
        assert!(hf.is_ready());
        // Add an outlier
        let outlier_result = hf.update_bar(100.0, 101.0, 99.0, 200.0, 1000.0);
        // The filter should clamp the outlier
        assert!(outlier_result < 200.0, "Hampel should clamp outliers");
    }

    #[test]
    fn test_hampel_finite() {
        let mut hf = HampelFilter::new(10, 3.0);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = hf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Hampel should always be finite");
        }
    }

    #[test]
    fn test_hampel_reset() {
        let mut hf = HampelFilter::new(10, 3.0);
        for i in 1..=20 {
            hf.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        assert!(hf.is_ready());
        hf.reset();
        assert!(!hf.is_ready());
    }
}
