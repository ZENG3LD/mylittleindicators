// Lempel–Ziv complexity (binaryized returns sign sequence, rolling window)

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct LempelZivComplexity {
    window: usize,
    bits: Vec<u8>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    pub value: f64,
}

impl LempelZivComplexity {
    pub fn new(window: usize) -> Self {
        let w = window.max(32);
        Self {
            window: w,
            bits: vec![0; w],
            idx: 0,
            filled: false,
            last_close: None,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.bits.fill(0);
        self.last_close = None;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        if let Some(p) = self.last_close {
            let r = c - p;
            self.bits[self.idx] = if r >= 0.0 { 1 } else { 0 };
            self.idx = (self.idx + 1) % self.window;
            if !self.filled && self.idx == 0 {
                self.filled = true;
            }
        }
        self.last_close = Some(c);
        if self.filled {
            self.value = self.compute_lz();
        }
        self.value
    }

    fn compute_lz(&self) -> f64 {
        // LZ76 complexity: count number of distinct substrings encountered in parsing
        let n = self.window;
        let mut i = 0;
        let mut c = 1;
        let mut l = 1;
        let mut k = 1;
        let mut k_max = 1;
        let s = &self.bits;
        while i + l <= n {
            if i + k >= n || s[i + k] != s[k - 1] {
                if k > k_max {
                    k_max = k;
                }
                i += 1;
                if i == k {
                    c += 1;
                    k += k_max;
                    if k >= n {
                        break;
                    }
                    i = 0;
                    l = 1;
                    k_max = 1;
                    continue;
                }
                k = 1;
                l = 1;
            } else {
                k += 1;
                l += 1;
            }
        }
        // normalize by n/log n
        let n_f = n as f64;
        let norm = n_f / n_f.ln().max(1.0001);
        c as f64 / norm
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
    fn test_lempel_ziv_creation() {
        let lz = LempelZivComplexity::new(64);
        assert!(!lz.is_ready());
        assert_eq!(lz.value().main(), 0.0);
        assert_eq!(lz.window(), 64);
    }

    #[test]
    fn test_lempel_ziv_warmup() {
        let mut lz = LempelZivComplexity::new(32);
        for i in 0..33 {
            let price = 100.0 + (i as f64);
            lz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(lz.is_ready());
    }

    #[test]
    fn test_lempel_ziv_finite() {
        let mut lz = LempelZivComplexity::new(32);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            let value = lz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "LZ complexity should be finite");
        }
    }

    #[test]
    fn test_lempel_ziv_reset() {
        let mut lz = LempelZivComplexity::new(32);
        for i in 0..50 {
            lz.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        lz.reset();
        assert!(!lz.is_ready());
        assert_eq!(lz.value().main(), 0.0);
    }
}
