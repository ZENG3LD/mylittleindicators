// Linear Regression slope over rolling window

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct LrSlope {
    window: usize,
    buf: ArrayVec<f64, 1024>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl LrSlope {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.clamp(2, 1024),
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
            let n = self.window as f64;
            let sx: f64 = (0..self.window).map(|i| i as f64).sum();
            let sy: f64 = self.buf.iter().sum();
            let sxx: f64 = (0..self.window).map(|i| (i as f64).powi(2)).sum();
            let sxy: f64 = (0..self.window).map(|i| (i as f64) * self.buf[i]).sum();
            let den = n * sxx - sx * sx;
            self.value = if den.abs() > 1e-12 {
                (n * sxy - sx * sy) / den
            } else {
                0.0
            };
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lr_slope_creation() {
        let lr = LrSlope::new(10);
        assert!(!lr.is_ready());
        assert_eq!(lr.value().main(), 0.0);
    }

    #[test]
    fn test_lr_slope_warmup() {
        let mut lr = LrSlope::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            lr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(lr.is_ready());
    }

    #[test]
    fn test_lr_slope_values_finite() {
        let mut lr = LrSlope::new(10);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            let value = lr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if lr.is_ready() {
                assert!(value.is_finite());
            }
        }
    }

    #[test]
    fn test_lr_slope_reset() {
        let mut lr = LrSlope::new(10);
        for i in 0..15 {
            lr.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        lr.reset();
        assert!(!lr.is_ready());
        assert_eq!(lr.value().main(), 0.0);
    }
}
