//! IndexTrackingError — rolling standard deviation of (index_price - composite_price).
//!
//! Dual consumer: `IndexPriceConsumer` + `CompositeIndexConsumer`.
//!
//! Logic:
//! - On each update, if both last values are > 0, push `(last_index - last_composite)` to window.
//! - `tracking_error` = rolling std of differences in window.
//!
//! Output: `Single(tracking_error)`.

use std::collections::VecDeque;

use crate::bar_indicators::composite_index_consumer::CompositeIndexConsumer;
use crate::bar_indicators::index_price_consumer::IndexPriceConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::types::{CompositeIndex, IndexPrice};

/// Rolling tracking error between index price and composite price.
///
/// Implements both `IndexPriceConsumer` and `CompositeIndexConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Clone)]
pub struct IndexTrackingError {
    window: usize,
    last_index: f64,
    last_composite: f64,
    diffs: VecDeque<f64>,
    last_error: f64,
}

impl IndexTrackingError {
    /// Create a new indicator.
    ///
    /// - `window` — rolling window for std computation (minimum 2, default 20).
    pub fn new(window: usize) -> Self {
        let w = window.max(2);
        Self {
            window: w,
            last_index: 0.0,
            last_composite: 0.0,
            diffs: VecDeque::with_capacity(w),
            last_error: 0.0,
        }
    }

    fn push_diff(&mut self) {
        if self.last_index <= 0.0 || self.last_composite <= 0.0 {
            return;
        }
        let d = self.last_index - self.last_composite;
        if self.diffs.len() >= self.window {
            self.diffs.pop_front();
        }
        self.diffs.push_back(d);
        self.recompute_error();
    }

    fn recompute_error(&mut self) {
        if self.diffs.len() < 2 {
            self.last_error = 0.0;
            return;
        }
        let n = self.diffs.len() as f64;
        let mean = self.diffs.iter().sum::<f64>() / n;
        let variance = self.diffs.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / n;
        self.last_error = variance.sqrt();
    }

    /// Passthrough for bar pipeline — returns current value.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_error)
    }

    /// True when at least 2 paired observations have been collected.
    pub fn indicator_is_ready(&self) -> bool {
        self.diffs.len() >= 2
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.last_index = 0.0;
        self.last_composite = 0.0;
        self.diffs.clear();
        self.last_error = 0.0;
    }
}

impl Default for IndexTrackingError {
    fn default() -> Self {
        Self::new(20)
    }
}

impl IndexPriceConsumer for IndexTrackingError {
    fn update_index_price(&mut self, ip: &IndexPrice) -> IndicatorValue {
        self.last_index = ip.price;
        self.push_diff();
        self.indicator_value()
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
    }

    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl CompositeIndexConsumer for IndexTrackingError {
    fn update_composite_index(&mut self, ci: &CompositeIndex) -> IndicatorValue {
        self.last_composite = ci.price;
        self.push_diff();
        self.indicator_value()
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
    }

    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ip(price: f64) -> IndexPrice {
        IndexPrice { price, timestamp: 1000 }
    }

    fn make_ci(price: f64) -> CompositeIndex {
        CompositeIndex { price, components: vec![], timestamp: 1000 }
    }

    #[test]
    fn zero_error_when_perfectly_tracking() {
        let mut ind = IndexTrackingError::new(5);
        for _ in 0..5 {
            ind.update_index_price(&make_ip(100.0));
            ind.update_composite_index(&make_ci(100.0));
        }
        if let IndicatorValue::Single(err) = ind.indicator_value() {
            assert!(err < 1e-9, "err={err}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn nonzero_error_when_diverging() {
        let mut ind = IndexTrackingError::new(5);
        // Alternate offsets: diff = +1, -1, +1, -1, +1
        for i in 0..5 {
            let offset = if i % 2 == 0 { 1.0_f64 } else { -1.0_f64 };
            ind.update_index_price(&make_ip(100.0 + offset));
            ind.update_composite_index(&make_ci(100.0));
        }
        if let IndicatorValue::Single(err) = ind.indicator_value() {
            assert!(err > 0.0, "err={err}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn not_ready_before_two_observations() {
        let mut ind = IndexTrackingError::new(5);
        ind.update_index_price(&make_ip(100.0));
        ind.update_composite_index(&make_ci(100.0));
        assert!(!ind.indicator_is_ready(), "single observation should not be ready");
        ind.update_index_price(&make_ip(100.0));
        ind.update_composite_index(&make_ci(100.0));
        assert!(ind.indicator_is_ready());
    }

    #[test]
    fn constant_positive_spread_gives_zero_std() {
        let mut ind = IndexTrackingError::new(5);
        // Constant spread of 5.0 every time
        for _ in 0..6 {
            ind.update_index_price(&make_ip(105.0));
            ind.update_composite_index(&make_ci(100.0));
        }
        if let IndicatorValue::Single(err) = ind.indicator_value() {
            assert!(err < 1e-9, "constant spread should yield std≈0, got {err}");
        } else {
            panic!("expected Single");
        }
    }
}
