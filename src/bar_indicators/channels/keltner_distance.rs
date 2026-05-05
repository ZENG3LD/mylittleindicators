// Keltner Distance: distance of price from Keltner center in ATR units

use crate::bar_indicators::average::moving_average::MovingAverageType;
use crate::bar_indicators::channels::keltner_channel::{KeltnerChannel, KeltnerMode};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct KeltnerDistance {
    kc: KeltnerChannel,
    value: f64,
}

impl KeltnerDistance {
    pub fn new(period: usize, multiplier: f64) -> Self {
        Self {
            kc: KeltnerChannel::new(
                period.max(2),
                multiplier.max(0.1),
                KeltnerMode::Classic,
                MovingAverageType::SMA,
                MovingAverageType::RMA,
            ),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.kc.reset();
        self.value = 0.0;
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
        let _ = self.kc.update_bar(o, h, l, c, v);
        self.value = self.kc.distance_from_center_atr(c);
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keltner_distance_creation() {
        let kd = KeltnerDistance::new(20, 2.0);
        assert!(!kd.is_ready());
        assert_eq!(kd.value().main(), 0.0);
    }

    #[test]
    fn test_keltner_distance_warmup() {
        let mut kd = KeltnerDistance::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kd.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(kd.is_ready());
    }

    #[test]
    fn test_keltner_distance_finite() {
        let mut kd = KeltnerDistance::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kd.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Distance should be finite");
        }
    }

    #[test]
    fn test_keltner_distance_reset() {
        let mut kd = KeltnerDistance::new(20, 2.0);
        for i in 0..25 {
            kd.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        kd.reset();
        assert!(!kd.is_ready());
        assert_eq!(kd.value().main(), 0.0);
    }
}
