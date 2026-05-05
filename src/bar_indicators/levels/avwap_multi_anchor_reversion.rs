// Multi-anchor AVWAP reversion score: distance to nearest AVWAP among multiple anchors (z-normalized)

use crate::bar_indicators::levels::anchored_vwap::{AnchoredVwap, AnchoredVwapParams};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct AvwapMultiAnchorReversion {
    anchors: Vec<AnchoredVwap>,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl AvwapMultiAnchorReversion {
    pub fn new(params_list: Vec<AnchoredVwapParams>, z_window: usize) -> Self {
        let mut anchors = Vec::new();
        for p in params_list {
            anchors.push(AnchoredVwap::new(p));
        }
        let w = z_window.max(20);
        Self {
            anchors,
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
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
        self.filled && !self.anchors.is_empty()
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let mut best = std::f64::INFINITY;
        let mut any = false;
        // NOTE: requires unix_time_secs; for proxy use 0 here or integrate timestamp upstream
        for a in &mut self.anchors {
            let vw = a.update_bar(o, h, l, c, v, 0);
            let dist = (c - vw).abs();
            if dist < best {
                best = dist;
                any = true;
            }
        }
        let d = if any { best } else { 0.0 };
        self.buf[self.idx] = d;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let n = self.window;
            let mut mean = 0.0;
            for i in 0..n {
                mean += self.buf[i];
            }
            mean /= n as f64;
            let mut var = 0.0;
            for i in 0..n {
                let dd = self.buf[i] - mean;
                var += dd * dd;
            }
            let std = (var / (n as f64)).sqrt().max(1e-9);
            self.value = -(d - mean) / std;
        } // negative for reversion (more negative -> further from AVWAP)
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
    fn test_avwap_multi_anchor_reversion_creation() {
        let params = vec![AnchoredVwapParams::default()];
        let mar = AvwapMultiAnchorReversion::new(params, 30);
        assert!(!mar.is_ready());
        assert_eq!(mar.value, 0.0);
    }

    #[test]
    fn test_avwap_multi_anchor_reversion_warmup() {
        let params = vec![AnchoredVwapParams::default()];
        let mut mar = AvwapMultiAnchorReversion::new(params, 30);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            mar.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(mar.is_ready());
    }

    #[test]
    fn test_avwap_multi_anchor_reversion_values() {
        let params = vec![AnchoredVwapParams::default()];
        let mut mar = AvwapMultiAnchorReversion::new(params, 30);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = mar.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Value should be finite");
        }
    }

    #[test]
    fn test_avwap_multi_anchor_reversion_reset() {
        let params = vec![AnchoredVwapParams::default()];
        let mut mar = AvwapMultiAnchorReversion::new(params, 30);
        for i in 0..40 {
            mar.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0, 1000.0);
        }
        mar.reset();
        assert!(!mar.is_ready());
        assert_eq!(mar.value, 0.0);
    }
}
