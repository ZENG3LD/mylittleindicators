// Robust EWMAC: median-smoothed price and MAD-normalized signal

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::utils::math::percentile::median;

#[derive(Clone)]
pub struct EwmacRobust {
    fast_period: usize,
    slow_period: usize,
    ma_type: MovingAverageType,
    fast: MovingAverageProvider,
    slow: MovingAverageProvider,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl EwmacRobust {
    /// Create EWMAC Robust with default MA type (EMA)
    pub fn new(fast_period: usize, slow_period: usize, robust_window: usize) -> Self {
        Self::new_with_ma_type(fast_period, slow_period, robust_window, MovingAverageType::EMA)
    }

    /// Create EWMAC Robust with specified MA type
    pub fn new_with_ma_type(fast_period: usize, slow_period: usize, robust_window: usize, ma_type: MovingAverageType) -> Self {
        let window = robust_window.max(15);
        Self {
            fast_period,
            slow_period,
            ma_type,
            fast: MovingAverageProvider::new(ma_type, fast_period),
            slow: MovingAverageProvider::new(ma_type, slow_period),
            window,
            buf: vec![0.0; window],
            idx: 0,
            filled: false,
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
        self.idx = 0;
        self.filled = false;
        self.buf.fill(0.0);
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.fast.is_ready() && self.slow.is_ready()
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Optimized median using O(n) quickselect instead of O(n log n) sort
    fn median_value(&self) -> f64 {
        let mut v = self.buf.clone();
        median(&mut v)
    }

    /// Optimized MAD using O(n) quickselect instead of O(n log n) sort
    fn mad(&self, med: f64) -> f64 {
        let mut d: Vec<f64> = self.buf.iter().map(|x| (x - med).abs()).collect();
        let m = median(&mut d);
        (m * 1.4826).max(1e-9)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        self.buf[self.idx] = c;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        let f = self.fast.update_bar(0.0, 0.0, 0.0, c, 0.0);
        let s = self.slow.update_bar(0.0, 0.0, 0.0, c, 0.0);
        let raw = f - s;
        if self.filled {
            let med = self.median_value();
            let scale = self.mad(med);
            self.value = (raw / scale).tanh();
        } else {
            self.value = 0.0;
        }
        self.value
    }

    pub fn fast_period(&self) -> usize {
        self.fast_period
    }

    pub fn slow_period(&self) -> usize {
        self.slow_period
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ewmac_robust_creation() {
        let ewmac = EwmacRobust::new(8, 32, 30);
        assert!(!ewmac.is_ready());
        assert_eq!(ewmac.value, 0.0);
        assert_eq!(ewmac.fast_period(), 8);
        assert_eq!(ewmac.slow_period(), 32);
    }

    #[test]
    fn test_ewmac_robust_uptrend() {
        let mut ewmac = EwmacRobust::new(8, 32, 30);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            ewmac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ewmac.is_ready());
        assert!(ewmac.value > 0.0, "EWMAC Robust should be > 0 in uptrend, got {}", ewmac.value);
    }

    #[test]
    fn test_ewmac_robust_range() {
        let mut ewmac = EwmacRobust::new(8, 32, 30);
        for i in 1..=60 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = ewmac.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            if ewmac.is_ready() {
                assert!(value >= -1.0 && value <= 1.0, "EWMAC Robust should be in [-1, 1], got {}", value);
            }
        }
    }

    #[test]
    fn test_ewmac_robust_reset() {
        let mut ewmac = EwmacRobust::new(8, 32, 30);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            ewmac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ewmac.is_ready());
        ewmac.reset();
        assert!(!ewmac.is_ready());
        assert_eq!(ewmac.value, 0.0);
    }
}
