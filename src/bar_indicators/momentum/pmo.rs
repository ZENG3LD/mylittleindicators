// PMO (Price Momentum Oscillator) - simplified proxy using EMA of ROC

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct Pmo {
    roc_period: usize,
    ma_type: MovingAverageType,
    smooth1: usize,
    smooth2: usize,
    signal_period: usize,
    ema1: MovingAverageProvider,
    ema2: MovingAverageProvider,
    signal_ema: MovingAverageProvider,
    prev_close: f64,
    initialized: bool,
    pmo_value: f64,
    signal_value: f64,
}

impl Pmo {
    pub fn new(roc_period: usize, smooth1: usize, smooth2: usize) -> Self {
        Self::new_with_signal(roc_period, smooth1, smooth2, 10, MovingAverageType::EMA)
    }

    pub fn new_default(roc_period: usize, smooth1: usize, smooth2: usize) -> Self {
        Self::new_with_signal(roc_period, smooth1, smooth2, 10, MovingAverageType::EMA)
    }

    pub fn new_with_ma_type(roc_period: usize, smooth1: usize, smooth2: usize, ma_type: MovingAverageType) -> Self {
        Self::new_with_signal(roc_period, smooth1, smooth2, 10, ma_type)
    }

    pub fn new_with_signal(roc_period: usize, smooth1: usize, smooth2: usize, signal_period: usize, ma_type: MovingAverageType) -> Self {
        let s1 = smooth1.max(1);
        let s2 = smooth2.max(1);
        let sig = signal_period.max(1);
        Self {
            roc_period: roc_period.max(1),
            ma_type,
            smooth1: s1,
            smooth2: s2,
            signal_period: sig,
            ema1: MovingAverageProvider::new(ma_type, s1),
            ema2: MovingAverageProvider::new(ma_type, s2),
            signal_ema: MovingAverageProvider::new(ma_type, sig),
            prev_close: 0.0,
            initialized: false,
            pmo_value: 0.0,
            signal_value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.ema1 = MovingAverageProvider::new(self.ma_type, self.smooth1);
        self.ema2 = MovingAverageProvider::new(self.ma_type, self.smooth2);
        self.signal_ema = MovingAverageProvider::new(self.ma_type, self.signal_period);
        self.prev_close = 0.0;
        self.initialized = false;
        self.pmo_value = 0.0;
        self.signal_value = 0.0;
    }

    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.initialized && self.ema2.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.pmo_value, self.signal_value)
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        if !self.initialized {
            self.prev_close = c;
            self.initialized = true;
            return self.pmo_value;
        }
        let roc = if self.prev_close.abs() > 1e-12 {
            (c - self.prev_close) / self.prev_close * 100.0
        } else {
            0.0
        };
        self.prev_close = c;
        let s1 = self.ema1.update_bar(roc, roc, roc, roc, 0.0);
        self.pmo_value = self.ema2.update_bar(s1, s1, s1, s1, 0.0);
        self.signal_value = self.signal_ema.update_bar(self.pmo_value, self.pmo_value, self.pmo_value, self.pmo_value, 0.0);
        self.pmo_value
    }

    pub fn roc_period(&self) -> usize {
        self.roc_period
    }

    pub fn smooth1_period(&self) -> usize {
        self.smooth1
    }

    pub fn smooth2_period(&self) -> usize {
        self.smooth2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pmo_creation() {
        let pmo = Pmo::new(1, 35, 20);
        assert!(!pmo.is_ready());
        assert_eq!(pmo.value().main(), 0.0);
        assert_eq!(pmo.roc_period(), 1);
        assert_eq!(pmo.smooth1_period(), 35);
        assert_eq!(pmo.smooth2_period(), 20);
    }

    #[test]
    fn test_pmo_default() {
        let pmo = Pmo::new_default(1, 35, 20);
        assert!(!pmo.is_ready());
    }

    #[test]
    fn test_pmo_with_ma_type() {
        let pmo = Pmo::new_with_ma_type(1, 10, 5, MovingAverageType::SMA);
        assert!(!pmo.is_ready());
    }

    #[test]
    fn test_pmo_basic_calculation() {
        let mut pmo = Pmo::new(1, 5, 3);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            pmo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pmo.is_ready());
        // In uptrend, PMO should be positive
        assert!(pmo.value().main() > 0.0, "PMO should be positive in uptrend");
    }

    #[test]
    fn test_pmo_downtrend() {
        let mut pmo = Pmo::new(1, 5, 3);
        for i in 1..=30 {
            let price = 200.0 - i as f64;
            pmo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pmo.is_ready());
        // In downtrend, PMO should be negative
        assert!(pmo.value().main() < 0.0, "PMO should be negative in downtrend");
    }

    #[test]
    fn test_pmo_reset() {
        let mut pmo = Pmo::new(1, 5, 3);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            pmo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pmo.is_ready());
        pmo.reset();
        assert!(!pmo.is_ready());
        assert_eq!(pmo.value().main(), 0.0);
    }

    #[test]
    fn test_pmo_set_ma_type() {
        let mut pmo = Pmo::new(1, 5, 3);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            pmo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pmo.is_ready());
        pmo.set_ma_type(MovingAverageType::SMA);
        assert!(!pmo.is_ready()); // should reset
    }

    #[test]
    fn test_pmo_finite_values() {
        let mut pmo = Pmo::new(1, 10, 5);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            let value = pmo.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "PMO should always be finite");
        }
    }
}
