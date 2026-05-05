// Alpha–Beta–Gamma filter - constant acceleration model (placeholder EMA chain)

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct AlphaBetaGammaFilter {
    a: MovingAverageProvider,
    b: MovingAverageProvider,
    g: MovingAverageProvider,
    value: f64,
}

impl AlphaBetaGammaFilter {
    pub fn new(period: usize) -> Self {
        Self {
            a: MovingAverageProvider::new(MovingAverageType::EMA, period.max(1)),
            b: MovingAverageProvider::new(MovingAverageType::EMA, (period / 2).max(1)),
            g: MovingAverageProvider::new(MovingAverageType::EMA, (period / 4).max(1)),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.a.reset();
        self.b.reset();
        self.g.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.a.is_ready() && self.b.is_ready() && self.g.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let p = self.a.update_bar(0.0, 0.0, 0.0, c, 0.0);
        let v = self.b.update_bar(0.0, 0.0, 0.0, p, 0.0);
        let acc = self.g.update_bar(0.0, 0.0, 0.0, v, 0.0);
        self.value = p + v + acc;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alpha_beta_gamma_filter_creation() {
        let filter = AlphaBetaGammaFilter::new(14);
        assert!(!filter.is_ready());
        assert_eq!(filter.value().main(), 0.0);
    }

    #[test]
    fn test_alpha_beta_gamma_filter_warmup() {
        let mut filter = AlphaBetaGammaFilter::new(10);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            filter.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(filter.is_ready());
    }

    #[test]
    fn test_alpha_beta_gamma_filter_values_finite() {
        let mut filter = AlphaBetaGammaFilter::new(10);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = filter.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_alpha_beta_gamma_filter_reset() {
        let mut filter = AlphaBetaGammaFilter::new(10);
        for i in 0..20 {
            filter.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        filter.reset();
        assert!(!filter.is_ready());
        assert_eq!(filter.value().main(), 0.0);
    }
}
