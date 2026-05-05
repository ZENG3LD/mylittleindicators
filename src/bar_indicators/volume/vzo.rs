// Volume Zone Oscillator (VZO) - optimized with O(1) running sum

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct Vzo {
    period: usize,
    vol_pos: ArrayVec<f64, 1024>,
    vol_neg: ArrayVec<f64, 1024>,

    // Running sums for O(1) calculation
    sum_pos: f64,
    sum_neg: f64,

    idx: usize,
    count: usize,
    value: f64,
    prev_close: f64,
    initialized: bool,
}

impl Vzo {
    pub fn new(period: usize) -> Self {
        Self {
            period: period.clamp(2, 1024),
            vol_pos: ArrayVec::new(),
            vol_neg: ArrayVec::new(),
            sum_pos: 0.0,
            sum_neg: 0.0,
            idx: 0,
            count: 0,
            value: 0.0,
            prev_close: 0.0,
            initialized: false,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.vol_pos.clear();
        self.vol_neg.clear();
        self.sum_pos = 0.0;
        self.sum_neg = 0.0;
        self.idx = 0;
        self.count = 0;
        self.value = 0.0;
        self.prev_close = 0.0;
        self.initialized = false;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, v: f64) -> f64 {
        if !self.initialized {
            self.prev_close = c;
            self.initialized = true;
        }
        let up = (c >= self.prev_close) as i32 as f64;
        let down = 1.0 - up;
        let pos_v = up * v;
        let neg_v = down * v;

        if self.count < self.period {
            // Building up initial buffer
            self.vol_pos.push(pos_v);
            self.vol_neg.push(neg_v);
            self.sum_pos += pos_v;
            self.sum_neg += neg_v;
            self.count += 1;
            self.idx = self.count % self.period;
        } else {
            // Ring buffer is full - update with O(1) running sum
            // Subtract old values
            self.sum_pos -= self.vol_pos[self.idx];
            self.sum_neg -= self.vol_neg[self.idx];

            // Update ring buffer
            self.vol_pos[self.idx] = pos_v;
            self.vol_neg[self.idx] = neg_v;

            // Add new values
            self.sum_pos += pos_v;
            self.sum_neg += neg_v;

            self.idx = (self.idx + 1) % self.period;
        }
        self.prev_close = c;

        // O(1) calculation using running sums
        let denom = (self.sum_pos + self.sum_neg).abs().max(1e-9);
        self.value = 100.0 * (self.sum_pos - self.sum_neg) / denom;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vzo_creation() {
        let vzo = Vzo::new(14);
        assert!(!vzo.is_ready());
        assert_eq!(vzo.value().main(), 0.0);
    }

    #[test]
    fn test_vzo_warmup() {
        let mut vzo = Vzo::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vzo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vzo.is_ready());
    }

    #[test]
    fn test_vzo_range() {
        let mut vzo = Vzo::new(14);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vzo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= -100.0 && value <= 100.0, "VZO should be in [-100, 100]");
        }
    }

    #[test]
    fn test_vzo_reset() {
        let mut vzo = Vzo::new(14);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            vzo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        vzo.reset();
        assert!(!vzo.is_ready());
        assert_eq!(vzo.value().main(), 0.0);
    }
}
