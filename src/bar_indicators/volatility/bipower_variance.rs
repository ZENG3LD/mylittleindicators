// Realized Bipower Variance (RBV) - jump-robust volatility measure

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct BipowerVariance {
    window: usize,
    abs_ret: Vec<f64>,
    idx: usize,
    filled: bool,
    prev_close: f64,
    value: f64,
}

impl BipowerVariance {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            abs_ret: vec![0.0; window.max(2)],
            idx: 0,
            filled: false,
            prev_close: 0.0,
            value: 0.0,
        }
    }
    pub fn reset(&mut self) {
        self.abs_ret.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.prev_close = 0.0;
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
        if self.prev_close <= 0.0 {
            self.prev_close = c.max(1e-12);
            return self.value;
        }
        let r = (c / self.prev_close).ln();
        self.prev_close = c.max(1e-12);
        let ar = r.abs();
        let _old = self.abs_ret[self.idx];
        self.abs_ret[self.idx] = ar;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        // RBV ~ (π/2) * sum(|r_t||r_{t-1}|) / (N-1)
        if self.idx > 0 || self.filled {
            let n = if self.filled { self.window } else { self.idx };
            let mut s = 0.0;
            let len = if self.filled { self.window } else { self.idx };
            for i in 1..len {
                s += self.abs_ret[i] * self.abs_ret[i - 1];
            }
            let denom = (n as f64 - 1.0).max(1.0);
            // Аннуализируем и масштабируем для лучшей читаемости (252 торговых дня)
            self.value = std::f64::consts::PI * 0.5 * (s / denom) * 252.0 * 10000.0;
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bipower_variance_creation() {
        let bv = BipowerVariance::new(20);
        assert!(!bv.is_ready());
        assert_eq!(bv.value().main(), 0.0);
    }

    #[test]
    fn test_bipower_variance_warmup() {
        let mut bv = BipowerVariance::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            bv.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(bv.is_ready());
    }

    #[test]
    fn test_bipower_variance_positive() {
        let mut bv = BipowerVariance::new(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            bv.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(bv.value().main() >= 0.0);
    }

    #[test]
    fn test_bipower_variance_reset() {
        let mut bv = BipowerVariance::new(20);
        for i in 0..25 {
            bv.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        bv.reset();
        assert!(!bv.is_ready());
        assert_eq!(bv.value().main(), 0.0);
    }
}
