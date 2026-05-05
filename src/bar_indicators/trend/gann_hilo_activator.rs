// Gann HiLo Activator - placeholder using simple SMA of highs/lows

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct GannHiLoActivator {
    ma_high: MovingAverageProvider,
    ma_low: MovingAverageProvider,
    upper: f64,
    lower: f64,
}

impl GannHiLoActivator {
    pub fn new(period: usize) -> Self {
        Self {
            ma_high: MovingAverageProvider::new(MovingAverageType::SMA, period.max(1)),
            ma_low: MovingAverageProvider::new(MovingAverageType::SMA, period.max(1)),
            upper: 0.0,
            lower: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.ma_high.reset();
        self.ma_low.reset();
        self.upper = 0.0;
        self.lower = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ma_high.is_ready() && self.ma_low.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.upper, self.lower)
    }
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, _c: f64, _v: f64) -> (f64, f64) {
        let uh = self.ma_high.update_bar(0.0, h, h, h, 0.0);
        let dl = self.ma_low.update_bar(0.0, l, l, l, 0.0);
        self.upper = uh;
        self.lower = dl;
        (self.upper, self.lower)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gann_hilo_creation() {
        let gann = GannHiLoActivator::new(10);
        assert!(!gann.is_ready());
        assert_eq!(gann.value(), IndicatorValue::Double(0.0, 0.0));
    }

    #[test]
    fn test_gann_hilo_warmup() {
        let mut gann = GannHiLoActivator::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            gann.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(gann.is_ready());
    }

    #[test]
    fn test_gann_hilo_values() {
        let mut gann = GannHiLoActivator::new(10);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            let (upper, lower) = gann.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if gann.is_ready() {
                assert!(upper > lower, "Upper should be > lower");
            }
        }
    }

    #[test]
    fn test_gann_hilo_reset() {
        let mut gann = GannHiLoActivator::new(10);
        for i in 0..15 {
            gann.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        gann.reset();
        assert!(!gann.is_ready());
        assert_eq!(gann.value(), IndicatorValue::Double(0.0, 0.0));
    }
}
