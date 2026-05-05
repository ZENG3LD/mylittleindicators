// RAVI (Range Action Verification Index): |EMA(fast)-EMA(slow)|/EMA(slow)*100

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct Ravi {
    ma_type: MovingAverageType,
    fast_period: usize,
    slow_period: usize,
    fast: MovingAverageProvider,
    slow: MovingAverageProvider,
    value: f64,
}

impl Ravi {
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self::new_with_ma_type(fast_period, slow_period, MovingAverageType::EMA)
    }

    pub fn new_default(fast_period: usize, slow_period: usize) -> Self {
        Self::new_with_ma_type(fast_period, slow_period, MovingAverageType::EMA)
    }

    pub fn new_with_ma_type(fast_period: usize, slow_period: usize, ma_type: MovingAverageType) -> Self {
        let fast = fast_period.max(1);
        let slow = slow_period.max(2);
        Self {
            ma_type,
            fast_period: fast,
            slow_period: slow,
            fast: MovingAverageProvider::new(ma_type, fast),
            slow: MovingAverageProvider::new(ma_type, slow),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.fast = MovingAverageProvider::new(self.ma_type, self.fast_period);
        self.slow = MovingAverageProvider::new(self.ma_type, self.slow_period);
        self.value = 0.0;
    }

    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.fast.is_ready() && self.slow.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let f = self.fast.update_bar(0.0, 0.0, 0.0, c, 0.0);
        let s = self.slow.update_bar(0.0, 0.0, 0.0, c, 0.0);
        self.value = if s.abs() > 1e-12 {
            ((f - s).abs() / s) * 100.0
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
    fn test_ravi_creation() {
        let ravi = Ravi::new(7, 65);
        assert!(!ravi.is_ready());
        assert_eq!(ravi.value().main(), 0.0);
    }

    #[test]
    fn test_ravi_warmup() {
        let mut ravi = Ravi::new(7, 65);
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ravi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ravi.is_ready());
    }

    #[test]
    fn test_ravi_values_non_negative() {
        let mut ravi = Ravi::new(7, 65);
        for i in 0..80 {
            let price = 100.0 + i as f64;
            let value = ravi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "RAVI should be non-negative");
        }
    }

    #[test]
    fn test_ravi_reset() {
        let mut ravi = Ravi::new(7, 65);
        for i in 0..80 {
            ravi.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ravi.reset();
        assert!(!ravi.is_ready());
        assert_eq!(ravi.value().main(), 0.0);
    }
}
