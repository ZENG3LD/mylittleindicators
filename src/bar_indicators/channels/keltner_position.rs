// Keltner Position: (Close - Lower) / (Upper - Lower)

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::channels::keltner_channel::{KeltnerChannel, KeltnerMode};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct KeltnerPosition {
    kc: KeltnerChannel,
    value: f64,
}

impl KeltnerPosition {
    pub fn new(period: usize, mult: f64) -> Self {
        Self {
            kc: KeltnerChannel::new(
                period.max(2),
                mult.max(0.1),
                KeltnerMode::Classic,
                MovingAverageType::SMA,
                MovingAverageType::RMA,
            ),
            value: 0.5,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.kc.reset();
        self.value = 0.5;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.kc.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let (upper, _mid, lower) = self.kc.update_bar(o, h, l, c, v);
        let width = (upper - lower).max(0.0);
        self.value = if width > 0.0 {
            (c - lower) / width
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
    fn test_keltner_position_creation() {
        let kp = KeltnerPosition::new(20, 2.0);
        assert!(!kp.is_ready());
        assert_eq!(kp.value().main(), 0.5);
    }

    #[test]
    fn test_keltner_position_warmup() {
        let mut kp = KeltnerPosition::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(kp.is_ready());
    }

    #[test]
    fn test_keltner_position_range() {
        let mut kp = KeltnerPosition::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_keltner_position_reset() {
        let mut kp = KeltnerPosition::new(20, 2.0);
        for i in 0..25 {
            kp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        kp.reset();
        assert!(!kp.is_ready());
        assert_eq!(kp.value().main(), 0.5);
    }
}
