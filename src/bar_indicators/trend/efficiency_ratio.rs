// Kaufman's Efficiency Ratio (ER) on close: direction/volatility over window

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct EfficiencyRatio {
    window: usize,
    closes: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    pub value: f64,
}

impl EfficiencyRatio {
    pub fn new(window: usize) -> Self {
        let w = window.max(2);
        Self {
            window: w,
            closes: vec![0.0; w],
            idx: 0,
            filled: false,
            last_close: None,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.last_close = None;
        self.closes.fill(0.0);
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> f64 {
        let n = self.window;
        self.closes[self.idx] = close;
        self.idx = (self.idx + 1) % n;
        if !self.filled && self.idx == 0 {
            self.filled = true;
        }

        if self.filled {
            let oldest_idx = self.idx;
            let newest_idx = (self.idx + n - 1) % n;
            let direction = (self.closes[newest_idx] - self.closes[oldest_idx]).abs();
            let mut volatility = 0.0;
            for i in 0..n - 1 {
                let a = (self.idx + i) % n;
                let b = (self.idx + i + 1) % n;
                volatility += (self.closes[b] - self.closes[a]).abs();
            }
            self.value = if volatility > 0.0 {
                (direction / volatility).clamp(0.0, 1.0)
            } else {
                0.0
            };
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_efficiency_ratio_creation() {
        let er = EfficiencyRatio::new(10);
        assert!(!er.is_ready());
        assert_eq!(er.value, 0.0);
    }

    #[test]
    fn test_efficiency_ratio_warmup() {
        let mut er = EfficiencyRatio::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            er.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(er.is_ready());
    }

    #[test]
    fn test_efficiency_ratio_range() {
        let mut er = EfficiencyRatio::new(10);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            let value = er.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 1.0, "ER should be in [0, 1]");
        }
    }

    #[test]
    fn test_efficiency_ratio_reset() {
        let mut er = EfficiencyRatio::new(10);
        for i in 0..15 {
            er.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        er.reset();
        assert!(!er.is_ready());
        assert_eq!(er.value, 0.0);
    }
}
