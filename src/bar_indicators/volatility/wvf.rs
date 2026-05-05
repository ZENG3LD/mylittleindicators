// Williams VIX Fix (WVF)

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct Wvf {
    lookback: usize,
    closes: ArrayVec<f64, 1024>,
    lows: ArrayVec<f64, 1024>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl Wvf {
    pub fn new(lookback: usize) -> Self {
        let lb = lookback.clamp(2, 1024);
        Self {
            lookback: lb,
            closes: ArrayVec::new(),
            lows: ArrayVec::new(),
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.closes.clear();
        self.lows.clear();
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
    pub fn update_bar(&mut self, _o: f64, _h: f64, l: f64, c: f64, _v: f64) -> f64 {
        if self.closes.len() < self.lookback {
            self.closes.push(c);
            self.lows.push(l);
            if self.closes.len() == self.lookback {
                self.filled = true;
            }
        } else {
            self.closes[self.idx] = c;
            self.lows[self.idx] = l;
        }
        self.idx = (self.idx + 1) % self.lookback;
        if self.is_ready() {
            let highest_close = self
                .closes
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max);
            if highest_close.abs() > 1e-12 {
                self.value = (highest_close - l) / highest_close * 100.0;
            }
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wvf_creation() {
        let wvf = Wvf::new(22);
        assert!(!wvf.is_ready());
        assert_eq!(wvf.value().main(), 0.0);
    }

    #[test]
    fn test_wvf_warmup() {
        let mut wvf = Wvf::new(22);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            wvf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(wvf.is_ready());
    }

    #[test]
    fn test_wvf_non_negative() {
        let mut wvf = Wvf::new(22);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = wvf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "WVF should be non-negative");
        }
    }

    #[test]
    fn test_wvf_reset() {
        let mut wvf = Wvf::new(22);
        for i in 0..25 {
            wvf.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        wvf.reset();
        assert!(!wvf.is_ready());
        assert_eq!(wvf.value().main(), 0.0);
    }
}
