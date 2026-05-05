// RSI Percentile Rank over a rolling window

use crate::bar_indicators::momentum::rsi::Rsi;
use crate::bar_indicators::utils::math::percentile::percentile_rank;
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;


#[derive(Clone)]
pub struct RsiPercentileRank {
    rsi: Rsi,
    window: usize,
    buf: ArrayVec<f64, 1024>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl RsiPercentileRank {
    pub fn new(rsi_period: usize, window: usize) -> Self {
        Self {
            rsi: Rsi::new(rsi_period.max(1)),
            window: window.clamp(5, 1024),
            buf: ArrayVec::new(),
            idx: 0,
            filled: false,
            value: 50.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.rsi.reset();
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.value = 50.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.rsi.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
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
        if self.is_ready() {
            // 🚀 O(n) percentile calculation instead of O(n log n) sorting
            self.value = percentile_rank(&self.buf[..], r);
        }
        self.value
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsi_percentile_rank_creation() {
        let rpr = RsiPercentileRank::new(14, 50);
        assert!(!rpr.is_ready());
        assert_eq!(rpr.value().main(), 50.0);
        assert_eq!(rpr.window(), 50);
    }

    #[test]
    fn test_rsi_percentile_rank_basic() {
        let mut rpr = RsiPercentileRank::new(14, 50);
        for i in 1..=80 {
            let price = 100.0 + i as f64 * 2.0;
            rpr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rpr.is_ready());
        assert!(rpr.value().main().is_finite());
    }

    #[test]
    fn test_rsi_percentile_rank_range() {
        let mut rpr = RsiPercentileRank::new(14, 50);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = rpr.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "RSI Percentile Rank should always be finite");
            if rpr.is_ready() {
                assert!(value >= 0.0 && value <= 100.0, "Percentile should be in [0, 100], got {}", value);
            }
        }
    }

    #[test]
    fn test_rsi_percentile_rank_reset() {
        let mut rpr = RsiPercentileRank::new(14, 50);
        for i in 1..=80 {
            let price = 100.0 + i as f64;
            rpr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rpr.is_ready());
        rpr.reset();
        assert!(!rpr.is_ready());
        assert_eq!(rpr.value().main(), 50.0);
    }
}
