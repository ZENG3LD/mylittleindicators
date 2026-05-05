// High-performance Archer Moving Averages Trends (AMAT)
// (c) 2024
use std::collections::VecDeque;
use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct Amat {
    pub fast_period: usize,
    pub slow_period: usize,
    pub signal_period: usize,
    pub long_run: bool,
    pub short_run: bool,
    pub initialized: bool,
    fast_type: MovingAverageType,  // Store catalog types
    slow_type: MovingAverageType,
    fast_ma: MovingAverageProvider,
    slow_ma: MovingAverageProvider,
    fast_ma_price: VecDeque<f64>,
    slow_ma_price: VecDeque<f64>,
    has_inputs: bool,
}




impl Amat {
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize, fast_type: MovingAverageType, slow_type: MovingAverageType) -> Self {
        Self {
            fast_period,
            slow_period,
            signal_period,
            long_run: false,
            short_run: false,
            initialized: false,
            fast_type,
            slow_type,
            fast_ma: MovingAverageProvider::new(fast_type, fast_period),
            slow_ma: MovingAverageProvider::new(slow_type, slow_period),
            fast_ma_price: VecDeque::with_capacity(signal_period + 1),
            slow_ma_price: VecDeque::with_capacity(signal_period + 1),
            has_inputs: false,
        }
    }

    /// Обновить AMAT новым баром (используется close)
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> i8 {
        self.fast_ma.update_bar(0.0, 0.0, 0.0, close, 0.0);
        self.slow_ma.update_bar(0.0, 0.0, 0.0, close, 0.0);
        if self.slow_ma.is_ready() {
            self.fast_ma_price.push_back(self.fast_ma.value().main());
            self.slow_ma_price.push_back(self.slow_ma.value().main());
            if self.fast_ma_price.len() > self.signal_period + 1 {
                self.fast_ma_price.pop_front();
            }
            if self.slow_ma_price.len() > self.signal_period + 1 {
                self.slow_ma_price.pop_front();
            }
            let fast_back = self.fast_ma.value().main();
            let slow_back = self.slow_ma.value().main();
            let fast_front = *self.fast_ma_price.front().unwrap();
            let slow_front = *self.slow_ma_price.front().unwrap();
            self.long_run = fast_back - fast_front > 0.0 && slow_back - slow_front < 0.0;
            self.long_run = fast_back - fast_front > 0.0 && slow_back - slow_front > 0.0 || self.long_run;
            self.short_run = fast_back - fast_front < 0.0 && slow_back - slow_front > 0.0;
            self.short_run = fast_back - fast_front < 0.0 && slow_back - slow_front < 0.0 || self.short_run;
        }
        if !self.initialized {
            self.has_inputs = true;
            if self.slow_ma_price.len() > self.signal_period && self.slow_ma.is_ready() {
                self.initialized = true;
            }
        }
        if self.long_run {
            1
        } else if self.short_run {
            -1
        } else {
            0
        }
    }

    /// Получить сигнал AMAT как типизированный IndicatorValue
    pub fn value(&self) -> IndicatorValue {
        let signal = if self.long_run {
            1
        } else if self.short_run {
            -1
        } else {
            0
        };
        IndicatorValue::Signal(signal)
    }

    /// Получить сигнал AMAT как i8 (для обратной совместимости)
    pub fn value_signal(&self) -> i8 {
        if self.long_run {
            1
        } else if self.short_run {
            -1
        } else {
            0
        }
    }
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
    pub fn reset(&mut self) {
        // Use stored types directly - no pattern matching on MovingAverage enum
        self.fast_ma = MovingAverageProvider::new(self.fast_type, self.fast_period);
        self.slow_ma = MovingAverageProvider::new(self.slow_type, self.slow_period);
        self.fast_ma_price.clear();
        self.slow_ma_price.clear();
        self.long_run = false;
        self.short_run = false;
        self.has_inputs = false;
        self.initialized = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_amat_creation() {
        let amat = Amat::new(8, 21, 9, MovingAverageType::EMA, MovingAverageType::EMA);
        assert!(!amat.is_ready());
        assert_eq!(amat.value_signal(), 0);
    }

    #[test]
    fn test_amat_uptrend() {
        let mut amat = Amat::new(5, 10, 5, MovingAverageType::EMA, MovingAverageType::EMA);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            amat.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(amat.is_ready());
        // In uptrend, should signal long
        assert_eq!(amat.value_signal(), 1, "AMAT should signal long in uptrend");
    }

    #[test]
    fn test_amat_downtrend() {
        let mut amat = Amat::new(5, 10, 5, MovingAverageType::EMA, MovingAverageType::EMA);
        for i in 1..=50 {
            let price = 200.0 - i as f64 * 2.0;
            amat.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(amat.is_ready());
        // In downtrend, should signal short
        assert_eq!(amat.value_signal(), -1, "AMAT should signal short in downtrend");
    }

    #[test]
    fn test_amat_reset() {
        let mut amat = Amat::new(5, 10, 5, MovingAverageType::EMA, MovingAverageType::EMA);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            amat.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(amat.is_ready());
        amat.reset();
        assert!(!amat.is_ready());
        assert_eq!(amat.value_signal(), 0);
    }

    #[test]
    fn test_amat_value_types() {
        let amat = Amat::new(5, 10, 5, MovingAverageType::EMA, MovingAverageType::EMA);
        match amat.value() {
            IndicatorValue::Signal(s) => assert_eq!(s, 0),
            _ => panic!("AMAT should return Signal value"),
        }
    }

    #[test]
    fn test_amat_signal_range() {
        let mut amat = Amat::new(5, 10, 5, MovingAverageType::EMA, MovingAverageType::EMA);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let signal = amat.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(signal >= -1 && signal <= 1, "AMAT signal should be -1, 0, or 1, got {}", signal);
        }
    }
}






















