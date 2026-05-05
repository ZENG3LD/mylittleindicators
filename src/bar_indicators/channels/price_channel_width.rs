// Price Channel Width: upper - lower from PriceChannels

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::channels::price_channels::{
    PriceChannelMode, PriceChannels,
};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct PriceChannelWidth {
    pc: PriceChannels,
    value: f64,
}

impl PriceChannelWidth {
    pub fn new(period: usize) -> Self {
        Self {
            pc: PriceChannels::new(
                period.max(2),
                PriceChannelMode::Raw,
                MovingAverageType::SMA,
            ),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.pc.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.pc.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let (upper, _mid, lower) = self.pc.update_bar(o, h, l, c, v);
        self.value = upper - lower;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_channel_width_creation() {
        let pcw = PriceChannelWidth::new(20);
        assert!(!pcw.is_ready());
        assert_eq!(pcw.value().main(), 0.0);
    }

    #[test]
    fn test_price_channel_width_warmup() {
        let mut pcw = PriceChannelWidth::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pcw.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pcw.is_ready());
    }

    #[test]
    fn test_price_channel_width_positive() {
        let mut pcw = PriceChannelWidth::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pcw.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value >= 0.0, "Width should be non-negative");
        }
    }

    #[test]
    fn test_price_channel_width_reset() {
        let mut pcw = PriceChannelWidth::new(20);
        for i in 0..25 {
            pcw.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        pcw.reset();
        assert!(!pcw.is_ready());
        assert_eq!(pcw.value().main(), 0.0);
    }
}
