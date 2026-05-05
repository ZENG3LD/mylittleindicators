// Traders Dynamic Index (TDI) - placeholder using RSI smoothed lines

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::momentum::rsi::Rsi;
use crate::bar_indicators::indicator_value::IndicatorValue;


#[derive(Clone)]
pub struct Tdi {
    rsi: Rsi,
    signal_ma: MovingAverageProvider,
    band_ma: MovingAverageProvider,
    rsi_value: f64,
    signal_value: f64,
    band_value: f64,
}

impl Tdi {
    pub fn new(rsi_period: usize, signal_period: usize, band_period: usize) -> Self {
        Self {
            rsi: Rsi::new(rsi_period.max(1)),
            signal_ma: MovingAverageProvider::new(MovingAverageType::EMA, signal_period.max(1)),
            band_ma: MovingAverageProvider::new(MovingAverageType::EMA, band_period.max(1)),
            rsi_value: 50.0,
            signal_value: 50.0,
            band_value: 50.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.rsi.reset();
        self.signal_ma.reset();
        self.band_ma.reset();
        self.rsi_value = 50.0;
        self.signal_value = 50.0;
        self.band_value = 50.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rsi.is_ready()
    }
    #[inline]
    pub fn values(&self) -> (f64, f64, f64) {
        (self.rsi_value, self.signal_value, self.band_value)
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.rsi_value, self.signal_value, self.band_value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> (f64, f64, f64) {
        let r = self.rsi.update_bar(o, h, l, c, v);
        self.rsi_value = r;
        self.signal_value = self.signal_ma.update_bar(0.0, 0.0, 0.0, r, 0.0);
        self.band_value = self.band_ma.update_bar(0.0, 0.0, 0.0, r, 0.0);
        (self.rsi_value, self.signal_value, self.band_value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tdi_creation() {
        let tdi = Tdi::new(14, 9, 5);
        assert!(!tdi.is_ready());
        assert_eq!(tdi.values(), (50.0, 50.0, 50.0));
    }

    #[test]
    fn test_tdi_uptrend() {
        let mut tdi = Tdi::new(14, 9, 5);
        for i in 1..=40 {
            let price = 100.0 + i as f64 * 2.0;
            tdi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(tdi.is_ready());
        let (rsi, _, _) = tdi.values();
        assert!(rsi > 0.5, "TDI RSI should be > 0.5 in uptrend, got {}", rsi);
    }

    #[test]
    fn test_tdi_downtrend() {
        let mut tdi = Tdi::new(14, 9, 5);
        for i in 1..=40 {
            let price = 200.0 - i as f64 * 2.0;
            tdi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(tdi.is_ready());
        let (rsi, _, _) = tdi.values();
        assert!(rsi < 0.5, "TDI RSI should be < 0.5 in downtrend, got {}", rsi);
    }

    #[test]
    fn test_tdi_finite_values() {
        let mut tdi = Tdi::new(14, 9, 5);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let (rsi, signal, band) = tdi.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(rsi.is_finite() && signal.is_finite() && band.is_finite());
        }
    }

    #[test]
    fn test_tdi_reset() {
        let mut tdi = Tdi::new(14, 9, 5);
        for i in 1..=40 {
            let price = 100.0 + i as f64;
            tdi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(tdi.is_ready());
        tdi.reset();
        assert!(!tdi.is_ready());
        assert_eq!(tdi.values(), (50.0, 50.0, 50.0));
    }
}
