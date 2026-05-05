// Range-to-ATR ratio: (High-Low)/ATR

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct RangeToAtr {
    atr: Atr,
    value: f64,
}

impl RangeToAtr {
    pub fn new(atr_period: usize, ma_type: MovingAverageType) -> Self {
        Self {
            atr: Atr::new(atr_period, ma_type),
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.atr.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.atr.is_ready()
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let atr_v = self.atr.update_bar(open, high, low, close, volume);
        let hl = (high - low).max(0.0);
        self.value = if atr_v > 1e-12 { hl / atr_v } else { 0.0 };
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_to_atr_creation() {
        let ind = RangeToAtr::new(14, MovingAverageType::SMA);
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_range_to_atr_warmup() {
        let mut ind = RangeToAtr::new(10, MovingAverageType::EMA);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update_bar(price, price + 2.0, price - 2.0, price + 1.0, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_range_to_atr_values() {
        let mut ind = RangeToAtr::new(10, MovingAverageType::SMA);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 5.0;
            ind.update_bar(price, price + 2.0, price - 2.0, price + 1.0, 1000.0);
        }
        assert!(ind.value().main().is_finite());
        assert!(ind.value().main() >= 0.0);
    }

    #[test]
    fn test_range_to_atr_reset() {
        let mut ind = RangeToAtr::new(10, MovingAverageType::SMA);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            ind.update_bar(price, price + 2.0, price - 2.0, price + 1.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
}
