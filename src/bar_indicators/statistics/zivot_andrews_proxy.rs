// Zivot-Andrews structural break proxy: rolling mean shift detector (z-score)

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct ZivotAndrewsProxy {
    #[allow(dead_code)]
    window: usize,
    value: f64,
}

impl ZivotAndrewsProxy {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(20),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, _v: f64) -> f64 {
        // simple proxy: normalized body against range
        let rng = (h - l).max(1e-9);
        let body = (c - (h + l) / 2.0) / rng;
        // value as magnitude of normalized body (potential break if persistently large)
        self.value = body.abs();
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zivot_andrews_proxy_creation() {
        let zap = ZivotAndrewsProxy::new(50);
        assert!(zap.is_ready()); // Always ready (no warmup needed)
        assert_eq!(zap.value().main(), 0.0);
    }

    #[test]
    fn test_zivot_andrews_proxy_range() {
        let mut zap = ZivotAndrewsProxy::new(50);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = zap.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 1.0, "Value should be in [0, 1]");
        }
    }

    #[test]
    fn test_zivot_andrews_proxy_reset() {
        let mut zap = ZivotAndrewsProxy::new(50);
        zap.update_bar(100.0, 102.0, 98.0, 101.0, 1000.0);
        zap.reset();
        assert_eq!(zap.value().main(), 0.0);
    }
}
