// Jump test proxy using Bipower Variance vs Realized Variance

use crate::bar_indicators::volatility::bipower_variance::BipowerVariance;
use crate::bar_indicators::volatility::realized_vol::RealizedVol;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct RbvJumpTest {
    rbv: BipowerVariance,
    rv: RealizedVol,
    value: f64,
}

impl RbvJumpTest {
    pub fn new(window: usize, annualize_factor: f64) -> Self {
        Self {
            rbv: BipowerVariance::new(window.max(2)),
            rv: RealizedVol::new(window.max(2), annualize_factor),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.rbv.reset();
        self.rv.reset();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rbv.is_ready() && self.rv.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let rbv = self.rbv.update_bar(o, h, l, c, v);
        let rv = self.rv.update_bar(o, h, l, c, v);
        self.value = if rbv > 0.0 {
            (rv * rv - rbv).max(0.0) / (rbv + 1e-9)
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
    fn test_rbv_jump_test_creation() {
        let rjt = RbvJumpTest::new(20, 252.0);
        assert!(!rjt.is_ready());
        assert_eq!(rjt.value().main(), 0.0);
    }

    #[test]
    fn test_rbv_jump_test_warmup() {
        let mut rjt = RbvJumpTest::new(20, 252.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rjt.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rjt.is_ready());
    }

    #[test]
    fn test_rbv_jump_test_non_negative() {
        let mut rjt = RbvJumpTest::new(20, 252.0);
        for i in 0..35 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = rjt.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "Jump test value should be non-negative");
        }
    }

    #[test]
    fn test_rbv_jump_test_reset() {
        let mut rjt = RbvJumpTest::new(20, 252.0);
        for i in 0..30 {
            rjt.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        rjt.reset();
        assert!(!rjt.is_ready());
        assert_eq!(rjt.value().main(), 0.0);
    }
}
