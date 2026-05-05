// Rolling Quartiles (Q1, Median, Q3) of Close over window (naive O(window) updates)

use crate::bar_indicators::utils::math::percentile::quartiles;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct RollingQuartiles {
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub q1: f64,
    pub q2: f64,
    pub q3: f64,
}

impl RollingQuartiles {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            buf: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            q1: 0.0,
            q2: 0.0,
            q3: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.q1 = 0.0;
        self.q2 = 0.0;
        self.q3 = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> (f64, f64, f64) {
        self.buf[self.idx] = close;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            // 🚀 O(n) quartiles function instead of O(n log n) sorting
            let mut tmp = self.buf[..len].to_vec();
            let (q1, q2, q3) = quartiles(&mut tmp);
            self.q1 = q1;
            self.q2 = q2;
            self.q3 = q3;
        }
        (self.q1, self.q2, self.q3)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.q1, self.q2, self.q3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_quartiles_creation() {
        let rq = RollingQuartiles::new(20);
        assert!(!rq.is_ready());
        assert_eq!(rq.q1, 0.0);
        assert_eq!(rq.q2, 0.0);
        assert_eq!(rq.q3, 0.0);
    }

    #[test]
    fn test_rolling_quartiles_warmup() {
        let mut rq = RollingQuartiles::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rq.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rq.is_ready());
    }

    #[test]
    fn test_rolling_quartiles_order() {
        let mut rq = RollingQuartiles::new(20);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let (q1, q2, q3) = rq.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            // Q1 <= Q2 <= Q3
            assert!(q1 <= q2, "Q1 should be <= Q2");
            assert!(q2 <= q3, "Q2 should be <= Q3");
        }
    }

    #[test]
    fn test_rolling_quartiles_reset() {
        let mut rq = RollingQuartiles::new(20);
        for i in 0..25 {
            rq.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        rq.reset();
        assert!(!rq.is_ready());
        assert_eq!(rq.q1, 0.0);
        assert_eq!(rq.q2, 0.0);
        assert_eq!(rq.q3, 0.0);
    }
}
