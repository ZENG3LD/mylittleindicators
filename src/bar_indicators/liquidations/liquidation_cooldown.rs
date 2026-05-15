//! Liquidation Cooldown — time elapsed since the last liquidation event.
//!
//! Measures "market cooling" between forced liquidations. A long cooldown
//! suggests low liquidation pressure; a short cooldown indicates sustained
//! cascade activity.
//!
//! # Algorithm
//!
//! - Records `last_ts` (timestamp of the most recent liquidation, ms).
//! - On each `update_liquidation`:
//!   - If `last_ts > 0`, computes `delta = (liq.timestamp - last_ts) / 1000.0`.
//!   - Updates `last_ts` to `liq.timestamp`.
//!   - Returns `Single(delta_seconds)`.
//! - Before the first pair of events the output is `Single(0.0)`.
//!
//! No external clock or `update_bar` is needed — cooldown is the inter-event
//! gap, not "time since last event in wall-clock time". This is consistent
//! with the rest of the liquidation indicator family and avoids any dependency
//! on bar timestamps.
//!
//! # Output
//! `Single(seconds_since_last_liquidation)`.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::liquidation_consumer::LiquidationConsumer;
use crate::core::types::Liquidation;

/// Inter-event cooldown between consecutive liquidations (seconds).
#[derive(Clone)]
pub struct LiquidationCooldown {
    /// Timestamp of the most recent liquidation (ms). `None` until first event.
    last_ts: Option<i64>,
    /// Cached delta between the two most recent events (seconds).
    last_delta_sec: f64,
}

impl LiquidationCooldown {
    /// Create a new cooldown tracker.
    pub fn new() -> Self {
        Self { last_ts: None, last_delta_sec: 0.0 }
    }
}

impl Default for LiquidationCooldown {
    fn default() -> Self {
        Self::new()
    }
}

impl LiquidationConsumer for LiquidationCooldown {
    fn update_liquidation(&mut self, liq: &Liquidation) -> IndicatorValue {
        if let Some(prev_ts) = self.last_ts {
            let delta_ms = liq.timestamp.saturating_sub(prev_ts);
            self.last_delta_sec = delta_ms as f64 / 1_000.0;
        }
        self.last_ts = Some(liq.timestamp);
        IndicatorValue::Single(self.last_delta_sec)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_delta_sec)
    }

    fn reset(&mut self) {
        self.last_ts = None;
        self.last_delta_sec = 0.0;
    }

    fn is_ready(&self) -> bool {
        // Ready only after at least two events (so we have a real delta).
        self.last_delta_sec > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::LiquidationSide;

    fn liq(ts: i64) -> Liquidation {
        Liquidation { side: LiquidationSide::Long, price: 30_000.0, quantity: 0.1, timestamp: ts, value: None }
    }

    #[test]
    fn zero_before_any_event() {
        let lc = LiquidationCooldown::new();
        assert_eq!(lc.value(), IndicatorValue::Single(0.0));
        assert!(!lc.is_ready());
    }

    #[test]
    fn zero_after_first_event() {
        let mut lc = LiquidationCooldown::new();
        let v = lc.update_liquidation(&liq(1_000));
        // No previous timestamp — delta stays 0.
        assert_eq!(v, IndicatorValue::Single(0.0));
        assert!(!lc.is_ready());
    }

    #[test]
    fn cooldown_after_second_event() {
        let mut lc = LiquidationCooldown::new();
        lc.update_liquidation(&liq(0));
        // 5 seconds later.
        let v = lc.update_liquidation(&liq(5_000));
        assert_eq!(v, IndicatorValue::Single(5.0));
        assert!(lc.is_ready());
    }

    #[test]
    fn successive_cooldowns() {
        let mut lc = LiquidationCooldown::new();
        lc.update_liquidation(&liq(0));
        lc.update_liquidation(&liq(2_000)); // 2 s
        let v3 = lc.update_liquidation(&liq(7_000)); // 5 s
        assert_eq!(v3, IndicatorValue::Single(5.0));
    }

    #[test]
    fn reset_clears_state() {
        let mut lc = LiquidationCooldown::new();
        lc.update_liquidation(&liq(0));
        lc.update_liquidation(&liq(3_000));
        lc.reset();
        assert_eq!(lc.value(), IndicatorValue::Single(0.0));
        assert!(!lc.is_ready());
    }

    #[test]
    fn no_underflow_on_equal_timestamps() {
        let mut lc = LiquidationCooldown::new();
        lc.update_liquidation(&liq(1_000));
        let v = lc.update_liquidation(&liq(1_000));
        // Same timestamp → 0 seconds cooldown.
        assert_eq!(v, IndicatorValue::Single(0.0));
    }
}
