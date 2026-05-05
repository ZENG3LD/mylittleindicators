// Hurst exponent percentile over rolling window

use crate::bar_indicators::chaos::hurst_exponent::HurstExponent;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct HurstPercentile {
    inner: HurstExponent,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl HurstPercentile {
    pub fn new(window: usize) -> Self {
        let w = window.max(50);
        Self {
            inner: HurstExponent::new(w),
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.5,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.5;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.inner.is_ready()
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let hval = self.inner.update(c);
        self.buf[self.idx] = hval;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut cnt = 0usize;
            for i in 0..self.window {
                if self.buf[i] <= hval {
                    cnt += 1;
                }
            }
            self.value = (cnt as f64) / (self.window as f64);
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hurst_percentile_creation() {
        let ind = HurstPercentile::new(50);
        assert!(!ind.is_ready());
        assert_eq!(ind.value, 0.5);
    }

    #[test]
    fn test_hurst_percentile_warmup() {
        let mut ind = HurstPercentile::new(50);
        // Need: inner HurstExponent to be ready + percentile buffer to fill
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_hurst_percentile_values_range() {
        let mut ind = HurstPercentile::new(50);
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let pct = ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(pct >= 0.0 && pct <= 1.0);
        }
    }

    #[test]
    fn test_hurst_percentile_reset() {
        let mut ind = HurstPercentile::new(50);
        for i in 0..100 {
            ind.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value, 0.5);
    }
}
