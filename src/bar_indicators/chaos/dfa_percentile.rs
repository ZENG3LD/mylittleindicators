// DFA percentile over rolling window (uses Dfa result as feature)

use crate::bar_indicators::chaos::dfa::Dfa;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct DfaPercentile {
    inner: Dfa,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl DfaPercentile {
    pub fn new(scales: [usize; 4], window: usize) -> Self {
        let w = window.max(50);
        Self {
            inner: Dfa::new(scales),
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

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let dval = self.inner.update_bar(o, h, l, c, v);
        self.buf[self.idx] = dval;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut cnt = 0usize;
            for i in 0..self.window {
                if self.buf[i] <= dval {
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
    fn test_dfa_percentile_creation() {
        let ind = DfaPercentile::new([8, 16, 32, 64], 50);
        assert!(!ind.is_ready());
        assert_eq!(ind.value, 0.5);
    }

    #[test]
    fn test_dfa_percentile_warmup() {
        let mut ind = DfaPercentile::new([8, 16, 32, 64], 50);
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_dfa_percentile_values_range() {
        let mut ind = DfaPercentile::new([8, 16, 32, 64], 50);
        for i in 0..150 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let pct = ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(pct >= 0.0 && pct <= 1.0);
        }
    }

    #[test]
    fn test_dfa_percentile_reset() {
        let mut ind = DfaPercentile::new([8, 16, 32, 64], 50);
        for i in 0..120 {
            ind.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value, 0.5);
    }
}
