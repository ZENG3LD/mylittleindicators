// Order Book Slope (proxy) - slope of price vs normalized volume (approx via OHLCV)

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct OrderBookSlope {
    value: f64,
}

impl Default for OrderBookSlope {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderBookSlope {
    pub fn new() -> Self {
        Self { value: 0.0 }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, _c: f64, v: f64) -> f64 {
        let spread = (h - l).max(1e-9);
        self.value = (v.ln().max(0.0)) / spread;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_book_slope_creation() {
        let ind = OrderBookSlope::new();
        assert!(ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_order_book_slope_update() {
        let mut ind = OrderBookSlope::new();
        let value = ind.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0);
        assert!(value.is_finite());
        assert!(value > 0.0);
    }

    #[test]
    fn test_order_book_slope_reset() {
        let mut ind = OrderBookSlope::new();
        ind.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0);
        ind.reset();
        assert_eq!(ind.value().main(), 0.0);
    }
}
