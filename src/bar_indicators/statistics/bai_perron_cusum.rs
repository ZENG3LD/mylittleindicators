// Simplified Bai-Perron break detector: segment-wise CUSUM with windowed reinitialization

use crate::bar_indicators::statistics::cusum_break_detector::CusumBreakDetector;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct BaiPerronCusum {
    inner: CusumBreakDetector,
    seg_window: usize,
    seg_idx: usize,
    pub value: f64,
    ever_updated: bool,
}

impl BaiPerronCusum {
    pub fn new(threshold: f64, kappa: f64, seg_window: usize) -> Self {
        Self {
            inner: CusumBreakDetector::new(threshold, kappa),
            seg_window: seg_window.max(50),
            seg_idx: 0,
            value: 0.0,
            ever_updated: false,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.seg_idx = 0;
        self.value = 0.0;
        self.ever_updated = false;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ever_updated
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let s = self.inner.update_bar(o, h, l, c, v);
        self.seg_idx += 1;
        self.ever_updated = true;
        if self.seg_idx >= self.seg_window {
            self.inner.reset();
            self.seg_idx = 0;
        }
        self.value = s;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bai_perron_cusum_creation() {
        let bpc = BaiPerronCusum::new(0.05, 0.95, 100);
        assert!(!bpc.is_ready());
        assert_eq!(bpc.value, 0.0);
    }

    #[test]
    fn test_bai_perron_cusum_warmup() {
        let mut bpc = BaiPerronCusum::new(0.05, 0.95, 100);
        bpc.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0);
        bpc.update_bar(100.0, 102.0, 98.0, 101.0, 1000.0);
        assert!(bpc.is_ready());
    }

    #[test]
    fn test_bai_perron_cusum_values() {
        let mut bpc = BaiPerronCusum::new(0.05, 0.95, 100);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = bpc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "CUSUM should be non-negative");
        }
    }

    #[test]
    fn test_bai_perron_cusum_reset() {
        let mut bpc = BaiPerronCusum::new(0.05, 0.95, 100);
        for i in 0..10 {
            bpc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        bpc.reset();
        assert!(!bpc.is_ready());
        assert_eq!(bpc.value, 0.0);
    }
}
