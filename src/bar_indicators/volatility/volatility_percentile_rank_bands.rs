// Volatility Percentile Rank Bands: middle=price, bands by ATR percentile rank

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::utils::math::percentile::quickselect_nth;
use crate::bar_indicators::indicator_value::IndicatorValue;
use arrayvec::ArrayVec;

#[derive(Debug, Clone)]
pub struct VolatilityPercentileRankBands {
    atr: Atr,
    window: usize,
    buf: ArrayVec<f64, 512>,
    idx: usize,
    filled: bool,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl VolatilityPercentileRankBands {
    pub fn new(atr_period: usize, rank_window: usize) -> Self {
        Self {
            atr: Atr::new(atr_period.max(1), MovingAverageType::EMA),
            window: rank_window.clamp(5, 512),
            buf: ArrayVec::new(),
            idx: 0,
            filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.atr.reset();
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.atr.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 { upper: self.upper, middle: self.middle, lower: self.lower }
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> (f64, f64, f64) {
        let atr = self.atr.update_bar(o, h, l, c, v);
        let val = atr.max(1e-12);
        if self.buf.len() < self.window {
            self.buf.push(val);
            if self.buf.len() == self.window {
                self.filled = true;
            }
        } else {
            self.buf[self.idx] = val;
        }
        self.idx = (self.idx + 1) % self.window;
        self.middle = c;
        if self.is_ready() {
            // 🚀 O(n) quickselect instead of O(n log n) sorting
            let mut temp: Vec<f64> = self.buf.iter().copied().collect();
            let len = temp.len();
            let p20 = quickselect_nth(&mut temp[..], (len * 20) / 100);
            let p80 = quickselect_nth(&mut temp[..], (len * 80) / 100);
            self.upper = c + p80;
            self.lower = c - p20;
        }
        (self.upper, self.middle, self.lower)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volatility_percentile_rank_bands_creation() {
        let vprb = VolatilityPercentileRankBands::new(14, 50);
        assert!(!vprb.is_ready());
        assert_eq!(vprb.value(), IndicatorValue::Channel3 { upper: 0.0, middle: 0.0, lower: 0.0 });
    }

    #[test]
    fn test_volatility_percentile_rank_bands_warmup() {
        let mut vprb = VolatilityPercentileRankBands::new(14, 50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vprb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vprb.is_ready());
    }

    #[test]
    fn test_volatility_percentile_rank_bands_ordering() {
        let mut vprb = VolatilityPercentileRankBands::new(14, 50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, middle, lower) = vprb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if vprb.is_ready() {
                assert!(upper >= middle, "Upper should be >= middle");
                assert!(middle >= lower, "Middle should be >= lower");
            }
        }
    }

    #[test]
    fn test_volatility_percentile_rank_bands_reset() {
        let mut vprb = VolatilityPercentileRankBands::new(14, 50);
        for i in 0..60 {
            vprb.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        vprb.reset();
        assert!(!vprb.is_ready());
        assert_eq!(vprb.value(), IndicatorValue::Channel3 { upper: 0.0, middle: 0.0, lower: 0.0 });
    }
}
