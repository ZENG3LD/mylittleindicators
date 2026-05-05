// Queue Imbalance (Level-1 proxy from OHLCV) - approximation using close vs mid

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct QueueImbalance {
    value: f64,
}

impl Default for QueueImbalance {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueImbalance {
    pub fn new() -> Self {
        Self { value: 0.0 }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, _v: f64) -> f64 {
        let mid = (h + l) / 2.0;
        let rng = (h - l).max(1e-9);
        self.value = (c - mid) / rng;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_imbalance_creation() {
        let ind = QueueImbalance::new();
        assert!(ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_queue_imbalance_update() {
        let mut ind = QueueImbalance::new();
        let value = ind.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0);
        assert!(value.is_finite());
        assert!(value >= -0.5 && value <= 0.5);
    }

    #[test]
    fn test_queue_imbalance_range() {
        let mut ind = QueueImbalance::new();
        // Close at high should be positive
        let high_close = ind.update_bar(100.0, 105.0, 95.0, 105.0, 1000.0);
        assert!(high_close > 0.0);
        // Close at low should be negative
        let low_close = ind.update_bar(100.0, 105.0, 95.0, 95.0, 1000.0);
        assert!(low_close < 0.0);
    }

    #[test]
    fn test_queue_imbalance_reset() {
        let mut ind = QueueImbalance::new();
        ind.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0);
        ind.reset();
        assert_eq!(ind.value().main(), 0.0);
    }
}
