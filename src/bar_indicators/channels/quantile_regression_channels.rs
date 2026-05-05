// Quantile Regression Channels - robust channel via quantile lines
// Placeholder: compute rolling median and MAD-based bands as proxy

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::utils::math::percentile::median;
use arrayvec::ArrayVec;

#[derive(Debug, Clone)]
pub struct QuantileRegressionChannels {
    window: usize,
    k: f64,
    buf: ArrayVec<f64, 512>,
    idx: usize,
    filled: bool,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl QuantileRegressionChannels {
    pub fn new(window: usize, k: f64) -> Self {
        Self {
            window: window.clamp(2, 512),
            k: if k > 0.0 { k } else { 2.0 },
            buf: ArrayVec::new(),
            idx: 0,
            filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }
    pub fn reset(&mut self) {
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.upper,
            middle: self.middle,
            lower: self.lower,
        }
    }

    #[inline]
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper, self.middle, self.lower)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> (f64, f64, f64) {
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
            let mid = median(&mut v);
            self.middle = mid;

            // Optimized: Use O(n) quickselect for MAD instead of O(n log n) full sort
            let mut dev: Vec<f64> = self.buf.iter().map(|x| (x - mid).abs()).collect();
            let mad = median(&mut dev);

            let sigma = 1.4826 * mad; // approx std from MAD
            self.upper = mid + self.k * sigma;
            self.lower = mid - self.k * sigma;
        }
        (self.upper, self.middle, self.lower)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantile_regression_channels_creation() {
        let qrc = QuantileRegressionChannels::new(20, 2.0);
        assert!(!qrc.is_ready());
        assert_eq!(qrc.upper, 0.0);
        assert_eq!(qrc.lower, 0.0);
    }

    #[test]
    fn test_quantile_regression_channels_warmup() {
        let mut qrc = QuantileRegressionChannels::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            qrc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(qrc.is_ready());
    }

    #[test]
    fn test_quantile_regression_channels_values() {
        let mut qrc = QuantileRegressionChannels::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            qrc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(qrc.upper >= qrc.middle);
        assert!(qrc.middle >= qrc.lower);
    }

    #[test]
    fn test_quantile_regression_channels_reset() {
        let mut qrc = QuantileRegressionChannels::new(20, 2.0);
        for i in 0..25 {
            qrc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        qrc.reset();
        assert!(!qrc.is_ready());
        assert_eq!(qrc.upper, 0.0);
        assert_eq!(qrc.lower, 0.0);
    }
}
