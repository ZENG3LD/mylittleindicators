//! CharmTracker — approximates Charm (∂Δ/∂t) from consecutive OptionGreeks snapshots.
//!
//! Charm = dDelta/dt ≈ (current_delta − prev_delta) / dt_seconds
//!
//! Uses the `timestamp` field of `OptionGreeks` for dt computation.
//! Returns 0.0 when dt == 0 (same timestamp) or when there is no prior snapshot.
//!
//! Output: `Single(charm)`.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::OptionGreeksConsumer;
use crate::core::types::OptionGreeks;

/// Approximates the Charm Greek (delta decay, ∂Δ/∂t).
///
/// `charm ≈ (current_delta − prev_delta) / dt_seconds`
///
/// Returns 0 until two snapshots have been seen, or if dt is too small.
#[derive(Clone, Debug)]
pub struct CharmTracker {
    prev_delta: f64,
    prev_ts: i64,
    last_charm: f64,
}

impl CharmTracker {
    /// Create a new CharmTracker.
    pub fn new() -> Self {
        Self {
            prev_delta: 0.0,
            prev_ts: 0,
            last_charm: 0.0,
        }
    }
}

impl Default for CharmTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl OptionGreeksConsumer for CharmTracker {
    fn update_option_greeks(&mut self, g: &OptionGreeks) -> IndicatorValue {
        if self.prev_ts > 0 {
            let dt = (g.timestamp - self.prev_ts) as f64 / 1000.0; // ms → seconds
            if dt > 1e-9 {
                self.last_charm = (g.delta - self.prev_delta) / dt;
            }
            // If dt == 0 (duplicate timestamp), charm is unchanged
        }
        self.prev_delta = g.delta;
        self.prev_ts = g.timestamp;
        IndicatorValue::Single(self.last_charm)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_charm)
    }

    fn reset(&mut self) {
        self.prev_delta = 0.0;
        self.prev_ts = 0;
        self.last_charm = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.prev_ts > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_greeks(delta: f64, ts: i64) -> OptionGreeks {
        OptionGreeks {
            delta,
            gamma: 0.0,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
            mark_iv: 0.0,
            bid_iv: None,
            ask_iv: None,
            timestamp: ts,
        }
    }

    #[test]
    fn first_update_returns_zero() {
        let mut ind = CharmTracker::new();
        let val = ind.update_option_greeks(&make_greeks(0.5, 1000));
        assert_eq!(val.main(), 0.0);
    }

    #[test]
    fn increasing_delta_gives_positive_charm() {
        let mut ind = CharmTracker::new();
        ind.update_option_greeks(&make_greeks(0.4, 1000));
        // 1 second later, delta increased by 0.1 → charm = 0.1 / 1.0 = 0.1
        let val = ind.update_option_greeks(&make_greeks(0.5, 2000));
        let expected = (0.5 - 0.4) / 1.0;
        assert!((val.main() - expected).abs() < 1e-9, "expected {expected}, got {}", val.main());
    }

    #[test]
    fn decreasing_delta_gives_negative_charm() {
        let mut ind = CharmTracker::new();
        ind.update_option_greeks(&make_greeks(0.5, 1000));
        let val = ind.update_option_greeks(&make_greeks(0.4, 2000));
        assert!(val.main() < 0.0, "decreasing delta should give negative charm, got {}", val.main());
    }

    #[test]
    fn zero_dt_preserves_last_charm() {
        let mut ind = CharmTracker::new();
        ind.update_option_greeks(&make_greeks(0.4, 1000));
        ind.update_option_greeks(&make_greeks(0.5, 2000)); // charm = 0.1
        // Same timestamp — charm should not change
        let val = ind.update_option_greeks(&make_greeks(0.6, 2000));
        assert!((val.main() - 0.1).abs() < 1e-9, "charm should be preserved on zero dt, got {}", val.main());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = CharmTracker::new();
        ind.update_option_greeks(&make_greeks(0.5, 1000));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
}
