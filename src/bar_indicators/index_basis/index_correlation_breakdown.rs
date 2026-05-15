//! IndexCorrelationBreakdown — minimum pairwise Pearson correlation of component weights.

use std::collections::{HashMap, VecDeque};

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::CompositeIndexConsumer;
use crate::core::types::CompositeIndex;

/// Rolling minimum pairwise Pearson correlation of component weights.
///
/// Maintains a rolling window of component weight snapshots.
/// For each pair of components computes the Pearson correlation of their
/// weight time series, then returns the minimum.
///
/// Output: `Single(min_correlation)`. Returns `1.0` until at least two
/// snapshots and two components are available.
#[derive(Clone)]
pub struct IndexCorrelationBreakdown {
    window_size: usize,
    snapshots: VecDeque<HashMap<String, f64>>,
    last_min_corr: f64,
}

impl IndexCorrelationBreakdown {
    /// Create a new indicator. `window_size` is clamped to at least 2.
    pub fn new(window_size: usize) -> Self {
        let window_size = window_size.max(2);
        Self {
            window_size,
            snapshots: VecDeque::with_capacity(window_size),
            last_min_corr: 1.0,
        }
    }

    fn compute_min_corr(&self) -> f64 {
        if self.snapshots.len() < 2 {
            return 1.0;
        }
        // Collect all unique symbols across all snapshots
        let mut symbols: Vec<String> = self
            .snapshots
            .iter()
            .flat_map(|snap| snap.keys().cloned())
            .collect();
        symbols.sort();
        symbols.dedup();

        if symbols.len() < 2 {
            return 1.0;
        }

        let n = self.snapshots.len();

        // Build weight series per symbol
        let series: Vec<Vec<f64>> = symbols
            .iter()
            .map(|sym| {
                self.snapshots
                    .iter()
                    .map(|snap| snap.get(sym).copied().unwrap_or(0.0))
                    .collect()
            })
            .collect();

        let mut min_corr = 1.0_f64;

        for i in 0..series.len() {
            for j in (i + 1)..series.len() {
                let corr = pearson_correlation(&series[i], &series[j], n);
                if corr < min_corr {
                    min_corr = corr;
                }
            }
        }

        min_corr
    }
}

fn pearson_correlation(a: &[f64], b: &[f64], n: usize) -> f64 {
    if n < 2 {
        return 1.0;
    }
    let mean_a = a.iter().sum::<f64>() / n as f64;
    let mean_b = b.iter().sum::<f64>() / n as f64;

    let mut num = 0.0_f64;
    let mut denom_a = 0.0_f64;
    let mut denom_b = 0.0_f64;

    for k in 0..n {
        let da = a[k] - mean_a;
        let db = b[k] - mean_b;
        num += da * db;
        denom_a += da * da;
        denom_b += db * db;
    }

    let denom = (denom_a * denom_b).sqrt();
    if denom < 1e-12 {
        1.0
    } else {
        (num / denom).clamp(-1.0, 1.0)
    }
}

impl Default for IndexCorrelationBreakdown {
    fn default() -> Self {
        Self::new(20)
    }
}

impl CompositeIndexConsumer for IndexCorrelationBreakdown {
    fn update_composite_index(&mut self, ci: &CompositeIndex) -> IndicatorValue {
        let snap: HashMap<String, f64> = ci.components.iter().cloned().collect();
        self.snapshots.push_back(snap);
        while self.snapshots.len() > self.window_size {
            self.snapshots.pop_front();
        }
        self.last_min_corr = self.compute_min_corr();
        IndicatorValue::Single(self.last_min_corr)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_min_corr)
    }

    fn reset(&mut self) {
        self.snapshots.clear();
        self.last_min_corr = 1.0;
    }

    fn is_ready(&self) -> bool {
        self.snapshots.len() >= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ci(btc: f64, eth: f64) -> CompositeIndex {
        CompositeIndex {
            price: 1.0,
            components: vec![("BTC".to_string(), btc), ("ETH".to_string(), eth)],
            timestamp: 0,
        }
    }

    #[test]
    fn perfect_negative_correlation() {
        let mut ind = IndexCorrelationBreakdown::new(5);
        // BTC goes up, ETH goes down — perfect negative correlation
        for i in 0..5 {
            ind.update_composite_index(&make_ci(i as f64, (4 - i) as f64));
        }
        if let IndicatorValue::Single(c) = ind.value() {
            assert!(c < -0.9, "correlation should be near -1, got {c}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn returns_one_on_single_snapshot() {
        let mut ind = IndexCorrelationBreakdown::new(5);
        ind.update_composite_index(&make_ci(0.5, 0.5));
        if let IndicatorValue::Single(c) = ind.value() {
            assert!((c - 1.0).abs() < 1e-9);
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = IndexCorrelationBreakdown::new(5);
        for i in 0..3 {
            ind.update_composite_index(&make_ci(i as f64, i as f64));
        }
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(c) = ind.value() {
            assert!((c - 1.0).abs() < 1e-9);
        }
    }
}
