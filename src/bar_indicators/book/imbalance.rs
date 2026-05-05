// High-performance Book Imbalance Ratio
// (c) 2024

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct BookImbalanceRatio {
    value: f64,
    count: usize,
    ready: bool,
}

impl Default for BookImbalanceRatio {
    fn default() -> Self {
        Self::new()
    }
}

impl BookImbalanceRatio {
    pub fn new() -> Self {
        Self {
            value: 0.0,
            count: 0,
            ready: false,
        }
    }
    /// Обновить индикатор новыми best_bid и best_ask (размеры стакана)
    pub fn update_book(&mut self, best_bid: f64, best_ask: f64) -> f64 {
        self.count += 1;
        if best_bid > 0.0 && best_ask > 0.0 {
            let smaller = best_bid.min(best_ask);
            let larger = best_bid.max(best_ask);
            self.value = smaller / larger;
            self.ready = true;
        } else {
            self.value = 0.0;
            self.ready = false;
        }
        self.value
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.count = 0;
        self.ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_book_imbalance_ratio_creation() {
        let ind = BookImbalanceRatio::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_book_imbalance_ratio_warmup() {
        let mut ind = BookImbalanceRatio::new();
        ind.update_book(100.0, 110.0);
        assert!(ind.is_ready());
    }

    #[test]
    fn test_book_imbalance_ratio_values() {
        let mut ind = BookImbalanceRatio::new();
        // Equal sizes should give ratio 1.0
        ind.update_book(100.0, 100.0);
        assert!((ind.value().main() - 1.0).abs() < 1e-10);

        // Different sizes - smaller/larger
        ind.update_book(50.0, 100.0);
        assert!((ind.value().main() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_book_imbalance_ratio_reset() {
        let mut ind = BookImbalanceRatio::new();
        ind.update_book(100.0, 110.0);
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
}






















