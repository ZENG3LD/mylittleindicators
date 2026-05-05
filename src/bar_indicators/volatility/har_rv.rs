// HAR-RV: Heterogeneous AutoRegressive model of Realized Volatility proxy
// Minimal online proxy: combine short/medium/long horizon RVs

use crate::bar_indicators::volatility::realized_vol::RealizedVol;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct HarRv {
    d: RealizedVol,
    w: RealizedVol,
    m: RealizedVol,
    value: f64,
}

impl HarRv {
    pub fn new(day_win: usize, week_win: usize, month_win: usize, annualize_factor: f64) -> Self {
        Self {
            d: RealizedVol::new(day_win.max(1), annualize_factor),
            w: RealizedVol::new(week_win.max(1), annualize_factor),
            m: RealizedVol::new(month_win.max(1), annualize_factor),
            value: 0.0,
        }
    }
    pub fn reset(&mut self) {
        self.d.reset();
        self.w.reset();
        self.m.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.d.is_ready() && self.w.is_ready() && self.m.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let rd = self.d.update_bar(open, high, low, close, volume);
        let rw = self.w.update_bar(open, high, low, close, volume);
        let rm = self.m.update_bar(open, high, low, close, volume);
        // Simple convex combo; coefficients can be tuned offline
        self.value = 0.6 * rd + 0.3 * rw + 0.1 * rm;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_har_rv_creation() {
        let har = HarRv::new(5, 20, 60, 252.0);
        assert!(!har.is_ready());
        assert_eq!(har.value().main(), 0.0);
    }

    #[test]
    fn test_har_rv_warmup() {
        let mut har = HarRv::new(5, 20, 60, 252.0);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            har.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(har.is_ready());
    }

    #[test]
    fn test_har_rv_values() {
        let mut har = HarRv::new(5, 20, 60, 252.0);
        for i in 0..70 {
            let price = 100.0 + i as f64;
            let value = har.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_har_rv_reset() {
        let mut har = HarRv::new(5, 20, 60, 252.0);
        for i in 0..70 {
            har.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        har.reset();
        assert!(!har.is_ready());
        assert_eq!(har.value().main(), 0.0);
    }
}
