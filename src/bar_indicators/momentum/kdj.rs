// KDJ oscillator: %K, %D (SMA of %K), %J = 3*D - 2*K

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::momentum::stochastics::Stochastics;


#[derive(Clone)]
pub struct Kdj {
    stoch: Stochastics,
    d_ma: MovingAverageProvider,
    k: f64,
    d: f64,
    j: f64,
}

impl Kdj {
    pub fn new(k_period: usize, d_period: usize) -> Self {
        Self {
            stoch: Stochastics::new(k_period.max(1), d_period.max(1)),
            d_ma: MovingAverageProvider::new(MovingAverageType::SMA, d_period.max(1)),
            k: 0.0,
            d: 0.0,
            j: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.stoch.reset();
        self.d_ma.reset();
        self.k = 0.0;
        self.d = 0.0;
        self.j = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.d_ma.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.k, self.d, self.j)
    }
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, _v: f64) -> (f64, f64, f64) {
        let (k_raw, _d_ignore) = self.stoch.update_bar(h, l, c, 0.0);
        self.k = k_raw;
        self.d = self.d_ma.update_bar(0.0, 0.0, 0.0, self.k, 0.0);
        self.j = 3.0 * self.d - 2.0 * self.k;
        (self.k, self.d, self.j)
    }

    pub fn k(&self) -> f64 {
        self.k
    }

    pub fn d(&self) -> f64 {
        self.d
    }

    pub fn j(&self) -> f64 {
        self.j
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kdj_creation() {
        let kdj = Kdj::new(9, 3);
        assert!(!kdj.is_ready());
        if let IndicatorValue::Triple(k, d, j) = kdj.value() {
            assert_eq!(k, 0.0);
            assert_eq!(d, 0.0);
            assert_eq!(j, 0.0);
        } else { panic!("Expected Triple"); }
    }

    #[test]
    fn test_kdj_basic_calculation() {
        let mut kdj = Kdj::new(9, 3);

        for i in 1..=20 {
            let price = 100.0 + i as f64;
            let (k, d, j) = kdj.update_bar(price, price + 2.0, price - 1.0, price + 1.0, 1000.0);

            if kdj.is_ready() {
                assert!(k >= 0.0 && k <= 100.0);
                assert!(d >= 0.0 && d <= 100.0);
                // J can be outside 0-100 range
                assert!(j.is_finite());
            }
        }
    }

    #[test]
    fn test_kdj_uptrend() {
        let mut kdj = Kdj::new(9, 3);

        for i in 1..=25 {
            let price = 100.0 + i as f64;
            kdj.update_bar(price, price + 1.0, price - 0.5, price + 0.5, 1000.0);
        }

        if kdj.is_ready() {
            // In uptrend, K should be high
            assert!(kdj.k() > 50.0, "K in uptrend should be > 50, got {}", kdj.k());
        }
    }

    #[test]
    fn test_kdj_downtrend() {
        let mut kdj = Kdj::new(9, 3);

        for i in 1..=25 {
            let price = 200.0 - i as f64;
            kdj.update_bar(price, price + 0.5, price - 1.0, price - 0.5, 1000.0);
        }

        if kdj.is_ready() {
            // In downtrend, K should be low
            assert!(kdj.k() < 50.0, "K in downtrend should be < 50, got {}", kdj.k());
        }
    }

    #[test]
    fn test_kdj_j_formula() {
        let mut kdj = Kdj::new(9, 3);

        for i in 1..=20 {
            let price = 100.0 + i as f64;
            kdj.update_bar(price, price + 2.0, price - 1.0, price + 1.0, 1000.0);
        }

        // J = 3*D - 2*K
        let expected_j = 3.0 * kdj.d() - 2.0 * kdj.k();
        assert!((kdj.j() - expected_j).abs() < 1e-10);
    }

    #[test]
    fn test_kdj_reset() {
        let mut kdj = Kdj::new(9, 3);

        for i in 1..=20 {
            let price = 100.0 + i as f64;
            kdj.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }

        kdj.reset();
        assert!(!kdj.is_ready());
        assert_eq!(kdj.k(), 0.0);
        assert_eq!(kdj.d(), 0.0);
        assert_eq!(kdj.j(), 0.0);
    }

    #[test]
    fn test_kdj_value() {
        let mut kdj = Kdj::new(9, 3);

        for i in 1..=20 {
            let price = 100.0 + i as f64;
            kdj.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }

        if let IndicatorValue::Triple(k, d, j) = kdj.value() {
            assert_eq!(k, kdj.k());
            assert_eq!(d, kdj.d());
            assert_eq!(j, kdj.j());
        } else { panic!("Expected Triple"); }
    }

    #[test]
    fn test_kdj_getters() {
        let mut kdj = Kdj::new(9, 3);

        for i in 1..=20 {
            let price = 100.0 + i as f64;
            kdj.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }

        assert!(kdj.k().is_finite());
        assert!(kdj.d().is_finite());
        assert!(kdj.j().is_finite());
    }
}
