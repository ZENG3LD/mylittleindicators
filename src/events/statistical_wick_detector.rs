//! StatisticalWickDetector — flags unusually long wicks vs rolling 95th percentile.
//!
//! Ported from `bar_indicators/candles/wick_spike.rs`.
//! Output: `IndicatorValue::DoubleFlag(upper_spike, lower_spike)`.

use crate::bar_indicators::indicator_value::IndicatorValue;

/// Detects statistically extreme wicks using a rolling percentile window.
#[derive(Clone)]
pub struct StatisticalWickDetector {
    window: usize,
    upper_buf: Vec<f64>,
    lower_buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub is_upper_spike: bool,
    pub is_lower_spike: bool,
    pub upper_percentile: f64,
    pub lower_percentile: f64,
}

impl StatisticalWickDetector {
    pub fn new(window: usize) -> Self {
        let w = window.max(1);
        Self {
            window: w,
            upper_buf: vec![0.0; w],
            lower_buf: vec![0.0; w],
            idx: 0,
            filled: false,
            is_upper_spike: false,
            is_lower_spike: false,
            upper_percentile: 0.0,
            lower_percentile: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.upper_buf.fill(0.0);
        self.lower_buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.is_upper_spike = false;
        self.is_lower_spike = false;
        self.upper_percentile = 0.0;
        self.lower_percentile = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        _volume: f64,
    ) -> (bool, bool) {
        let range = (high - low).abs().max(1e-12);
        let upper = (high - open.max(close)).max(0.0) / range;
        let lower = (open.min(close) - low).max(0.0) / range;
        self.upper_buf[self.idx] = upper;
        self.lower_buf[self.idx] = lower;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }

        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            let mut cnt_u = 0usize;
            let mut cnt_l = 0usize;
            for i in 0..len {
                if self.upper_buf[i] <= upper {
                    cnt_u += 1;
                }
                if self.lower_buf[i] <= lower {
                    cnt_l += 1;
                }
            }
            self.upper_percentile = cnt_u as f64 / len as f64;
            self.lower_percentile = cnt_l as f64 / len as f64;
        }
        self.is_upper_spike = self.upper_percentile >= 0.95;
        self.is_lower_spike = self.lower_percentile >= 0.95;
        (self.is_upper_spike, self.is_lower_spike)
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::DoubleFlag(self.is_upper_spike, self.is_lower_spike)
    }
}

impl std::fmt::Debug for StatisticalWickDetector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatisticalWickDetector")
            .field("window", &self.window)
            .field("filled", &self.filled)
            .field("is_upper_spike", &self.is_upper_spike)
            .field("is_lower_spike", &self.is_lower_spike)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creation() {
        let ind = StatisticalWickDetector::new(20);
        assert!(!ind.is_ready());
        assert!(!ind.is_upper_spike);
        assert!(!ind.is_lower_spike);
    }

    #[test]
    fn warmup() {
        let mut ind = StatisticalWickDetector::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1_f64).sin() * 5.0;
            ind.update_bar(price, price + 2.0, price - 2.0, price + 1.0, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn percentiles_in_range() {
        let mut ind = StatisticalWickDetector::new(10);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            ind.update_bar(price, price + 2.0, price - 2.0, price + 1.0, 1000.0);
        }
        assert!(ind.upper_percentile >= 0.0 && ind.upper_percentile <= 1.0);
        assert!(ind.lower_percentile >= 0.0 && ind.lower_percentile <= 1.0);
    }

    #[test]
    fn reset_clears() {
        let mut ind = StatisticalWickDetector::new(10);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            ind.update_bar(price, price + 2.0, price - 2.0, price + 1.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert!(!ind.is_upper_spike);
        assert!(!ind.is_lower_spike);
        assert_eq!(ind.upper_percentile, 0.0);
        assert_eq!(ind.lower_percentile, 0.0);
    }
}
