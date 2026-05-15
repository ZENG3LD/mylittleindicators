//! IndexComponentDrift — maximum relative weight change across composite index components.

use std::collections::HashMap;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::CompositeIndexConsumer;
use crate::core::types::CompositeIndex;

/// Measures maximum relative drift in component weights between consecutive composite index snapshots.
///
/// For each component computes `|new_weight - old_weight| / old_weight`.
/// Returns the maximum across all components.
///
/// Output: `Single(max_drift_pct)`. Returns 0.0 until two consecutive snapshots arrive.
#[derive(Clone)]
pub struct IndexComponentDrift {
    prev_weights: HashMap<String, f64>,
    last_drift: f64,
}

impl IndexComponentDrift {
    /// Create a new indicator.
    pub fn new() -> Self {
        Self {
            prev_weights: HashMap::new(),
            last_drift: 0.0,
        }
    }
}

impl Default for IndexComponentDrift {
    fn default() -> Self {
        Self::new()
    }
}

impl CompositeIndexConsumer for IndexComponentDrift {
    fn update_composite_index(&mut self, ci: &CompositeIndex) -> IndicatorValue {
        let mut max_drift = 0.0_f64;
        if !self.prev_weights.is_empty() {
            for (sym, new_w) in &ci.components {
                if let Some(&old_w) = self.prev_weights.get(sym.as_str()) {
                    if old_w.abs() > 1e-12 {
                        let drift = (new_w - old_w).abs() / old_w.abs();
                        if drift > max_drift {
                            max_drift = drift;
                        }
                    }
                }
            }
        }
        self.prev_weights.clear();
        for (sym, w) in &ci.components {
            self.prev_weights.insert(sym.clone(), *w);
        }
        self.last_drift = max_drift;
        IndicatorValue::Single(self.last_drift)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_drift)
    }

    fn reset(&mut self) {
        self.prev_weights.clear();
        self.last_drift = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.prev_weights.is_empty()
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
        let mut ind = IndexComponentDrift::new();
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
    fn zero_drift_on_identical_snapshot() {
        let mut ind = IndexComponentDrift::new();
        ind.update_composite_index(&make_ci(vec![("BTC", 0.5)]));
        let v = ind.update_composite_index(&make_ci(vec![("BTC", 0.5)]));
        if let IndicatorValue::Single(d) = v {
            assert!(d.abs() < 1e-12);
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = IndexComponentDrift::new();
        ind.update_composite_index(&make_ci(vec![("BTC", 0.5)]));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
