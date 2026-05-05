// VWAP Distance: (Close - VWAP) / VWAP

use crate::bar_indicators::average::vwap::Vwap;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct VwapDistance {
    vwap: Vwap,
    value: f64,
}

impl VwapDistance {
    pub fn new(period: usize) -> Self {
        Self {
            vwap: Vwap::new(period.max(1)),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.vwap.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.vwap.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let vwap_val = self.vwap.update_bar(o, h, l, c, v);
        self.value = if vwap_val.abs() > 1e-12 {
            (c - vwap_val) / vwap_val
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
    fn test_vwap_distance_creation() {
        let vd = VwapDistance::new(20);
        assert!(!vd.is_ready());
        assert_eq!(vd.value().main(), 0.0);
    }

    #[test]
    fn test_vwap_distance_warmup() {
        let mut vd = VwapDistance::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vd.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vd.is_ready());
    }

    #[test]
    fn test_vwap_distance_values() {
        let mut vd = VwapDistance::new(20);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let value = vd.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Distance should be finite");
        }
    }

    #[test]
    fn test_vwap_distance_reset() {
        let mut vd = VwapDistance::new(20);
        for i in 0..25 {
            vd.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0, 1000.0);
        }
        vd.reset();
        assert!(!vd.is_ready());
        assert_eq!(vd.value().main(), 0.0);
    }
}
