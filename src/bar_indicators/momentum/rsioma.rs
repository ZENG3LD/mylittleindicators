// RSIOMA - RSI of Moving Average or MA of RSI combined

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::momentum::rsi::Rsi;
use crate::bar_indicators::indicator_value::IndicatorValue;


#[derive(Clone)]
pub struct RsiOma {
    rsi: Rsi,
    ema: MovingAverageProvider,
    value: f64,
}

impl RsiOma {
    pub fn new(rsi_period: usize, ema_period: usize) -> Self {
        Self::with_ma_type(rsi_period, ema_period, MovingAverageType::EMA)
    }

    /// Create RSIOMA with configurable outer MA type.
    ///
    /// # Arguments
    /// * `rsi_period`  - RSI lookback period
    /// * `ma_period`   - Outer MA smoothing period
    /// * `ma_type`     - MA type for outer smoothing (default EMA)
    pub fn with_ma_type(rsi_period: usize, ma_period: usize, ma_type: MovingAverageType) -> Self {
        Self {
            rsi: Rsi::new(rsi_period.max(1)),
            ema: MovingAverageProvider::new(ma_type, ma_period.max(1)),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.rsi.reset();
        self.ema.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ema.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let _ = self.rsi.update_bar(o, h, l, c, v);
        self.value = self.ema.update_bar(0.0, 0.0, 0.0, self.rsi.value().main(), 0.0);
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsioma_creation() {
        let rsioma = RsiOma::new(14, 9);
        assert!(!rsioma.is_ready());
        assert_eq!(rsioma.value().main(), 0.0);
    }

    #[test]
    fn test_rsioma_uptrend() {
        let mut rsioma = RsiOma::new(14, 9);
        for i in 1..=40 {
            let price = 100.0 + i as f64 * 2.0;
            rsioma.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rsioma.is_ready());
        // In uptrend, RSI > 0.5 (50%), so smoothed RSI should also be > 0.5
        assert!(rsioma.value().main() > 0.5, "RSIOMA should be > 0.5 in uptrend, got {}", rsioma.value().main());
    }

    #[test]
    fn test_rsioma_downtrend() {
        let mut rsioma = RsiOma::new(14, 9);
        for i in 1..=40 {
            let price = 200.0 - i as f64 * 2.0;
            rsioma.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rsioma.is_ready());
        // In downtrend, RSI < 0.5 (50%), so smoothed RSI should also be < 0.5
        assert!(rsioma.value().main() < 0.5, "RSIOMA should be < 0.5 in downtrend, got {}", rsioma.value().main());
    }

    #[test]
    fn test_rsioma_reset() {
        let mut rsioma = RsiOma::new(14, 9);
        for i in 1..=40 {
            let price = 100.0 + i as f64;
            rsioma.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rsioma.is_ready());
        rsioma.reset();
        assert!(!rsioma.is_ready());
        assert_eq!(rsioma.value().main(), 0.0);
    }

    #[test]
    fn test_rsioma_finite_values() {
        let mut rsioma = RsiOma::new(14, 9);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = rsioma.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "RSIOMA should always be finite");
        }
    }

    #[test]
    fn test_rsioma_with_ma_type() {
        let mut rsioma = RsiOma::with_ma_type(14, 9, MovingAverageType::SMA);
        for i in 1..=40 {
            let p = 100.0 + i as f64 * 0.5;
            let v = rsioma.update_bar(p, p + 1.0, p - 1.0, p, 1000.0);
            assert!(v.is_finite());
        }
        assert!(rsioma.is_ready());
    }
}
