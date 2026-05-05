// Envelope Bandwidth: (Upper - Lower) / Middle for EnvelopeChannels

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::channels::envelope_channels::{
    EnvelopeChannels, EnvelopeMode,
};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct EnvelopeBandwidth {
    env: EnvelopeChannels,
    value: f64,
}

impl EnvelopeBandwidth {
    pub fn new(period: usize, pct: f64) -> Self {
        Self {
            env: EnvelopeChannels::new(
                period.max(1),
                pct.max(0.01),
                EnvelopeMode::Fixed,
                MovingAverageType::SMA,
            ),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.env.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.env.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let (upper, middle, lower) = self.env.update_bar(o, h, l, c, v);
        let width = (upper - lower).abs();
        self.value = if middle.abs() > 1e-12 {
            width / middle.abs()
        } else {
            0.0
        };
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_bandwidth_creation() {
        let eb = EnvelopeBandwidth::new(20, 2.5);
        assert!(!eb.is_ready());
        assert_eq!(eb.value().main(), 0.0);
    }

    #[test]
    fn test_envelope_bandwidth_warmup() {
        let mut eb = EnvelopeBandwidth::new(20, 2.5);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            eb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(eb.is_ready());
    }

    #[test]
    fn test_envelope_bandwidth_positive() {
        let mut eb = EnvelopeBandwidth::new(20, 2.5);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = eb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "Bandwidth should be non-negative");
        }
    }

    #[test]
    fn test_envelope_bandwidth_reset() {
        let mut eb = EnvelopeBandwidth::new(20, 2.5);
        for i in 0..25 {
            eb.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        eb.reset();
        assert!(!eb.is_ready());
        assert_eq!(eb.value().main(), 0.0);
    }
}
