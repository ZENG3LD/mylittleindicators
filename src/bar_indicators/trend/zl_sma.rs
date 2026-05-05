// Zero-lag LSMA (approx): linear regression + de-lag via projection

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct ZlSma {
    window: usize,
    buf: ArrayVec<f64, 512>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl ZlSma {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.clamp(2, 512),
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
            self.value = self.lr_projection();
        }
        self.value
    }

    fn lr_projection(&self) -> f64 {
        let n = self.window as f64;
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut sxx = 0.0;
        let mut sxy = 0.0;
        for (i, &y) in self.buf.iter().enumerate() {
            let x = i as f64;
            sx += x;
            sy += y;
            sxx += x * x;
            sxy += x * y;
        }
        let den = (n * sxx - sx * sx).max(1e-12);
        let b = (n * sxy - sx * sy) / den;
        let a = (sy - b * sx) / n;
        // project one step ahead to reduce lag
        let x_next = n;
        a + b * x_next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zl_sma_creation() {
        let zl = ZlSma::new(10);
        assert!(!zl.is_ready());
        assert_eq!(zl.value().main(), 0.0);
    }

    #[test]
    fn test_zl_sma_warmup() {
        let mut zl = ZlSma::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            zl.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(zl.is_ready());
    }

    #[test]
    fn test_zl_sma_values_finite() {
        let mut zl = ZlSma::new(10);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            let value = zl.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if zl.is_ready() {
                assert!(value.is_finite());
                assert!(value > 0.0);
            }
        }
    }

    #[test]
    fn test_zl_sma_reset() {
        let mut zl = ZlSma::new(10);
        for i in 0..15 {
            zl.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        zl.reset();
        assert!(!zl.is_ready());
        assert_eq!(zl.value().main(), 0.0);
    }
}
