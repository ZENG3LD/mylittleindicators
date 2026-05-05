// RSX (Katz/Ehlers RSI) - placeholder smoothed RSI variant

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct Rsx {
    period: usize,
    rsi_gain: MovingAverageProvider,
    rsi_loss: MovingAverageProvider,
    smooth: MovingAverageProvider,
    prev: f64,
    has_prev: bool,
    value: f64,
}

impl Rsx {
    pub fn new(period: usize) -> Self {
        Self {
            period: period.max(1),
            rsi_gain: MovingAverageProvider::new(MovingAverageType::EMA, period.max(1)),
            rsi_loss: MovingAverageProvider::new(MovingAverageType::EMA, period.max(1)),
            smooth: MovingAverageProvider::new(MovingAverageType::EMA, (period / 2).max(1)),
            prev: 0.0,
            has_prev: false,
            value: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.rsi_gain.reset();
        self.rsi_loss.reset();
        self.smooth.reset();
        self.prev = 0.0;
        self.has_prev = false;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.smooth.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> f64 {
        if !self.has_prev {
            self.prev = close;
            self.has_prev = true;
            return self.value;
        }
        let diff = close - self.prev;
        self.prev = close;
        let gain = if diff > 0.0 { diff } else { 0.0 };
        let loss = if diff < 0.0 { -diff } else { 0.0 };
        self.rsi_gain.update_bar(0.0, 0.0, 0.0, gain, 0.0);
        self.rsi_loss.update_bar(0.0, 0.0, 0.0, loss, 0.0);
        let avg_loss = self.rsi_loss.value().main();
        let rsi = if avg_loss.abs() < 1e-12 {
            1.0 // No losses = 100% RSI
        } else {
            let rs = self.rsi_gain.value().main() / avg_loss;
            1.0 - 1.0 / (1.0 + rs)
        };
        self.value = self.smooth.update_bar(0.0, 0.0, 0.0, rsi, 0.0);
        self.value
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsx_creation() {
        let rsx = Rsx::new(14);
        assert!(!rsx.is_ready());
        assert_eq!(rsx.value().main(), 0.0);
        assert_eq!(rsx.period(), 14);
    }

    #[test]
    fn test_rsx_uptrend() {
        let mut rsx = Rsx::new(10);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            rsx.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rsx.is_ready(), "RSX should be ready after 50 bars with period 10");
        // In uptrend, RSX should be > 0.5
        assert!(rsx.value().main() > 0.5, "RSX should be > 0.5 in uptrend, got {}", rsx.value().main());
    }

    #[test]
    fn test_rsx_downtrend() {
        let mut rsx = Rsx::new(10);
        for i in 1..=50 {
            let price = 200.0 - i as f64 * 2.0;
            rsx.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rsx.is_ready(), "RSX should be ready after 50 bars with period 10");
        // In downtrend, RSX should be < 0.5
        assert!(rsx.value().main() < 0.5, "RSX should be < 0.5 in downtrend, got {}", rsx.value().main());
    }

    #[test]
    fn test_rsx_range() {
        let mut rsx = Rsx::new(10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = rsx.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            if rsx.is_ready() {
                assert!(value >= 0.0 && value <= 1.0, "RSX should be in [0, 1], got {}", value);
            }
        }
    }

    #[test]
    fn test_rsx_reset() {
        let mut rsx = Rsx::new(10);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            rsx.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rsx.is_ready(), "RSX should be ready after 50 bars with period 10");
        rsx.reset();
        assert!(!rsx.is_ready());
        assert_eq!(rsx.value().main(), 0.0);
    }
}
