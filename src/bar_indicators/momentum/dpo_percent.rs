// DPO%: detrended price oscillator as percent of price

use crate::bar_indicators::momentum::dpo::DetrendedPriceOscillator;
use crate::bar_indicators::indicator_value::IndicatorValue;


#[derive(Clone)]
pub struct DpoPercent {
    dpo: DetrendedPriceOscillator,
    value: f64,
}

impl DpoPercent {
    pub fn new(period: usize) -> Self {
        Self {
            dpo: DetrendedPriceOscillator::with_period(period.max(2)),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.dpo.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.dpo.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let d = self.dpo.update_bar(0.0, 0.0, 0.0, c, 0.0);
        self.value = if c.abs() > 1e-12 { d / c } else { 0.0 };
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dpo_percent_creation() {
        let dpo = DpoPercent::new(14);
        assert!(!dpo.is_ready());
        assert_eq!(dpo.value().main(), 0.0);
    }

    #[test]
    fn test_dpo_percent_basic() {
        let mut dpo = DpoPercent::new(10);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            dpo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(dpo.is_ready());
        // DPO% is DPO divided by price, should be finite
        assert!(dpo.value().main().is_finite());
    }

    #[test]
    fn test_dpo_percent_reset() {
        let mut dpo = DpoPercent::new(10);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            dpo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(dpo.is_ready());
        dpo.reset();
        assert!(!dpo.is_ready());
        assert_eq!(dpo.value().main(), 0.0);
    }

    #[test]
    fn test_dpo_percent_finite_values() {
        let mut dpo = DpoPercent::new(10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = dpo.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "DPO% should always be finite");
        }
    }
}
