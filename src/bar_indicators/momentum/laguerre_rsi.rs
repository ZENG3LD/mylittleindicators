// Laguerre RSI (LRSI) - lightweight implementation
// Placeholder implementation: basic EMA-based RSI proxy to compile and be wired

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct LaguerreRsi {
    period: usize,
    gain_ma: MovingAverageProvider,
    loss_ma: MovingAverageProvider,
    prev: f64,
    has_prev: bool,
    value: f64,
}

impl LaguerreRsi {
    pub fn new(period: usize) -> Self {
        Self {
            period: period.max(1),
            gain_ma: MovingAverageProvider::new(MovingAverageType::EMA, period.max(1)),
            loss_ma: MovingAverageProvider::new(MovingAverageType::EMA, period.max(1)),
            prev: 0.0,
            has_prev: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.gain_ma.reset();
        self.loss_ma.reset();
        self.prev = 0.0;
        self.has_prev = false;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.gain_ma.is_ready() && self.loss_ma.is_ready()
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
        self.gain_ma.update_bar(0.0, 0.0, 0.0, gain, 0.0);
        self.loss_ma.update_bar(0.0, 0.0, 0.0, loss, 0.0);

        if self.is_ready() {
            let avg_gain = self.gain_ma.value().main();
            let avg_loss = self.loss_ma.value().main();
            if avg_loss.abs() < 1e-12 {
                self.value = 1.0;
            } else {
                let rs = avg_gain / avg_loss;
                self.value = 1.0 - 1.0 / (1.0 + rs);
            }
        }
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
    fn test_laguerre_rsi_creation() {
        let lrsi = LaguerreRsi::new(14);
        assert!(!lrsi.is_ready());
        assert_eq!(lrsi.value().main(), 0.0);
        assert_eq!(lrsi.period(), 14);
    }

    #[test]
    fn test_laguerre_rsi_uptrend() {
        let mut lrsi = LaguerreRsi::new(10);
        for i in 1..=40 {
            let price = 100.0 + i as f64 * 2.0;
            lrsi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(lrsi.is_ready());
        // In uptrend, gains > losses, so LRSI should be high
        assert!(lrsi.value().main() > 0.5, "Laguerre RSI should be > 0.5 in uptrend, got {}", lrsi.value().main());
    }

    #[test]
    fn test_laguerre_rsi_downtrend() {
        let mut lrsi = LaguerreRsi::new(10);
        for i in 1..=40 {
            let price = 200.0 - i as f64 * 2.0;
            lrsi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(lrsi.is_ready());
        // In downtrend, losses > gains, so LRSI should be low
        assert!(lrsi.value().main() < 0.5, "Laguerre RSI should be < 0.5 in downtrend, got {}", lrsi.value().main());
    }

    #[test]
    fn test_laguerre_rsi_range() {
        let mut lrsi = LaguerreRsi::new(10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = lrsi.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            if lrsi.is_ready() {
                assert!(value >= 0.0 && value <= 1.0, "Laguerre RSI should be in [0, 1], got {}", value);
            }
        }
    }

    #[test]
    fn test_laguerre_rsi_reset() {
        let mut lrsi = LaguerreRsi::new(10);
        for i in 1..=40 {
            let price = 100.0 + i as f64;
            lrsi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(lrsi.is_ready());
        lrsi.reset();
        assert!(!lrsi.is_ready());
        assert_eq!(lrsi.value().main(), 0.0);
    }
}
