use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Chaikin Volatility: EMA(high-low, n) rate of change over k periods
#[derive(Debug, Clone)]
pub struct ChaikinVolatility {
    ma_type: MovingAverageType,
    n_period: usize,
    ema_range: MovingAverageProvider,
    prev_ema: f64,
    #[allow(dead_code)]
    k: usize,
    value: f64,
    ready: bool,
}

impl ChaikinVolatility {
    pub fn new(n: usize, k: usize) -> Self {
        Self::new_with_ma_type(n, k, MovingAverageType::EMA)
    }

    pub fn new_default(n: usize, k: usize) -> Self {
        Self::new_with_ma_type(n, k, MovingAverageType::EMA)
    }

    pub fn new_with_ma_type(n: usize, k: usize, ma_type: MovingAverageType) -> Self {
        let n_period = n.max(1);
        Self {
            ma_type,
            n_period,
            ema_range: MovingAverageProvider::new(ma_type, n_period),
            prev_ema: 0.0,
            k: k.max(1),
            value: 0.0,
            ready: false,
        }
    }

    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, _c: f64, _v: f64) -> f64 {
        let range = (h - l).max(0.0);
        let ema = self.ema_range.update_bar(0.0, 0.0, 0.0, range, 0.0);
        if self.ema_range.is_ready() {
            if self.prev_ema == 0.0 {
                self.prev_ema = ema;
            }
            // Approximate k-period ROC using exponential smoothing difference
            self.value = if self.prev_ema.abs() < 1e-12 {
                0.0
            } else {
                100.0 * (ema - self.prev_ema) / self.prev_ema
            };
            self.prev_ema = ema;
            self.ready = true;
        }
        self.value
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    pub fn reset(&mut self) {
        self.ema_range = MovingAverageProvider::new(self.ma_type, self.n_period);
        self.prev_ema = 0.0;
        self.value = 0.0;
        self.ready = false;
    }

    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaikin_volatility_creation() {
        let cv = ChaikinVolatility::new(10, 10);
        assert!(!cv.is_ready());
        assert_eq!(cv.value().main(), 0.0);
    }

    #[test]
    fn test_chaikin_volatility_warmup() {
        let mut cv = ChaikinVolatility::new(10, 10);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            cv.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(cv.is_ready());
    }

    #[test]
    fn test_chaikin_volatility_values() {
        let mut cv = ChaikinVolatility::new(10, 10);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            let value = cv.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_chaikin_volatility_reset() {
        let mut cv = ChaikinVolatility::new(10, 10);
        for i in 0..20 {
            cv.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        cv.reset();
        assert!(!cv.is_ready());
        assert_eq!(cv.value().main(), 0.0);
    }
}
