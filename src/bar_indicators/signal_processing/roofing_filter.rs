// Roofing Filter (Ehlers) - high-pass followed by low-pass (placeholder)

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct RoofingFilter {
    hp_alpha: f64,
    lp_alpha: f64,
    hp_prev: f64,
    lp_prev: f64,
    value: f64,
    ready: bool,
}

impl RoofingFilter {
    pub fn new(hp_alpha: f64, lp_alpha: f64) -> Self {
        Self {
            hp_alpha: hp_alpha.clamp(0.0, 1.0),
            lp_alpha: lp_alpha.clamp(0.0, 1.0),
            hp_prev: 0.0,
            lp_prev: 0.0,
            value: 0.0,
            ready: false,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.hp_prev = 0.0;
        self.lp_prev = 0.0;
        self.value = 0.0;
        self.ready = false;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        // simple 1-pole HP then LP
        let hp = self.hp_alpha * (self.hp_prev + c - self.lp_prev);
        self.hp_prev = hp;
        let lp = self.lp_prev + self.lp_alpha * (hp - self.lp_prev);
        self.lp_prev = lp;
        self.value = lp;
        self.ready = true;
        self.value
    }

    pub fn hp_alpha(&self) -> f64 {
        self.hp_alpha
    }

    pub fn lp_alpha(&self) -> f64 {
        self.lp_alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roofing_filter_creation() {
        let rf = RoofingFilter::new(0.5, 0.2);
        assert!(!rf.is_ready());
        assert_eq!(rf.value().main(), 0.0);
        assert!((rf.hp_alpha() - 0.5).abs() < 1e-9);
        assert!((rf.lp_alpha() - 0.2).abs() < 1e-9);
    }

    #[test]
    fn test_roofing_filter_ready_after_update() {
        let mut rf = RoofingFilter::new(0.5, 0.2);
        rf.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0);
        assert!(rf.is_ready());
    }

    #[test]
    fn test_roofing_filter_finite() {
        let mut rf = RoofingFilter::new(0.5, 0.2);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = rf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Roofing filter should always be finite");
        }
    }

    #[test]
    fn test_roofing_filter_reset() {
        let mut rf = RoofingFilter::new(0.5, 0.2);
        for i in 1..=20 {
            rf.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        rf.reset();
        assert!(!rf.is_ready());
        assert_eq!(rf.value().main(), 0.0);
    }
}
