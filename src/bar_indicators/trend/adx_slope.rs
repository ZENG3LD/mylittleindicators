// ADX slope and persistence score

use crate::bar_indicators::momentum::adx::Adx;
use crate::bar_indicators::indicator_value::IndicatorValue;


#[derive(Clone)]
pub struct AdxSlope {
    adx: Adx,
    prev: f64,
    slope: f64,
}

impl AdxSlope {
    pub fn new(period: usize) -> Self {
        Self {
            adx: Adx::new(period.max(2)),
            prev: 0.0,
            slope: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.adx.reset();
        self.prev = 0.0;
        self.slope = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.adx.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.slope)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let val = self.adx.update_bar(o, h, l, c, v);
        if self.adx.is_ready() {
            self.slope = val - self.prev;
            self.prev = val;
        }
        self.slope
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adx_slope_creation() {
        let adx = AdxSlope::new(14);
        assert!(!adx.is_ready());
        assert_eq!(adx.value().main(), 0.0);
    }

    #[test]
    fn test_adx_slope_warmup() {
        let mut adx = AdxSlope::new(14);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            adx.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(adx.is_ready());
    }

    #[test]
    fn test_adx_slope_values_finite() {
        let mut adx = AdxSlope::new(14);
        for i in 0..50 {
            let price = 100.0 + i as f64;
            let value = adx.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_adx_slope_reset() {
        let mut adx = AdxSlope::new(14);
        for i in 0..50 {
            adx.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        adx.reset();
        assert!(!adx.is_ready());
        assert_eq!(adx.value().main(), 0.0);
    }
}
