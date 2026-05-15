//! AggTradeSizeDistribution — rolling median, p95 and current trade size.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::AggTradeConsumer;
use crate::core::types::AggTrade;

/// Tracks the size distribution of the last `window_size` aggregated trades.
///
/// Computes median and 95th percentile on every update.
///
/// Output: `IndicatorValue::Triple(median, p95, current_size)`.
#[derive(Clone)]
pub struct AggTradeSizeDistribution {
    window_size: usize,
    sizes: VecDeque<f64>,
    last_median: f64,
    last_p95: f64,
    last_current: f64,
}

impl AggTradeSizeDistribution {
    /// Create a new indicator. `window_size` is clamped to at least 1.
    pub fn new(window_size: usize) -> Self {
        let window_size = window_size.max(1);
        Self {
            window_size,
            sizes: VecDeque::with_capacity(window_size),
            last_median: 0.0,
            last_p95: 0.0,
            last_current: 0.0,
        }
    }

    /// Passthrough for bar events — returns last computed stats unchanged.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        IndicatorValue::Triple(self.last_median, self.last_p95, self.last_current)
    }
}

impl AggTradeConsumer for AggTradeSizeDistribution {
    fn update_agg_trade(&mut self, t: &AggTrade) -> IndicatorValue {
        self.sizes.push_back(t.quantity);
        while self.sizes.len() > self.window_size {
            self.sizes.pop_front();
        }

        let mut sorted: Vec<f64> = self.sizes.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let n = sorted.len();
        self.last_median = if n > 0 { sorted[n / 2] } else { 0.0 };
        self.last_p95 = if n > 0 {
            let idx = ((n as f64 - 1.0) * 0.95) as usize;
            sorted[idx]
        } else {
            0.0
        };
        self.last_current = t.quantity;

        IndicatorValue::Triple(self.last_median, self.last_p95, self.last_current)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_median, self.last_p95, self.last_current)
    }

    fn reset(&mut self) {
        self.sizes.clear();
        self.last_median = 0.0;
        self.last_p95 = 0.0;
        self.last_current = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.sizes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_trade(quantity: f64) -> AggTrade {
        AggTrade {
            aggregate_id: 0,
            price: 100.0,
            quantity,
            first_trade_id: 0,
            last_trade_id: 0,
            is_buy: true,
            timestamp: 0,
        }
    }

    #[test]
    fn single_trade() {
        let mut ind = AggTradeSizeDistribution::new(10);
        let v = ind.update_agg_trade(&make_trade(5.0));
        if let IndicatorValue::Triple(median, p95, cur) = v {
            assert_eq!(cur, 5.0);
            assert_eq!(median, 5.0);
            assert_eq!(p95, 5.0);
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn median_correct_for_sorted_window() {
        let mut ind = AggTradeSizeDistribution::new(5);
        for q in [1.0, 2.0, 3.0, 4.0, 5.0] {
            ind.update_agg_trade(&make_trade(q));
        }
        if let IndicatorValue::Triple(median, _p95, _cur) = ind.value() {
            // window = [1,2,3,4,5], sorted[2] = 3.0
            assert_eq!(median, 3.0, "expected median 3.0, got {median}");
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn window_rolls_out_old_trades() {
        let mut ind = AggTradeSizeDistribution::new(3);
        for q in [1.0, 1.0, 1.0, 100.0, 100.0, 100.0] {
            ind.update_agg_trade(&make_trade(q));
        }
        // window should now be [100, 100, 100]
        if let IndicatorValue::Triple(median, _p95, _cur) = ind.value() {
            assert_eq!(median, 100.0, "old small trades should be evicted");
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn reset_works() {
        let mut ind = AggTradeSizeDistribution::new(5);
        ind.update_agg_trade(&make_trade(10.0));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Triple(m, p, c) = ind.value() {
            assert_eq!(m, 0.0);
            assert_eq!(p, 0.0);
            assert_eq!(c, 0.0);
        }
    }
}
