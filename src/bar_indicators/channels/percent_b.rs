// PercentB - wrapper over BollingerBands percent_b metric

use crate::bar_indicators::average::moving_average::MovingAverageType;
use crate::bar_indicators::channels::bollinger_bands::{BollingerBands, BollingerMode};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct PercentB {
    bb: BollingerBands,
    value: f64,
}

impl PercentB {
    pub fn new(period: usize, std_mult: f64) -> Self {
        Self {
            bb: BollingerBands::new(
                period.max(2),
                std_mult.max(0.1),
                BollingerMode::Close,
                MovingAverageType::SMA,
            ),
            value: 0.5,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.bb.reset();
        self.value = 0.5;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.bb.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let _ = self.bb.update_bar(o, h, l, c, v);
        self.value = self.bb.percent_b();
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_b_creation() {
        let pb = PercentB::new(20, 2.0);
        assert!(!pb.is_ready());
        assert_eq!(pb.value().main(), 0.5);
    }

    #[test]
    fn test_percent_b_warmup() {
        let mut pb = PercentB::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pb.is_ready());
    }

    #[test]
    fn test_percent_b_values() {
        let mut pb = PercentB::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_percent_b_reset() {
        let mut pb = PercentB::new(20, 2.0);
        for i in 0..25 {
            pb.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        pb.reset();
        assert!(!pb.is_ready());
        assert_eq!(pb.value().main(), 0.5);
    }
}
