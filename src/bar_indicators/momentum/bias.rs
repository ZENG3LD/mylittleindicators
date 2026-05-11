// High-performance Bias indicator
// (c) 2024

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

#[derive(Clone)]
pub struct Bias {
    source: OhlcvField,
    ma: MovingAverageProvider,
    value: f64,
    filled: bool,
}

impl Bias {
    pub fn new(period: usize, ma_type: Option<MovingAverageType>) -> Self {
        Self::with_source(period, ma_type, OhlcvField::Close)
    }

    pub fn with_source(period: usize, ma_type: Option<MovingAverageType>, source: OhlcvField) -> Self {
        let ma_type = ma_type.unwrap_or(MovingAverageType::SMA);
        Self {
            source,
            ma: MovingAverageProvider::new(ma_type, period),
            value: 0.0,
            filled: false,
        }
    }
    /// Обновить Bias новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let value = self.source.extract(open, high, low, close, volume);
        self.ma.update_bar(0.0, 0.0, 0.0, value, 0.0);
        if self.ma.is_ready() {
            let ma = self.ma.value().main();
            if ma.abs() < 1e-12 {
                self.value = 0.0;
            } else {
                self.value = value / ma - 1.0;
            }
            self.filled = true;
        } else {
            self.value = 0.0;
            self.filled = false;
        }
        self.value
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn reset(&mut self) {
        self.ma.reset();
        self.value = 0.0;
        self.filled = false;
    }

    pub fn period(&self) -> usize {
        self.ma.period()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bias_creation() {
        let bias = Bias::new(14, None);
        assert!(!bias.is_ready());
        assert_eq!(bias.value().main(), 0.0);
        assert_eq!(bias.period(), 14);
    }

    #[test]
    fn test_bias_with_ema() {
        let bias = Bias::new(10, Some(MovingAverageType::EMA));
        assert!(!bias.is_ready());
        assert_eq!(bias.period(), 10);
    }

    #[test]
    fn test_bias_basic_calculation() {
        let mut bias = Bias::new(5, None);
        // Feed constant price - bias should be 0
        for _ in 0..10 {
            bias.update_bar(100.0, 105.0, 95.0, 100.0, 1000.0);
        }
        assert!(bias.is_ready());
        // close/ma - 1 = 100/100 - 1 = 0
        assert!((bias.value().main()).abs() < 1e-10, "Bias should be 0 for constant prices");
    }

    #[test]
    fn test_bias_uptrend() {
        let mut bias = Bias::new(5, None);
        for i in 1..=20 {
            let price = 100.0 + i as f64 * 2.0;
            bias.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(bias.is_ready());
        // In uptrend, close > MA, so bias > 0
        assert!(bias.value().main() > 0.0, "Bias should be positive in uptrend");
    }

    #[test]
    fn test_bias_downtrend() {
        let mut bias = Bias::new(5, None);
        for i in 1..=20 {
            let price = 200.0 - i as f64 * 2.0;
            bias.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(bias.is_ready());
        // In downtrend, close < MA, so bias < 0
        assert!(bias.value().main() < 0.0, "Bias should be negative in downtrend");
    }

    #[test]
    fn test_bias_reset() {
        let mut bias = Bias::new(5, None);
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            bias.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(bias.is_ready());
        bias.reset();
        assert!(!bias.is_ready());
        assert_eq!(bias.value().main(), 0.0);
    }

    #[test]
    fn test_bias_is_ready_timing() {
        let mut bias = Bias::new(5, None);
        for i in 1..=10 {
            let price = 100.0 + i as f64;
            bias.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if i < 5 {
                assert!(!bias.is_ready(), "Bias should not be ready before period bars");
            } else {
                assert!(bias.is_ready(), "Bias should be ready after period bars");
            }
        }
    }

    #[test]
    fn test_bias_finite_values() {
        let mut bias = Bias::new(10, None);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = bias.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "Bias should always be finite");
        }
    }
}






















