//! MarkPriceVolatility — rolling standard deviation of mark price.
//!
//! Measures short-term volatility of the mark price feed.
//!
//! Output: `Single(std)`.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::mark_price_consumer::MarkPriceConsumer;
use crate::core::types::MarkPrice;

/// Rolling standard deviation of mark price.
///
/// Returns population std of the last `window` mark price observations.
#[derive(Clone)]
pub struct MarkPriceVolatility {
    window: usize,
    history: VecDeque<f64>,
    last_std: f64,
}

impl MarkPriceVolatility {
    /// Create with given lookback window (min 2).
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            history: VecDeque::new(),
            last_std: 0.0,
        }
    }

    fn compute_std(&self) -> f64 {
        let n = self.history.len();
        if n < 2 {
            return 0.0;
        }
        let mean = self.history.iter().sum::<f64>() / n as f64;
        let variance = self.history.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        variance.sqrt()
    }
}

impl Default for MarkPriceVolatility {
    fn default() -> Self {
        Self::new(20)
    }
}

impl MarkPriceConsumer for MarkPriceVolatility {
    fn update_mark(&mut self, mp: &MarkPrice) -> IndicatorValue {
        self.history.push_back(mp.mark_price);
        if self.history.len() > self.window {
            self.history.pop_front();
        }
        self.last_std = self.compute_std();
        IndicatorValue::Single(self.last_std)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_std)
    }

    fn reset(&mut self) {
        self.history.clear();
        self.last_std = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mp(mark_price: f64) -> MarkPrice {
        MarkPrice {
            mark_price,
            index_price: None,
            funding_rate: None,
            timestamp: 1000,
        }
    }

    #[test]
    fn constant_price_gives_zero_std() {
        let mut ind = MarkPriceVolatility::new(5);
        for _ in 0..5 {
            ind.update_mark(&make_mp(50000.0));
        }
        if let IndicatorValue::Single(v) = ind.value() {
            assert!(v.abs() < 1e-10, "std of constant series should be 0, got {v}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn non_constant_price_gives_positive_std() {
        let mut ind = MarkPriceVolatility::new(4);
        for p in [50000.0, 50100.0, 50200.0, 50050.0] {
            ind.update_mark(&make_mp(p));
        }
        if let IndicatorValue::Single(v) = ind.value() {
            assert!(v > 0.0, "std should be positive for varying prices, got {v}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn not_ready_until_window_full() {
        let mut ind = MarkPriceVolatility::new(5);
        for i in 0..4 {
            ind.update_mark(&make_mp(50000.0 + i as f64 * 10.0));
        }
        assert!(!ind.is_ready());
        ind.update_mark(&make_mp(50040.0));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = MarkPriceVolatility::new(5);
        for p in [50000.0, 50100.0, 50200.0, 50300.0, 50400.0] {
            ind.update_mark(&make_mp(p));
        }
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
