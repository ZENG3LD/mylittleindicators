// Trend Intensity Index (TII) - percent of closes above MA in window

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct TrendIntensityIndex {
    window: usize,
    ma: MovingAverageProvider,
    buf: ArrayVec<f64, 512>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl TrendIntensityIndex {
    pub fn new(window: usize) -> Self {
        let w = window.clamp(2, 512);
        Self {
            window: w,
            ma: MovingAverageProvider::new(MovingAverageType::SMA, w),
            buf: ArrayVec::new(),
            idx: 0,
            filled: false,
            value: 50.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.ma.reset();
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.value = 50.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.ma.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let ma = self.ma.update_bar(o, h, l, c, v);
        if self.buf.len() < self.window {
            self.buf.push(c);
            if self.buf.len() == self.window {
                self.filled = true;
            }
        } else {
            self.buf[self.idx] = c;
        }
        self.idx = (self.idx + 1) % self.window;
        if self.is_ready() {
            let above = self.buf.iter().filter(|&&x| x > ma).count() as f64;
            self.value = 100.0 * above / (self.window as f64);
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trend_intensity_index_creation() {
        let tii = TrendIntensityIndex::new(20);
        assert!(!tii.is_ready());
        assert_eq!(tii.value().main(), 50.0);
    }

    #[test]
    fn test_trend_intensity_index_warmup() {
        let mut tii = TrendIntensityIndex::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            tii.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(tii.is_ready());
    }

    #[test]
    fn test_trend_intensity_index_range() {
        let mut tii = TrendIntensityIndex::new(20);
        for i in 0..40 {
            let price = 100.0 + i as f64;
            let value = tii.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 100.0, "TII should be in [0, 100]");
        }
    }

    #[test]
    fn test_trend_intensity_index_reset() {
        let mut tii = TrendIntensityIndex::new(20);
        for i in 0..30 {
            tii.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        tii.reset();
        assert!(!tii.is_ready());
        assert_eq!(tii.value().main(), 50.0);
    }
}
