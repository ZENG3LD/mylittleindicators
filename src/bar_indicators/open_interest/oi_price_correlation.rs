//! OiPriceCorrelation — rolling Pearson correlation between OI and mark price.
//!
//! Events arrive asynchronously: OI and MarkPrice are separate streams.
//! Resolution: on each update, record `(last_oi, last_price)` if both are finite.
//!
//! Pearson: corr = cov(x,y) / (std_x * std_y)
//! where x = OI values, y = price values over the rolling window of paired observations.
//!
//! Returns 0.0 if fewer than 2 pairs or if either std is 0.
//!
//! Output: `Single(corr)` ∈ [-1, 1].

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::mark_price_consumer::MarkPriceConsumer;
use crate::bar_indicators::open_interest_consumer::OpenInterestConsumer;
use crate::core::types::MarkPrice;
use crate::core::types::OpenInterest;

/// Rolling Pearson correlation between OI and mark price.
#[derive(Clone)]
pub struct OiPriceCorrelation {
    window: usize,
    last_oi: f64,
    last_price: f64,
    oi_initialized: bool,
    price_initialized: bool,
    pairs: VecDeque<(f64, f64)>,
    last_corr: f64,
}

impl OiPriceCorrelation {
    /// Create with given window size (minimum 2).
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            last_oi: 0.0,
            last_price: 0.0,
            oi_initialized: false,
            price_initialized: false,
            pairs: VecDeque::with_capacity(window.max(2)),
            last_corr: 0.0,
        }
    }

    fn try_push_pair(&mut self) {
        if !self.oi_initialized || !self.price_initialized {
            return;
        }
        if !self.last_oi.is_finite() || !self.last_price.is_finite() {
            return;
        }
        if self.pairs.len() == self.window {
            self.pairs.pop_front();
        }
        self.pairs.push_back((self.last_oi, self.last_price));
        self.last_corr = Self::pearson(&self.pairs);
    }

    fn pearson(pairs: &VecDeque<(f64, f64)>) -> f64 {
        let n = pairs.len();
        if n < 2 {
            return 0.0;
        }
        let nf = n as f64;
        let mean_x = pairs.iter().map(|(x, _)| x).sum::<f64>() / nf;
        let mean_y = pairs.iter().map(|(_, y)| y).sum::<f64>() / nf;

        let mut cov = 0.0f64;
        let mut var_x = 0.0f64;
        let mut var_y = 0.0f64;
        for (x, y) in pairs {
            let dx = x - mean_x;
            let dy = y - mean_y;
            cov += dx * dy;
            var_x += dx * dx;
            var_y += dy * dy;
        }

        let denom = (var_x * var_y).sqrt();
        if denom == 0.0 {
            0.0
        } else {
            (cov / denom).clamp(-1.0, 1.0)
        }
    }

    /// Current correlation value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_corr)
    }

    /// True once at least 2 (oi, price) pairs accumulated.
    pub fn indicator_is_ready(&self) -> bool {
        self.pairs.len() >= 2
    }

    /// Reset all state.
    pub fn indicator_reset(&mut self) {
        self.last_oi = 0.0;
        self.last_price = 0.0;
        self.oi_initialized = false;
        self.price_initialized = false;
        self.pairs.clear();
        self.last_corr = 0.0;
    }
}

impl Default for OiPriceCorrelation {
    fn default() -> Self {
        Self::new(50)
    }
}

impl OpenInterestConsumer for OiPriceCorrelation {
    fn update_oi(&mut self, oi: &OpenInterest) -> IndicatorValue {
        self.last_oi = oi.open_interest;
        self.oi_initialized = true;
        self.try_push_pair();
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

impl MarkPriceConsumer for OiPriceCorrelation {
    fn update_mark(&mut self, mp: &MarkPrice) -> IndicatorValue {
        self.last_price = mp.mark_price;
        self.price_initialized = true;
        self.try_push_pair();
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

    fn make_oi(oi: f64) -> OpenInterest {
        OpenInterest {
            open_interest: oi,
            open_interest_value: None,
            timestamp: 0,
        }
    }

    fn make_mark(price: f64) -> MarkPrice {
        MarkPrice {
            mark_price: price,
            index_price: None,
            funding_rate: None,
            timestamp: 0,
        }
    }

    #[test]
    fn not_ready_initially() {
        let ind = OiPriceCorrelation::new(10);
        assert!(!ind.indicator_is_ready());
    }

    #[test]
    fn perfect_positive_correlation() {
        // OI and price move together strictly in sync (OI update then price update alternating).
        // Because each stream update pushes a pair using the last known state of the other,
        // the resulting window contains both exact-match and one-step-lag pairs.
        // Correlation will be high (> 0.9) but not exactly 1.0 due to interleaving.
        let mut ind = OiPriceCorrelation::new(50);
        let pairs = [(100.0f64, 1000.0f64), (200.0, 2000.0), (300.0, 3000.0),
                     (400.0, 4000.0), (500.0, 5000.0)];
        for (oi, price) in pairs {
            ind.update_oi(&make_oi(oi));
            ind.update_mark(&make_mark(price));
        }
        if let IndicatorValue::Single(c) = ind.indicator_value() {
            assert!(c > 0.9, "expected corr > 0.9 for co-moving OI+price, got {c}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn perfect_negative_correlation() {
        // OI rises, price falls → high negative correlation
        let mut ind = OiPriceCorrelation::new(50);
        let oi_vals = [100.0f64, 200.0, 300.0, 400.0, 500.0];
        let price_vals = [500.0f64, 400.0, 300.0, 200.0, 100.0];
        for (&oi, &price) in oi_vals.iter().zip(price_vals.iter()) {
            ind.update_oi(&make_oi(oi));
            ind.update_mark(&make_mark(price));
        }
        if let IndicatorValue::Single(c) = ind.indicator_value() {
            assert!(c < -0.9, "expected corr < -0.9 for inverse OI+price, got {c}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn zero_correlation_on_constant_series() {
        // Constant OI → std=0 → corr=0
        let mut ind = OiPriceCorrelation::new(50);
        for i in 0..10 {
            ind.update_oi(&make_oi(100.0));
            ind.update_mark(&make_mark(1000.0 + i as f64));
        }
        if let IndicatorValue::Single(c) = ind.indicator_value() {
            assert_eq!(c, 0.0, "expected 0 when OI is constant");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn corr_bounded_minus_one_to_one() {
        let mut ind = OiPriceCorrelation::new(5);
        for i in 0..20 {
            let oi = (i as f64).sin() * 1000.0 + 5000.0;
            let price = (i as f64 * 1.3).cos() * 500.0 + 30000.0;
            ind.update_oi(&make_oi(oi));
            ind.update_mark(&make_mark(price));
        }
        if let IndicatorValue::Single(c) = ind.indicator_value() {
            assert!(c >= -1.0 && c <= 1.0, "corr out of bounds: {c}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = OiPriceCorrelation::new(5);
        ind.update_oi(&make_oi(100.0));
        ind.update_mark(&make_mark(1000.0));
        ind.update_oi(&make_oi(200.0));
        ind.update_mark(&make_mark(2000.0));
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        if let IndicatorValue::Single(c) = ind.indicator_value() {
            assert_eq!(c, 0.0);
        }
    }
}
