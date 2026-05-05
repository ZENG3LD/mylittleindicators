// AVWAP touch-probability score over recent anchors

use crate::bar_indicators::levels::anchored_vwap::{AnchoredVwap, AnchoredVwapParams};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct AvwapTouchProbability {
    anchors: Vec<AnchoredVwap>,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    threshold: f64,
    pub value: f64,
}

impl AvwapTouchProbability {
    pub fn new(
        params_list: Vec<AnchoredVwapParams>,
        prob_window: usize,
        touch_threshold: f64,
    ) -> Self {
        let mut anchors = Vec::new();
        for p in params_list {
            anchors.push(AnchoredVwap::new(p));
        }
        let w = prob_window.max(20);
        Self {
            anchors,
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            threshold: touch_threshold.max(0.0),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        for a in &mut self.anchors {
            a.reset();
        }
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let mut touched = false;
        for a in &mut self.anchors {
            let vw = a.update_bar(o, h, l, c, v, 0);
            if (c - vw).abs() / vw.max(1e-9) <= self.threshold {
                touched = true;
            }
        }
        self.buf[self.idx] = if touched { 1.0 } else { 0.0 };
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut s = 0.0;
            for &x in &self.buf {
                s += x;
            }
            self.value = s / (self.window as f64);
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avwap_touch_probability_creation() {
        let params = vec![AnchoredVwapParams::default()];
        let atp = AvwapTouchProbability::new(params, 30, 0.01);
        assert!(!atp.is_ready());
        assert_eq!(atp.value, 0.0);
    }

    #[test]
    fn test_avwap_touch_probability_warmup() {
        let params = vec![AnchoredVwapParams::default()];
        let mut atp = AvwapTouchProbability::new(params, 30, 0.01);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            atp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(atp.is_ready());
    }

    #[test]
    fn test_avwap_touch_probability_range() {
        let params = vec![AnchoredVwapParams::default()];
        let mut atp = AvwapTouchProbability::new(params, 30, 0.01);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = atp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 1.0, "Probability should be in [0, 1]");
        }
    }

    #[test]
    fn test_avwap_touch_probability_reset() {
        let params = vec![AnchoredVwapParams::default()];
        let mut atp = AvwapTouchProbability::new(params, 30, 0.01);
        for i in 0..40 {
            atp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0, 1000.0);
        }
        atp.reset();
        assert!(!atp.is_ready());
        assert_eq!(atp.value, 0.0);
    }
}
