// Median Channel Position: (price - lower) / (upper - lower) for MedianChannels (using upper_mad/lower_mad)

use crate::bar_indicators::channels::median_channels::{
    MedianChannels, MedianMode, MedianSource,
};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct MedianChannelPosition {
    mc: MedianChannels,
    value: f64,
}

impl MedianChannelPosition {
    pub fn new(period: usize) -> Self {
        Self {
            mc: MedianChannels::new_custom(
                period.max(3),
                MedianMode::Simple,
                MedianSource::Close,
                1.4826,
            ),
            value: 0.5,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.mc.reset();
        self.value = 0.5;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.mc.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let (_median, upper, lower) = self.mc.update_bar(o, h, l, c, v);
        let width = (upper - lower).abs();
        self.value = if width > 0.0 {
            ((c - lower) / width).clamp(0.0, 1.0)
        } else {
            0.5
        };
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_median_channel_position_creation() {
        let mcp = MedianChannelPosition::new(20);
        assert!(!mcp.is_ready());
        assert_eq!(mcp.value().main(), 0.5);
    }

    #[test]
    fn test_median_channel_position_warmup() {
        let mut mcp = MedianChannelPosition::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            mcp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(mcp.is_ready());
    }

    #[test]
    fn test_median_channel_position_range() {
        let mut mcp = MedianChannelPosition::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = mcp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 1.0, "Position should be in [0, 1]");
        }
    }

    #[test]
    fn test_median_channel_position_reset() {
        let mut mcp = MedianChannelPosition::new(20);
        for i in 0..25 {
            mcp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        mcp.reset();
        assert!(!mcp.is_ready());
        assert_eq!(mcp.value().main(), 0.5);
    }
}
