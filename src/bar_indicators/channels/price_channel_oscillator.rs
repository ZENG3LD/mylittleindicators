// Price Channel Oscillator - normalized position within Price Channels mapped to [-1, 1]

use crate::bar_indicators::channels::price_channels::{
    PriceChannelMode, PriceChannels,
};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct PriceChannelOscillator {
    channels: PriceChannels,
    value: f64,
}

impl PriceChannelOscillator {
    pub fn new(period: usize) -> Self {
        Self {
            channels: PriceChannels::new(
                period.max(2),
                PriceChannelMode::Raw,
                crate::bar_indicators::average::MovingAverageType::SMA,
            ),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.channels.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.channels.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let (_u, _m, _d) = self.channels.update_bar(o, h, l, c, v);
        let pos = self.channels.position_in_channel(c); // 0..1
        self.value = 2.0 * pos - 1.0; // -1..1
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_channel_oscillator_creation() {
        let pco = PriceChannelOscillator::new(20);
        assert!(!pco.is_ready());
        assert_eq!(pco.value().main(), 0.0);
    }

    #[test]
    fn test_price_channel_oscillator_warmup() {
        let mut pco = PriceChannelOscillator::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pco.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pco.is_ready());
    }

    #[test]
    fn test_price_channel_oscillator_range() {
        let mut pco = PriceChannelOscillator::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pco.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= -1.0 && value <= 1.0, "Oscillator should be in [-1, 1]");
        }
    }

    #[test]
    fn test_price_channel_oscillator_reset() {
        let mut pco = PriceChannelOscillator::new(20);
        for i in 0..25 {
            pco.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        pco.reset();
        assert!(!pco.is_ready());
        assert_eq!(pco.value().main(), 0.0);
    }
}
