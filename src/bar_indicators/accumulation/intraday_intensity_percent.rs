// Intraday Intensity Percent (IIP)

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct IntradayIntensityPercent {
    value: f64,
}

impl Default for IntradayIntensityPercent {
    fn default() -> Self {
        Self::new()
    }
}

impl IntradayIntensityPercent {
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
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let denom = (h - l).abs().max(1e-9);
        let iip = ((2.0 * c - h - l) / denom) * v;
        self.value = iip;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intraday_intensity_percent_creation() {
        let iip = IntradayIntensityPercent::new();
        assert!(iip.is_ready());
        assert_eq!(iip.value().main(), 0.0);
    }

    #[test]
    fn test_intraday_intensity_percent_values_finite() {
        let mut iip = IntradayIntensityPercent::new();
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = iip.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_intraday_intensity_percent_reset() {
        let mut iip = IntradayIntensityPercent::new();
        for i in 0..10 {
            iip.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        iip.reset();
        assert_eq!(iip.value().main(), 0.0);
    }
}
