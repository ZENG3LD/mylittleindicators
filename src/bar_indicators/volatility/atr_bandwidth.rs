// ATR Bandwidth: (High-Low)/ATR over rolling period

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct AtrBandwidth {
    atr: Atr,
    value: f64,
}

impl AtrBandwidth {
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
    fn test_atr_bandwidth_creation() {
        let ab = AtrBandwidth::new(14, MovingAverageType::RMA);
        assert!(!ab.is_ready());
        assert_eq!(ab.value().main(), 0.0);
    }

    #[test]
    fn test_atr_bandwidth_warmup() {
        let mut ab = AtrBandwidth::new(14, MovingAverageType::RMA);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ab.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ab.is_ready());
    }

    #[test]
    fn test_atr_bandwidth_values() {
        let mut ab = AtrBandwidth::new(14, MovingAverageType::RMA);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            let value = ab.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_atr_bandwidth_reset() {
        let mut ab = AtrBandwidth::new(14, MovingAverageType::RMA);
        for i in 0..20 {
            ab.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        ab.reset();
        assert!(!ab.is_ready());
        assert_eq!(ab.value().main(), 0.0);
    }
}
