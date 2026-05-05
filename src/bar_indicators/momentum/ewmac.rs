// EWMAC (Exponential Weighted Moving Average Crossover) signal

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct Ewmac {
    fast_period: usize,
    slow_period: usize,
    ma_type: MovingAverageType,
    fast: MovingAverageProvider,
    slow: MovingAverageProvider,
    value: f64,
}

impl Ewmac {
    /// Create EWMAC with default MA type (EMA)
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self::new_with_ma_type(fast_period, slow_period, MovingAverageType::EMA)
    }

    /// Create EWMAC with specified MA type
    pub fn new_with_ma_type(fast_period: usize, slow_period: usize, ma_type: MovingAverageType) -> Self {
        Self {
            fast_period,
            slow_period,
            ma_type,
            fast: MovingAverageProvider::new(ma_type, fast_period),
            slow: MovingAverageProvider::new(ma_type, slow_period),
            value: 0.0,
        }
    }

    /// Set the MA type and reset the indicator
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }

    #[inline]
    pub fn reset(&mut self) {
        self.fast = MovingAverageProvider::new(self.ma_type, self.fast_period);
        self.slow = MovingAverageProvider::new(self.ma_type, self.slow_period);
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.fast.is_ready() && self.slow.is_ready()
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> f64 {
        let f = self.fast.update_bar(0.0, 0.0, 0.0, close, 0.0);
        let s = self.slow.update_bar(0.0, 0.0, 0.0, close, 0.0);
        self.value = f - s;
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn fast_period(&self) -> usize {
        self.fast_period
    }

    pub fn slow_period(&self) -> usize {
        self.slow_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ewmac_creation() {
        let ewmac = Ewmac::new(8, 32);
        assert!(!ewmac.is_ready());
        assert_eq!(ewmac.value().main(), 0.0);
        assert_eq!(ewmac.fast_period(), 8);
        assert_eq!(ewmac.slow_period(), 32);
    }

    #[test]
    fn test_ewmac_uptrend() {
        let mut ewmac = Ewmac::new(8, 32);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            ewmac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ewmac.is_ready());
        // In uptrend, fast EMA > slow EMA, so EWMAC > 0
        assert!(ewmac.value().main() > 0.0, "EWMAC should be positive in uptrend, got {}", ewmac.value().main());
    }

    #[test]
    fn test_ewmac_downtrend() {
        let mut ewmac = Ewmac::new(8, 32);
        for i in 1..=50 {
            let price = 200.0 - i as f64 * 2.0;
            ewmac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ewmac.is_ready());
        // In downtrend, fast EMA < slow EMA, so EWMAC < 0
        assert!(ewmac.value().main() < 0.0, "EWMAC should be negative in downtrend, got {}", ewmac.value().main());
    }

    #[test]
    fn test_ewmac_reset() {
        let mut ewmac = Ewmac::new(8, 32);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            ewmac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ewmac.is_ready());
        ewmac.reset();
        assert!(!ewmac.is_ready());
        assert_eq!(ewmac.value().main(), 0.0);
    }

    #[test]
    fn test_ewmac_finite_values() {
        let mut ewmac = Ewmac::new(8, 32);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = ewmac.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "EWMAC should always be finite");
        }
    }

    #[test]
    fn test_ewmac_set_ma_type() {
        let mut ewmac = Ewmac::new(8, 32);
        for i in 1..=40 {
            let price = 100.0 + i as f64;
            ewmac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ewmac.is_ready());
        ewmac.set_ma_type(MovingAverageType::SMA);
        assert!(!ewmac.is_ready()); // should reset
    }
}
