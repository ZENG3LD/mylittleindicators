// Theil–Sen Regression Channels (robust slope via median of slopes) - placeholder O(N^2) simplified

use crate::bar_indicators::indicator_value::IndicatorValue;
use arrayvec::ArrayVec;

#[derive(Debug, Clone)]
pub struct TheilSenChannels {
    window: usize,
    k: f64,
    buf: ArrayVec<f64, 256>,
    idx: usize,
    filled: bool,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl TheilSenChannels {
    pub fn new(window: usize, k: f64) -> Self {
        Self {
            window: window.clamp(5, 256),
            k: if k > 0.0 { k } else { 2.0 },
            buf: ArrayVec::new(),
            idx: 0,
            filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }
    #[inline]
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
            let n = self.window;
            // median slope
            let mut slopes: Vec<f64> = Vec::with_capacity(n * n);
            for i in 0..n {
                for j in (i + 1)..n {
                    let dy = self.buf[j] - self.buf[i];
                    let dx = (j as f64 - i as f64).max(1e-9);
                    slopes.push(dy / dx);
                }
            }
            slopes.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let slope = slopes[slopes.len() / 2];
            // intercept as median of y - slope*x
            let mut intercepts: Vec<f64> = Vec::with_capacity(n);
            for i in 0..n {
                let x = i as f64;
                let y = self.buf[i];
                intercepts.push(y - slope * x);
            }
            intercepts.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let a = intercepts[intercepts.len() / 2];
            let mid = a + slope * (n as f64 - 1.0);
            self.middle = mid;
            // robust spread via MAD
            let mut dev: Vec<f64> = self
                .buf
                .iter()
                .enumerate()
                .map(|(i, &y)| (y - (a + slope * (i as f64))).abs())
                .collect();
            dev.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mad = dev[dev.len() / 2];
            let sigma = 1.4826 * mad;
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
    fn test_theil_sen_channels_creation() {
        let tsc = TheilSenChannels::new(20, 2.0);
        assert!(!tsc.is_ready());
        assert_eq!(tsc.upper, 0.0);
        assert_eq!(tsc.lower, 0.0);
    }

    #[test]
    fn test_theil_sen_channels_warmup() {
        let mut tsc = TheilSenChannels::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            tsc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(tsc.is_ready());
    }

    #[test]
    fn test_theil_sen_channels_values() {
        let mut tsc = TheilSenChannels::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            tsc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(tsc.upper >= tsc.middle);
        assert!(tsc.middle >= tsc.lower);
    }

    #[test]
    fn test_theil_sen_channels_reset() {
        let mut tsc = TheilSenChannels::new(20, 2.0);
        for i in 0..25 {
            tsc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        tsc.reset();
        assert!(!tsc.is_ready());
        assert_eq!(tsc.upper, 0.0);
        assert_eq!(tsc.lower, 0.0);
    }
}
