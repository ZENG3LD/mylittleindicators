// Minimalistic MA Cross indicator for Nemo
// (c) 2024
use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct MaCross {
    fast_period: usize,
    slow_period: usize,
    fast_type: MovingAverageType,  // Store catalog types
    slow_type: MovingAverageType,
    fast_ma: MovingAverageProvider,
    slow_ma: MovingAverageProvider,
    prev_trend: i8, // Предыдущий значимый сигнал (1 или -1)
    last_trend: i8, // Текущий сигнал (1, -1, 0)
    ready: bool,
}

impl MaCross {
    pub fn new(fast_period: usize, slow_period: usize, fast_type: MovingAverageType, slow_type: MovingAverageType) -> Self {
        Self {
            fast_period,
            slow_period,
            fast_type,
            slow_type,
            fast_ma: MovingAverageProvider::new(fast_type, fast_period),
            slow_ma: MovingAverageProvider::new(slow_type, slow_period),
            prev_trend: 0,
            last_trend: 0,
            ready: false,
        }
    }

    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> i8 {
        let fast = self.fast_ma.update_bar(0.0, 0.0, 0.0, close, 0.0);
        let slow = self.slow_ma.update_bar(0.0, 0.0, 0.0, close, 0.0);
        if self.slow_ma.is_ready() {
            let new_trend = if fast > slow { 1 } else if fast < slow { -1 } else { 0 };
            if new_trend != 0 && new_trend != self.prev_trend {
                self.last_trend = new_trend;
                self.prev_trend = new_trend;
            }
            self.ready = true;
        }
        self.last_trend
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_trend)
    }
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    pub fn reset(&mut self) {
        // Use stored types directly - no pattern matching on MovingAverage enum
        self.fast_ma = MovingAverageProvider::new(self.fast_type, self.fast_period);
        self.slow_ma = MovingAverageProvider::new(self.slow_type, self.slow_period);
        self.prev_trend = 0;
        self.last_trend = 0;
        self.ready = false;
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
    fn test_ma_cross_creation() {
        let mac = MaCross::new(9, 21, MovingAverageType::EMA, MovingAverageType::EMA);
        assert!(!mac.is_ready());
        assert_eq!(mac.fast_period(), 9);
        assert_eq!(mac.slow_period(), 21);
    }

    #[test]
    fn test_ma_cross_uptrend() {
        let mut mac = MaCross::new(9, 21, MovingAverageType::EMA, MovingAverageType::EMA);
        for i in 1..=40 {
            let price = 100.0 + i as f64 * 2.0;
            mac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(mac.is_ready());
        // In uptrend, fast > slow, signal should be 1
        match mac.value() {
            IndicatorValue::Signal(s) => assert_eq!(s, 1, "MA Cross should signal 1 in uptrend"),
            _ => panic!("MA Cross should return Signal value"),
        }
    }

    #[test]
    fn test_ma_cross_downtrend() {
        let mut mac = MaCross::new(9, 21, MovingAverageType::EMA, MovingAverageType::EMA);
        for i in 1..=40 {
            let price = 200.0 - i as f64 * 2.0;
            mac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(mac.is_ready());
        // In downtrend, fast < slow, signal should be -1
        match mac.value() {
            IndicatorValue::Signal(s) => assert_eq!(s, -1, "MA Cross should signal -1 in downtrend"),
            _ => panic!("MA Cross should return Signal value"),
        }
    }

    #[test]
    fn test_ma_cross_reset() {
        let mut mac = MaCross::new(9, 21, MovingAverageType::EMA, MovingAverageType::EMA);
        for i in 1..=40 {
            let price = 100.0 + i as f64;
            mac.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(mac.is_ready());
        mac.reset();
        assert!(!mac.is_ready());
    }

    #[test]
    fn test_ma_cross_signal_range() {
        let mut mac = MaCross::new(9, 21, MovingAverageType::EMA, MovingAverageType::EMA);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let signal = mac.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(signal >= -1 && signal <= 1, "MA Cross signal should be -1, 0, or 1, got {}", signal);
        }
    }
}






















