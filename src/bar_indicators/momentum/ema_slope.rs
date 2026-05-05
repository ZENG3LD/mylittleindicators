// EMA Slope: normalized slope of MA over lookback

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct EmaSlope {
    ma_period: usize,
    ma_type: MovingAverageType,
    ema: MovingAverageProvider,
    lookback: usize,
    buffer: Vec<f64>,
    idx: usize,
    filled: bool,
    slope: f64,
}

impl EmaSlope {
    /// Create EMA Slope with default MA type (EMA)
    pub fn new(ema_period: usize, lookback: usize) -> Self {
        Self::new_with_ma_type(ema_period, lookback, MovingAverageType::EMA)
    }

    /// Create EMA Slope with specified MA type
    pub fn new_with_ma_type(ma_period: usize, lookback: usize, ma_type: MovingAverageType) -> Self {
        let lb = lookback.max(1);
        Self {
            ma_period,
            ma_type,
            ema: MovingAverageProvider::new(ma_type, ma_period),
            lookback: lb,
            buffer: vec![0.0; lb],
            idx: 0,
            filled: false,
            slope: 0.0,
        }
    }

    /// Set the MA type and reset the indicator
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ema = MovingAverageProvider::new(self.ma_type, self.ma_period);
        self.buffer.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.slope = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ema.is_ready() && (self.filled || self.idx >= self.lookback)
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let v = self.ema.update_bar(open, high, low, close, volume);
        self.buffer[self.idx % self.lookback] = v;
        self.idx += 1;
        if self.idx >= self.lookback {
            self.filled = true;
        }

        if self.is_ready() {
            let last = v;
            let first = self.buffer[(self.idx - self.lookback) % self.lookback];
            let denom = (self.lookback as f64).max(1.0);
            self.slope = (last - first) / denom;
        } else {
            self.slope = 0.0;
        }
        self.slope
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.slope)
    }

    pub fn ma_period(&self) -> usize {
        self.ma_period
    }

    pub fn lookback(&self) -> usize {
        self.lookback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema_slope_creation() {
        let es = EmaSlope::new(13, 5);
        assert!(!es.is_ready());
        assert_eq!(es.value().main(), 0.0);
        assert_eq!(es.ma_period(), 13);
        assert_eq!(es.lookback(), 5);
    }

    #[test]
    fn test_ema_slope_with_ma_type() {
        let es = EmaSlope::new_with_ma_type(13, 5, MovingAverageType::SMA);
        assert_eq!(es.ma_period(), 13);
        assert_eq!(es.lookback(), 5);
    }

    #[test]
    fn test_ema_slope_uptrend() {
        let mut es = EmaSlope::new(10, 5);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            es.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(es.is_ready());
        // In uptrend, slope should be positive
        assert!(es.value().main() > 0.0, "EMA slope should be positive in uptrend, got {}", es.value().main());
    }

    #[test]
    fn test_ema_slope_downtrend() {
        let mut es = EmaSlope::new(10, 5);
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            es.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(es.is_ready());
        // In downtrend, slope should be negative
        assert!(es.value().main() < 0.0, "EMA slope should be negative in downtrend, got {}", es.value().main());
    }

    #[test]
    fn test_ema_slope_reset() {
        let mut es = EmaSlope::new(10, 5);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            es.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(es.is_ready());
        es.reset();
        assert!(!es.is_ready());
        assert_eq!(es.value().main(), 0.0);
    }

    #[test]
    fn test_ema_slope_finite_values() {
        let mut es = EmaSlope::new(10, 5);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = es.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "EMA slope should always be finite");
        }
    }

    #[test]
    fn test_ema_slope_set_ma_type() {
        let mut es = EmaSlope::new(10, 5);
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            es.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(es.is_ready());
        es.set_ma_type(MovingAverageType::SMA);
        assert!(!es.is_ready()); // should reset
    }
}
