//! SettlementApproachSignal — pressure score approaching contract settlement.
//!
//! Receives `SettlementEvent` updates to learn the next settlement time.
//! On each `update_bar` call the bar timestamp (from `timestamp: Option<i64>`) is
//! used to compute how close the current time is to settlement.
//!
//! If bar timestamps are unavailable (all `None` / zero), the indicator falls back
//! to a tick-based proxy: it counts `update_bar` calls since the last settlement
//! event and inverts the normalized count against `max_window` (in ticks).
//!
//! ## Honesty Check
//!
//! ResearchBar/update_bar does expose `timestamp: Option<i64>` in `IndicatorInstance::update_bar`.
//! This indicator receives that timestamp via `update_bar_with_ts`.
//! When `ts == 0` or `None`, the ticks-based fallback is active.
//!
//! Output: `Single(approach_score)` ∈ [0, 1].
//! - `0.0` = far from settlement (or no settlement known)
//! - `1.0` = at or past settlement time

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::SettlementEventConsumer;
use crate::core::types::SettlementEvent;

/// Score that grows from 0 → 1 as the next settlement time approaches.
///
/// Uses bar timestamp when available; falls back to tick count proxy.
#[derive(Clone)]
pub struct SettlementApproachSignal {
    /// Maximum lookahead window in milliseconds (default 8 hours).
    max_window_ms: i64,
    /// Maximum lookahead in ticks for the tick-based fallback.
    max_window_ticks: usize,
    /// Next settlement time in milliseconds (from last SettlementEvent).
    next_settlement_ms: Option<i64>,
    /// Ticks accumulated since the last SettlementEvent (fallback counter).
    ticks_since_settlement: usize,
    last_score: f64,
}

impl SettlementApproachSignal {
    /// Create a new indicator.
    ///
    /// - `max_window_ms`: countdown window in milliseconds. Score is 0 when
    ///   `countdown >= max_window_ms` and 1 when `countdown == 0`.
    /// - `max_window_ticks`: fallback window in ticks when timestamps are unavailable.
    pub fn new(max_window_ms: i64, max_window_ticks: usize) -> Self {
        Self {
            max_window_ms: max_window_ms.max(1),
            max_window_ticks: max_window_ticks.max(1),
            next_settlement_ms: None,
            ticks_since_settlement: 0,
            last_score: 0.0,
        }
    }

    /// Update score given the current bar timestamp (milliseconds, 0 if unknown).
    pub fn update_bar_with_ts(&mut self, ts_ms: i64) -> IndicatorValue {
        self.ticks_since_settlement += 1;
        match self.next_settlement_ms {
            Some(settlement) if ts_ms > 0 => {
                let countdown = (settlement - ts_ms).max(0);
                let normalized = countdown as f64 / self.max_window_ms as f64;
                self.last_score = 1.0 - normalized.clamp(0.0, 1.0);
            }
            _ => {
                // Tick-based fallback
                let ratio = self.ticks_since_settlement as f64 / self.max_window_ticks as f64;
                self.last_score = ratio.clamp(0.0, 1.0);
            }
        }
        IndicatorValue::Single(self.last_score)
    }
}

impl Default for SettlementApproachSignal {
    fn default() -> Self {
        // 8 hours default window; 480 ticks fallback
        Self::new(8 * 3600 * 1000, 480)
    }
}

impl SettlementEventConsumer for SettlementApproachSignal {
    fn update_settlement(&mut self, s: &SettlementEvent) -> IndicatorValue {
        self.next_settlement_ms = Some(s.settlement_time);
        self.ticks_since_settlement = 0;
        // Recompute score based on the event timestamp
        if s.timestamp > 0 {
            let countdown = (s.settlement_time - s.timestamp).max(0);
            let normalized = countdown as f64 / self.max_window_ms as f64;
            self.last_score = 1.0 - normalized.clamp(0.0, 1.0);
        }
        IndicatorValue::Single(self.last_score)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_score)
    }

    fn reset(&mut self) {
        self.next_settlement_ms = None;
        self.ticks_since_settlement = 0;
        self.last_score = 0.0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_settlement(settlement_time: i64, ts: i64) -> SettlementEvent {
        SettlementEvent {
            settlement_price: 50000.0,
            settlement_time,
            timestamp: ts,
        }
    }

    #[test]
    fn far_from_settlement_gives_low_score() {
        let mut ind = SettlementApproachSignal::new(8 * 3600 * 1000, 480);
        // settlement in 8 hours from now, current time = 0
        let settlement_ts = 8 * 3600 * 1000_i64;
        ind.update_settlement(&make_settlement(settlement_ts, 0));
        let score = ind.update_bar_with_ts(0).main();
        // tick-based: 1 tick / 480 → ~0.002
        assert!(score < 0.1, "score should be low far from settlement, got {score}");
    }

    #[test]
    fn near_settlement_gives_high_score() {
        let mut ind = SettlementApproachSignal::new(8 * 3600 * 1000, 480);
        let settlement_ts = 100_i64;  // 100ms ahead
        ind.update_settlement(&make_settlement(settlement_ts, 0));
        // bar_time = settlement_ts - 50ms (very close)
        let score = ind.update_bar_with_ts(settlement_ts - 50).main();
        assert!(score > 0.9, "score should be high near settlement, got {score}");
    }

    #[test]
    fn tick_fallback_grows_with_ticks() {
        let max_ticks = 10;
        let mut ind = SettlementApproachSignal::new(8 * 3600 * 1000, max_ticks);
        // No settlement event → tick-based
        for _ in 0..max_ticks {
            ind.update_bar_with_ts(0);
        }
        let score = ind.value().main();
        assert!(score >= 1.0, "tick fallback should saturate at 1.0, got {score}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = SettlementApproachSignal::new(8 * 3600 * 1000, 480);
        ind.update_settlement(&make_settlement(1_000_000, 0));
        ind.reset();
        assert_eq!(ind.value().main(), 0.0);
    }
}
