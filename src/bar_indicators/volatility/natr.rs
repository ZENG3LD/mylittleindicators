use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Normalized ATR (NATR) = 100 * ATR(period) / Close
#[derive(Debug, Clone)]
pub struct Natr {
    atr: Atr,
    value: f64,
}

impl Natr {
    pub fn new(period: usize) -> Self {
        Self {
            atr: Atr::new(period.max(1), MovingAverageType::RMA),
            value: 0.0,
        }
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let _ = self.atr.update_bar(open, high, low, close, volume);
        let atrv = self.atr.value().main();
        self.value = if close.abs() < 1e-12 {
            0.0
        } else {
            100.0 * atrv / close.abs()
        };
        self.value
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.atr.is_ready()
    }
    pub fn reset(&mut self) {
        self.atr.reset();
        self.value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_natr_creation() {
        let natr = Natr::new(14);
        assert!(!natr.is_ready());
        assert_eq!(natr.value().main(), 0.0);
    }

    #[test]
    fn test_natr_warmup() {
        let mut natr = Natr::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            natr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(natr.is_ready());
    }

    #[test]
    fn test_natr_positive() {
        let mut natr = Natr::new(14);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            let value = natr.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_natr_reset() {
        let mut natr = Natr::new(14);
        for i in 0..20 {
            natr.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        natr.reset();
        assert!(!natr.is_ready());
        assert_eq!(natr.value().main(), 0.0);
    }
}
