// VWAP Channel Width: (Upper - Lower) from VwapChannels

use crate::bar_indicators::channels::vwap_channels::{VwapChannelMode, VwapChannels};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct VwapChannelWidth {
    vc: VwapChannels,
    value: f64,
}

impl VwapChannelWidth {
    pub fn new(period: usize, mult: f64) -> Self {
        Self {
            vc: VwapChannels::new(period.max(1), mult.max(0.1), VwapChannelMode::Standard),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.vc.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.vc.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let (upper, _mid, lower) = self.vc.update_bar(o, h, l, c, v);
        self.value = (upper - lower).abs();
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vwap_channel_width_creation() {
        let vcw = VwapChannelWidth::new(20, 2.0);
        assert!(!vcw.is_ready());
        assert_eq!(vcw.value().main(), 0.0);
    }

    #[test]
    fn test_vwap_channel_width_warmup() {
        let mut vcw = VwapChannelWidth::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vcw.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vcw.is_ready());
    }

    #[test]
    fn test_vwap_channel_width_positive() {
        let mut vcw = VwapChannelWidth::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vcw.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value >= 0.0, "Width should be non-negative");
        }
    }

    #[test]
    fn test_vwap_channel_width_reset() {
        let mut vcw = VwapChannelWidth::new(20, 2.0);
        for i in 0..25 {
            vcw.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        vcw.reset();
        assert!(!vcw.is_ready());
        assert_eq!(vcw.value().main(), 0.0);
    }
}
