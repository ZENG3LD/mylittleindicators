use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Chaikin Oscillator: MA_fast(ADL) - MA_slow(ADL), where ADL = money flow volume cumulative
#[derive(Debug, Clone)]
pub struct ChaikinOscillator {
    fast_period: usize,
    slow_period: usize,
    ma_type: MovingAverageType,
    adl_fast: MovingAverageProvider,
    adl_slow: MovingAverageProvider,
    adl_value: f64,
    value: f64,
}

impl ChaikinOscillator {
    /// Create Chaikin Oscillator with default MA type (EMA)
    pub fn new(fast: usize, slow: usize) -> Self {
        Self::new_with_ma_type(fast, slow, MovingAverageType::EMA)
    }

    /// Create Chaikin Oscillator with specified MA type
    pub fn new_with_ma_type(fast: usize, slow: usize, ma_type: MovingAverageType) -> Self {
        let f = fast.max(1);
        let s = slow.max(1);
        Self {
            fast_period: f,
            slow_period: s,
            ma_type,
            adl_fast: MovingAverageProvider::new(ma_type, f),
            adl_slow: MovingAverageProvider::new(ma_type, s),
            adl_value: 0.0,
            value: 0.0,
        }
    }

    /// Set the MA type and reset the indicator
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }

    fn money_flow_multiplier(high: f64, low: f64, close: f64) -> f64 {
        let hl = (high - low).abs().max(1e-12);
        ((close - low) - (high - close)) / hl
    }

    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let mfm = Self::money_flow_multiplier(h, l, c);
        self.adl_value += mfm * v;
        let fast = self.adl_fast.update_bar(0.0, 0.0, 0.0, self.adl_value, 0.0);
        let slow = self.adl_slow.update_bar(0.0, 0.0, 0.0, self.adl_value, 0.0);
        self.value = fast - slow;
        self.value
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.adl_fast.is_ready() && self.adl_slow.is_ready()
    }

    pub fn reset(&mut self) {
        self.adl_fast = MovingAverageProvider::new(self.ma_type, self.fast_period);
        self.adl_slow = MovingAverageProvider::new(self.ma_type, self.slow_period);
        self.adl_value = 0.0;
        self.value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chaikin_oscillator_creation() {
        let co = ChaikinOscillator::new(3, 10);
        assert!(!co.is_ready());
        assert_eq!(co.value().main(), 0.0);
    }

    #[test]
    fn test_chaikin_oscillator_warmup() {
        let mut co = ChaikinOscillator::new(3, 10);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            co.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(co.is_ready());
    }

    #[test]
    fn test_chaikin_oscillator_values_finite() {
        let mut co = ChaikinOscillator::new(3, 10);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = co.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_chaikin_oscillator_reset() {
        let mut co = ChaikinOscillator::new(3, 10);
        for i in 0..20 {
            co.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        co.reset();
        assert!(!co.is_ready());
        assert_eq!(co.value().main(), 0.0);
    }
}
