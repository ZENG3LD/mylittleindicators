// RSI Percentile Bands: upper/lower percentiles of RSI distribution

use crate::bar_indicators::momentum::rsi::Rsi;
use crate::bar_indicators::utils::math::percentile::quickselect_nth;
use crate::bar_indicators::indicator_value::IndicatorValue;
use arrayvec::ArrayVec;


#[derive(Clone)]
pub struct RsiPercentileBands {
    rsi: Rsi,
    window: usize,
    buf: ArrayVec<f64, 1024>,
    idx: usize,
    filled: bool,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl RsiPercentileBands {
    pub fn new(rsi_period: usize, window: usize) -> Self {
        Self {
            rsi: Rsi::new(rsi_period.max(1)),
            window: window.clamp(10, 1024),
            buf: ArrayVec::new(),
            idx: 0,
            filled: false,
            upper: 80.0,
            middle: 50.0,
            lower: 20.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.rsi.reset();
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.upper = 80.0;
        self.middle = 50.0;
        self.lower = 20.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.rsi.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 { upper: self.upper, middle: self.middle, lower: self.lower }
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> (f64, f64, f64) {
        let r = self.rsi.update_bar(o, h, l, c, v);
        if self.buf.len() < self.window {
            self.buf.push(r);
            if self.buf.len() == self.window {
                self.filled = true;
            }
        } else {
            self.buf[self.idx] = r;
        }
        self.idx = (self.idx + 1) % self.window;
        self.middle = r;
        if self.is_ready() {
            // 🚀 O(n) quickselect instead of O(n log n) sorting
            let mut sorted: Vec<f64> = self.buf.iter().copied().collect();
            let len = sorted.len();
            self.lower = quickselect_nth(&mut sorted, (len * 20) / 100);
            self.upper = quickselect_nth(&mut sorted, (len * 80) / 100);
        }
        (self.upper, self.middle, self.lower)
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsi_percentile_bands_creation() {
        let rpb = RsiPercentileBands::new(14, 50);
        assert!(!rpb.is_ready());
        assert_eq!(rpb.value(), IndicatorValue::Channel3 { upper: 80.0, middle: 50.0, lower: 20.0 });
        assert_eq!(rpb.window(), 50);
    }

    #[test]
    fn test_rsi_percentile_bands_basic() {
        let mut rpb = RsiPercentileBands::new(14, 50);
        for i in 1..=80 {
            let price = 100.0 + i as f64 * 2.0;
            rpb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rpb.is_ready());
        if let IndicatorValue::Channel3 { upper, middle, lower } = rpb.value() {
            assert!(upper.is_finite() && middle.is_finite() && lower.is_finite());
            assert!(upper >= lower, "Upper band should >= lower band");
        } else { panic!("Expected Channel3"); }
    }

    #[test]
    fn test_rsi_percentile_bands_finite() {
        let mut rpb = RsiPercentileBands::new(14, 50);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let (upper, middle, lower) = rpb.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(upper.is_finite() && middle.is_finite() && lower.is_finite());
        }
    }

    #[test]
    fn test_rsi_percentile_bands_reset() {
        let mut rpb = RsiPercentileBands::new(14, 50);
        for i in 1..=80 {
            let price = 100.0 + i as f64;
            rpb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rpb.is_ready());
        rpb.reset();
        assert!(!rpb.is_ready());
        assert_eq!(rpb.value(), IndicatorValue::Channel3 { upper: 80.0, middle: 50.0, lower: 20.0 });
    }
}
