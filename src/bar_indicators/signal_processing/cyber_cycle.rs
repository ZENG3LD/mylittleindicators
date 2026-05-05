// Ehlers Cyber Cycle - simplified recursive filter

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct CyberCycle {
    alpha: f64,
    prev1: f64,
    prev2: f64,
    value: f64,
}

impl CyberCycle {
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
            prev1: 0.0,
            prev2: 0.0,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.prev1 = 0.0;
        self.prev2 = 0.0;
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
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let a = self.alpha;
        let val = (1.0 - a / 2.0) * (1.0 - a / 2.0) * (c - 2.0 * self.prev1 + self.prev2)
            + 2.0 * (1.0 - a) * self.prev1
            - (1.0 - a) * (1.0 - a) * self.prev2;
        self.prev2 = self.prev1;
        self.prev1 = c;
        self.value = val;
        self.value
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cyber_cycle_creation() {
        let cc = CyberCycle::new(0.07);
        assert!(cc.is_ready());
        assert_eq!(cc.value().main(), 0.0);
        assert!((cc.alpha() - 0.07).abs() < 1e-9);
    }

    #[test]
    fn test_cyber_cycle_finite() {
        let mut cc = CyberCycle::new(0.07);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = cc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "CyberCycle should always be finite");
        }
    }

    #[test]
    fn test_cyber_cycle_reset() {
        let mut cc = CyberCycle::new(0.07);
        for i in 1..=20 {
            cc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        cc.reset();
        assert_eq!(cc.value().main(), 0.0);
    }
}
