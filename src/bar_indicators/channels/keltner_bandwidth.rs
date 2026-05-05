// Keltner Bandwidth: (upper - lower) / middle

use crate::bar_indicators::average::moving_average::MovingAverageType;
use crate::bar_indicators::channels::keltner_channel::{KeltnerChannel, KeltnerMode};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct KeltnerBandwidth {
    kc: KeltnerChannel,
    value: f64,
}

impl KeltnerBandwidth {
    pub fn new(period: usize, mult: f64) -> Self {
        Self {
            kc: KeltnerChannel::new(
                period.max(2),
                mult.max(0.1),
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
        let (upper, middle, lower) = self.kc.update_bar(o, h, l, c, v);
        let width = upper - lower;
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
    fn test_keltner_bandwidth_creation() {
        let kb = KeltnerBandwidth::new(20, 2.0);
        assert!(!kb.is_ready());
        assert_eq!(kb.value().main(), 0.0);
    }

    #[test]
    fn test_keltner_bandwidth_warmup() {
        let mut kb = KeltnerBandwidth::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(kb.is_ready());
    }

    #[test]
    fn test_keltner_bandwidth_positive() {
        let mut kb = KeltnerBandwidth::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "Bandwidth should be non-negative");
        }
    }

    #[test]
    fn test_keltner_bandwidth_reset() {
        let mut kb = KeltnerBandwidth::new(20, 2.0);
        for i in 0..25 {
            kb.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        kb.reset();
        assert!(!kb.is_ready());
        assert_eq!(kb.value().main(), 0.0);
    }
}
