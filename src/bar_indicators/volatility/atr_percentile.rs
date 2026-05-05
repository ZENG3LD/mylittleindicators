// ATR Percentile over rolling window
// Returns percentile rank of current ATR within the last N ATR values [0..1]

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct AtrPercentile {
    atr: Atr,
    window: usize,
    buffer: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl AtrPercentile {
    pub fn new(atr_period: usize, ma_type: MovingAverageType, window: usize) -> Self {
        Self {
            atr: Atr::new(atr_period, ma_type),
            window,
            buffer: vec![0.0; window.max(1)],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.atr.reset();
        self.buffer.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }

    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        let atr_val = self.atr.update_bar(0.0, high, low, close, 0.0).abs();
        self.buffer[self.idx] = atr_val;
        self.idx = (self.idx + 1) % self.window.max(1);
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }

        // Percentile rank
        let len = if self.filled { self.window } else { self.idx };
        if len == 0 {
            self.value = 0.0;
            return self.value;
        }
        let mut count = 0usize;
        for i in 0..len {
            if self.buffer[i] <= atr_val {
                count += 1;
            }
        }
        self.value = (count as f64) / (len as f64);
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atr_percentile_creation() {
        let ap = AtrPercentile::new(14, MovingAverageType::RMA, 50);
        assert!(!ap.is_ready());
        assert_eq!(ap.value().main(), 0.0);
    }

    #[test]
    fn test_atr_percentile_warmup() {
        let mut ap = AtrPercentile::new(14, MovingAverageType::RMA, 50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ap.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ap.is_ready());
    }

    #[test]
    fn test_atr_percentile_range() {
        let mut ap = AtrPercentile::new(14, MovingAverageType::RMA, 50);
        for i in 0..60 {
            let price = 100.0 + i as f64;
            let value = ap.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 1.0, "Percentile should be in [0, 1]");
        }
    }

    #[test]
    fn test_atr_percentile_reset() {
        let mut ap = AtrPercentile::new(14, MovingAverageType::RMA, 50);
        for i in 0..60 {
            ap.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        ap.reset();
        assert!(!ap.is_ready());
        assert_eq!(ap.value().main(), 0.0);
    }
}
