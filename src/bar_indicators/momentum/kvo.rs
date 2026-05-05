// High-performance Klinger Volume Oscillator (KVO)
// (c) 2024

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct Kvo {
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    ma_type: MovingAverageType,
    fast_ema: MovingAverageProvider,
    slow_ema: MovingAverageProvider,
    signal_ema: MovingAverageProvider,
    prev_hlc3: f64,
    kvo_value: f64,
    signal_value: f64,
    ready: bool,
    count: usize,
}

impl Kvo {
    /// Create KVO with default MA type (EMA)
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self::new_with_ma_type(fast_period, slow_period, signal_period, MovingAverageType::EMA)
    }

    /// Create KVO with specified MA type
    pub fn new_with_ma_type(fast_period: usize, slow_period: usize, signal_period: usize, ma_type: MovingAverageType) -> Self {
        Self {
            fast_period,
            slow_period,
            signal_period,
            ma_type,
            fast_ema: MovingAverageProvider::new(ma_type, fast_period),
            slow_ema: MovingAverageProvider::new(ma_type, slow_period),
            signal_ema: MovingAverageProvider::new(ma_type, signal_period),
            prev_hlc3: 0.0,
            kvo_value: 0.0,
            signal_value: 0.0,
            ready: false,
            count: 0,
        }
    }

    /// Set the MA type and reset the indicator
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }
    /// Обновить KVO новым баром (high, low, close, volume)
    pub fn update_bar(&mut self, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let hlc3 = (high + low + close) / 3.0;
        let vol = if self.count == 0 {
            self.prev_hlc3 = hlc3;
            self.count += 1;
            0.0
        } else if hlc3 > self.prev_hlc3 {
            self.prev_hlc3 = hlc3;
            self.count += 1;
            volume
        } else if hlc3 < self.prev_hlc3 {
            self.prev_hlc3 = hlc3;
            self.count += 1;
            -volume
        } else {
            self.prev_hlc3 = hlc3;
            self.count += 1;
            0.0
        };
        let fast = self.fast_ema.update_bar(0.0, 0.0, 0.0, vol, 0.0);
        let slow = self.slow_ema.update_bar(0.0, 0.0, 0.0, vol, 0.0);
        let osc = fast - slow;
        self.kvo_value = osc;
        if self.slow_ema.is_ready() {
            self.signal_value = self.signal_ema.update_bar(0.0, 0.0, 0.0, osc, 0.0);
            self.ready = self.signal_ema.is_ready();
        }
        self.kvo_value
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.kvo_value, self.signal_value)
    }
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    pub fn reset(&mut self) {
        self.fast_ema = MovingAverageProvider::new(self.ma_type, self.fast_period);
        self.slow_ema = MovingAverageProvider::new(self.ma_type, self.slow_period);
        self.signal_ema = MovingAverageProvider::new(self.ma_type, self.signal_period);
        self.prev_hlc3 = 0.0;
        self.kvo_value = 0.0;
        self.signal_value = 0.0;
        self.ready = false;
        self.count = 0;
    }

    pub fn fast_period(&self) -> usize {
        self.fast_period
    }

    pub fn slow_period(&self) -> usize {
        self.slow_period
    }

    pub fn signal_period(&self) -> usize {
        self.signal_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kvo_creation() {
        let kvo = Kvo::new(34, 55, 13);
        assert!(!kvo.is_ready());
        assert_eq!(kvo.value().main(), 0.0);
    }

    #[test]
    fn test_kvo_with_ma_type() {
        let kvo = Kvo::new_with_ma_type(34, 55, 13, MovingAverageType::SMA);
        assert!(!kvo.is_ready());
    }

    #[test]
    fn test_kvo_basic_calculation() {
        let mut kvo = Kvo::new(34, 55, 13);

        for i in 1..=100 {
            let price = 100.0 + i as f64;
            let value = kvo.update_bar(price + 2.0, price - 1.0, price + 1.0, 10000.0 * (1.0 + (i as f64 * 0.1).sin()));

            if kvo.is_ready() {
                assert!(value.is_finite());
            }
        }

        assert!(kvo.is_ready());
    }

    #[test]
    fn test_kvo_uptrend_with_volume() {
        let mut kvo = Kvo::new(34, 55, 13);

        // Uptrend with increasing volume
        for i in 1..=100 {
            let price = 100.0 + i as f64;
            kvo.update_bar(price + 1.0, price - 0.5, price + 0.5, 10000.0 + i as f64 * 100.0);
        }

        // KVO tends to be positive in uptrend with increasing volume
        if kvo.is_ready() {
            let value = kvo.value().main();
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_kvo_downtrend_with_volume() {
        let mut kvo = Kvo::new(34, 55, 13);

        // Downtrend with increasing volume
        for i in 1..=100 {
            let price = 200.0 - i as f64;
            kvo.update_bar(price + 0.5, price - 1.0, price - 0.5, 10000.0 + i as f64 * 100.0);
        }

        // KVO tends to be negative in downtrend
        if kvo.is_ready() {
            let value = kvo.value().main();
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_kvo_reset() {
        let mut kvo = Kvo::new(34, 55, 13);

        for i in 1..=100 {
            let price = 100.0 + i as f64;
            kvo.update_bar(price + 1.0, price - 1.0, price, 10000.0);
        }

        assert!(kvo.is_ready());

        kvo.reset();
        assert!(!kvo.is_ready());
        assert_eq!(kvo.value().main(), 0.0);
    }

    #[test]
    fn test_kvo_periods() {
        let kvo = Kvo::new(34, 55, 13);
        assert_eq!(kvo.fast_period(), 34);
        assert_eq!(kvo.slow_period(), 55);
        assert_eq!(kvo.signal_period(), 13);
    }

    #[test]
    fn test_kvo_set_ma_type() {
        let mut kvo = Kvo::new(34, 55, 13);

        for i in 1..=100 {
            let price = 100.0 + i as f64;
            kvo.update_bar(price + 1.0, price - 1.0, price, 10000.0);
        }

        kvo.set_ma_type(MovingAverageType::SMA);
        assert!(!kvo.is_ready()); // Should reset
    }

    #[test]
    fn test_kvo_no_volume() {
        let mut kvo = Kvo::new(34, 55, 13);

        for i in 1..=100 {
            let price = 100.0 + i as f64;
            kvo.update_bar(price + 1.0, price - 1.0, price, 0.0);
        }

        // Should still work with zero volume
        let value = kvo.value().main();
        assert!(value.is_finite());
    }
} 






















