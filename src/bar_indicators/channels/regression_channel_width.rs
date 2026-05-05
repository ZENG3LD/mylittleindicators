// Regression Channel Width: upper - lower from RegressionChannels

use crate::bar_indicators::channels::regression_channels::{
    RegressionChannelMode, RegressionChannels,
};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct RegressionChannelWidth {
    rc: RegressionChannels,
    value: f64,
}

impl RegressionChannelWidth {
    pub fn new(period: usize, mult: f64) -> Self {
        Self {
            rc: RegressionChannels::new(
                period.max(2),
                mult.max(0.1),
                RegressionChannelMode::Standard,
            ),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.rc.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rc.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let (upper, _mid, lower) = self.rc.update_bar(o, h, l, c, v);
        self.value = (upper - lower).abs();
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regression_channel_width_creation() {
        let rcw = RegressionChannelWidth::new(20, 2.0);
        assert!(!rcw.is_ready());
        assert_eq!(rcw.value().main(), 0.0);
    }

    #[test]
    fn test_regression_channel_width_warmup() {
        let mut rcw = RegressionChannelWidth::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rcw.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rcw.is_ready());
    }

    #[test]
    fn test_regression_channel_width_positive() {
        let mut rcw = RegressionChannelWidth::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = rcw.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value >= 0.0, "Width should be non-negative");
        }
    }

    #[test]
    fn test_regression_channel_width_reset() {
        let mut rcw = RegressionChannelWidth::new(20, 2.0);
        for i in 0..25 {
            rcw.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        rcw.reset();
        assert!(!rcw.is_ready());
        assert_eq!(rcw.value().main(), 0.0);
    }
}
