// R-Squared (coefficient of determination) via rolling linear regression proxy

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct RSquared {
    window: usize,
    buf: ArrayVec<f64, 1024>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl RSquared {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.clamp(5, 1024),
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
            let syy: f64 = self.buf.iter().map(|&y| y * y).sum();
            let num = (n * sxy - sx * sy).powi(2);
            let den = (n * sxx - sx * sx) * (n * syy - sy * sy);
            self.value = if den.abs() > 1e-12 { num / den } else { 0.0 };
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_r_squared_creation() {
        let r2 = RSquared::new(20);
        assert!(!r2.is_ready());
        assert_eq!(r2.value().main(), 0.0);
    }

    #[test]
    fn test_r_squared_warmup() {
        let mut r2 = RSquared::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            r2.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(r2.is_ready());
    }

    #[test]
    fn test_r_squared_range() {
        let mut r2 = RSquared::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = r2.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 1.0, "R^2 should be in [0, 1]");
        }
    }

    #[test]
    fn test_r_squared_reset() {
        let mut r2 = RSquared::new(20);
        for i in 0..25 {
            r2.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        r2.reset();
        assert!(!r2.is_ready());
        assert_eq!(r2.value().main(), 0.0);
    }
}
