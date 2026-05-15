//! CompositeWeightDrift — rolling max relative drift across composite index components.
//!
//! Consumer: `CompositeIndexConsumer`.
//!
//! Logic: For each component, compute `|new_weight - prev_weight| / |prev_weight|`.
//! Returns the maximum drift across all components between consecutive snapshots.
//! Unlike `IndexComponentDrift` (which only looks at prev→current), this maintains
//! a rolling history of snapshots and returns max drift across the full window.
//!
//! Output: `Single(max_drift_pct)`.

use std::collections::HashMap;
use std::collections::VecDeque;

use crate::bar_indicators::CompositeIndexConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::types::CompositeIndex;

/// Rolling composite index weight drift indicator.
///
/// Implements `CompositeIndexConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Clone)]
pub struct CompositeWeightDrift {
    history: VecDeque<HashMap<String, f64>>,
    window_snapshots: usize,
    last_max_drift: f64,
}

impl CompositeWeightDrift {
    /// Create a new indicator.
    ///
    /// - `window_snapshots` — number of snapshots to keep in rolling history (default 10).
    pub fn new(window_snapshots: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(window_snapshots.max(2)),
            window_snapshots: window_snapshots.max(2),
            last_max_drift: 0.0,
        }
    }

    fn recompute_drift(&mut self) {
        if self.history.len() < 2 {
            self.last_max_drift = 0.0;
            return;
        }
        let prev = self.history.get(self.history.len() - 2).unwrap();
        let curr = self.history.back().unwrap();

        let mut max_drift = 0.0_f64;
        for (sym, new_w) in curr {
            if let Some(&old_w) = prev.get(sym.as_str()) {
                if old_w.abs() > 1e-12 {
                    let drift = (new_w - old_w).abs() / old_w.abs();
                    if drift > max_drift {
                        max_drift = drift;
                    }
                }
            }
        }
        self.last_max_drift = max_drift;
    }

    /// Passthrough for bar pipeline — returns current value.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_max_drift)
    }

    /// True when at least two snapshots have been received.
    pub fn indicator_is_ready(&self) -> bool {
        self.history.len() >= 2
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.history.clear();
        self.last_max_drift = 0.0;
    }
}

impl Default for CompositeWeightDrift {
    fn default() -> Self {
        Self::new(10)
    }
}

impl CompositeIndexConsumer for CompositeWeightDrift {
    fn update_composite_index(&mut self, ci: &CompositeIndex) -> IndicatorValue {
        if self.history.len() >= self.window_snapshots {
            self.history.pop_front();
        }
        let weights: HashMap<String, f64> = ci.components.iter().map(|(s, w)| (s.clone(), *w)).collect();
        self.history.push_back(weights);
        self.recompute_drift();
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

    fn make_ci(components: Vec<(&str, f64)>) -> CompositeIndex {
        CompositeIndex {
            price: 1.0,
            components: components.into_iter().map(|(s, w)| (s.to_string(), w)).collect(),
            timestamp: 0,
        }
    }

    #[test]
    fn drift_detected_on_weight_change() {
        let mut ind = CompositeWeightDrift::new(5);
        ind.update_composite_index(&make_ci(vec![("BTC", 0.5), ("ETH", 0.5)]));
        let v = ind.update_composite_index(&make_ci(vec![("BTC", 0.6), ("ETH", 0.4)]));
        // BTC: |0.6-0.5|/0.5 = 0.2, ETH: |0.4-0.5|/0.5 = 0.2
        if let IndicatorValue::Single(d) = v {
            assert!((d - 0.2).abs() < 1e-9, "drift={d}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn zero_drift_on_identical_snapshots() {
        let mut ind = CompositeWeightDrift::new(5);
        ind.update_composite_index(&make_ci(vec![("BTC", 0.5)]));
        let v = ind.update_composite_index(&make_ci(vec![("BTC", 0.5)]));
        if let IndicatorValue::Single(d) = v {
            assert!(d.abs() < 1e-12, "drift={d}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = CompositeWeightDrift::default();
        ind.update_composite_index(&make_ci(vec![("BTC", 0.5)]));
        ind.update_composite_index(&make_ci(vec![("BTC", 0.6)]));
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        assert_eq!(ind.indicator_value(), IndicatorValue::Single(0.0));
    }
}
