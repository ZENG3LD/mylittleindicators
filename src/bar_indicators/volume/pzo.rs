// Price Zone Oscillator (PZO) - optimized with O(1) running sum

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct Pzo {
    period: usize,
    pos: ArrayVec<f64, 1024>,
    neg: ArrayVec<f64, 1024>,

    // Running sums for O(1) calculation
    sum_pos: f64,
    sum_neg: f64,

    idx: usize,
    count: usize,
    value: f64,
    prev_close: f64,
    initialized: bool,
}

impl Pzo {
    pub fn new(period: usize) -> Self {
        Self {
            period: period.clamp(2, 1024),
            pos: ArrayVec::new(),
            neg: ArrayVec::new(),
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
        self.pos.clear();
        self.neg.clear();
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
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        if !self.initialized {
            self.prev_close = c;
            self.initialized = true;
        }
        let diff = c - self.prev_close;
        let pos_v = diff.max(0.0);
        let neg_v = (-diff).max(0.0);

        if self.count < self.period {
            // Building up initial buffer
            self.pos.push(pos_v);
            self.neg.push(neg_v);
            self.sum_pos += pos_v;
            self.sum_neg += neg_v;
            self.count += 1;
            self.idx = self.count % self.period;
        } else {
            // Ring buffer is full - update with O(1) running sum
            // Subtract old values
            self.sum_pos -= self.pos[self.idx];
            self.sum_neg -= self.neg[self.idx];

            // Update ring buffer
            self.pos[self.idx] = pos_v;
            self.neg[self.idx] = neg_v;

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
    fn test_pzo_creation() {
        let pzo = Pzo::new(14);
        assert!(!pzo.is_ready());
        assert_eq!(pzo.value().main(), 0.0);
    }

    #[test]
    fn test_pzo_warmup() {
        let mut pzo = Pzo::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pzo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pzo.is_ready());
    }

    #[test]
    fn test_pzo_values_finite() {
        let mut pzo = Pzo::new(14);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pzo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "PZO should be finite");
        }
    }

    #[test]
    fn test_pzo_reset() {
        let mut pzo = Pzo::new(14);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            pzo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        pzo.reset();
        assert!(!pzo.is_ready());
        assert_eq!(pzo.value().main(), 0.0);
    }
}
