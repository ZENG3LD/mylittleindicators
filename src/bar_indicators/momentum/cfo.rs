use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Chande Forecast Oscillator (CFO): close - LinearRegressionForecast, smoothed by MA
#[derive(Debug, Clone)]
pub struct Cfo {
    period: usize,
    ma_type: MovingAverageType,
    lr_ma: MovingAverageProvider,
    value: f64,
}

impl Cfo {
    /// Create CFO with default MA type (SMA)
    pub fn new(period: usize) -> Self {
        Self::new_with_ma_type(period, MovingAverageType::SMA)
    }

    /// Create CFO with specified MA type
    pub fn new_with_ma_type(period: usize, ma_type: MovingAverageType) -> Self {
        let p = period.max(2);
        Self {
            period: p,
            ma_type,
            lr_ma: MovingAverageProvider::new(ma_type, p),
            value: 0.0,
        }
    }

    /// Set the MA type and reset the indicator
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let forecast = self.lr_ma.update_bar(0.0, 0.0, 0.0, c, 0.0);
        self.value = c - forecast;
        self.value
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.lr_ma.is_ready()
    }
    pub fn reset(&mut self) {
        self.lr_ma = MovingAverageProvider::new(self.ma_type, self.period);
        self.value = 0.0;
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cfo_creation() {
        let cfo = Cfo::new(14);
        assert!(!cfo.is_ready());
        assert_eq!(cfo.value().main(), 0.0);
        assert_eq!(cfo.period(), 14);
    }

    #[test]
    fn test_cfo_with_ema() {
        let cfo = Cfo::new_with_ma_type(10, MovingAverageType::EMA);
        assert!(!cfo.is_ready());
        assert_eq!(cfo.period(), 10);
    }

    #[test]
    fn test_cfo_min_period() {
        let cfo = Cfo::new(1);
        assert_eq!(cfo.period(), 2); // min period is 2
    }

    #[test]
    fn test_cfo_basic_calculation() {
        let mut cfo = Cfo::new(5);
        // Feed constant price - CFO should be 0 (close == forecast)
        for _ in 0..20 {
            cfo.update_bar(100.0, 105.0, 95.0, 100.0, 1000.0);
        }
        assert!(cfo.is_ready());
        assert!((cfo.value().main()).abs() < 1e-10, "CFO should be 0 for constant prices");
    }

    #[test]
    fn test_cfo_uptrend() {
        let mut cfo = Cfo::new(5);
        for i in 1..=20 {
            let price = 100.0 + i as f64 * 2.0;
            cfo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(cfo.is_ready());
        // In uptrend, close > forecast, so CFO > 0
        assert!(cfo.value().main() > 0.0, "CFO should be positive in uptrend");
    }

    #[test]
    fn test_cfo_downtrend() {
        let mut cfo = Cfo::new(5);
        for i in 1..=20 {
            let price = 200.0 - i as f64 * 2.0;
            cfo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(cfo.is_ready());
        // In downtrend, close < forecast, so CFO < 0
        assert!(cfo.value().main() < 0.0, "CFO should be negative in downtrend");
    }

    #[test]
    fn test_cfo_reset() {
        let mut cfo = Cfo::new(5);
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            cfo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(cfo.is_ready());
        cfo.reset();
        assert!(!cfo.is_ready());
        assert_eq!(cfo.value().main(), 0.0);
    }

    #[test]
    fn test_cfo_set_ma_type() {
        let mut cfo = Cfo::new(10);
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            cfo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(cfo.is_ready());
        cfo.set_ma_type(MovingAverageType::EMA);
        assert!(!cfo.is_ready()); // should reset
    }

    #[test]
    fn test_cfo_finite_values() {
        let mut cfo = Cfo::new(10);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = cfo.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "CFO should always be finite");
        }
    }
}
