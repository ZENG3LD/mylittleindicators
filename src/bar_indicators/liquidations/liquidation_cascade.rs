//! Liquidation Cascade — detect rapid bursts of forced liquidations.
//!
//! Triggers when the number of liquidation events within a rolling window
//! reaches or exceeds `threshold_count`.
//!
//! # Output
//! `Double(in_cascade, count)`
//!
//! - `in_cascade` — `1.0` when cascade is active, `0.0` otherwise.
//! - `count`      — number of events in the current window (as `f64`).

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::liquidation_consumer::LiquidationConsumer;
use crate::core::types::Liquidation;

/// Burst cascade detector for public liquidation streams.
#[derive(Clone)]
pub struct LiquidationCascade {
    /// Rolling window length in milliseconds.
    window_ms: i64,
    /// Number of events in window required to declare a cascade.
    threshold_count: usize,
    /// Timestamps of events still inside the window.
    events: VecDeque<i64>,
    /// Cached cascade flag.
    last_in_cascade: bool,
    /// Cached event count.
    last_count: usize,
}

impl LiquidationCascade {
    /// Create with the given window and threshold.
    ///
    /// - `window_ms`       — rolling window in milliseconds.
    /// - `threshold_count` — minimum events in window to signal a cascade.
    pub fn new(window_ms: i64, threshold_count: usize) -> Self {
        Self {
            window_ms: window_ms.max(1),
            threshold_count: threshold_count.max(1),
            events: VecDeque::new(),
            last_in_cascade: false,
            last_count: 0,
        }
    }

    fn evict(&mut self, now: i64) {
        while let Some(&front) = self.events.front() {
            if now - front > self.window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }
    }
}

impl LiquidationConsumer for LiquidationCascade {
    fn update_liquidation(&mut self, liq: &Liquidation) -> IndicatorValue {
        self.events.push_back(liq.timestamp);
        self.evict(liq.timestamp);
        self.last_count = self.events.len();
        self.last_in_cascade = self.last_count >= self.threshold_count;
        IndicatorValue::Double(
            if self.last_in_cascade { 1.0 } else { 0.0 },
            self.last_count as f64,
        )
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(
            if self.last_in_cascade { 1.0 } else { 0.0 },
            self.last_count as f64,
        )
    }

    fn reset(&mut self) {
        self.events.clear();
        self.last_in_cascade = false;
        self.last_count = 0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::TradeSide;

    fn liq(ts: i64) -> Liquidation {
        Liquidation { symbol: String::new(), side: TradeSide::Buy, price: 30_000.0, quantity: 0.1, timestamp: ts, value: None }
    }

    #[test]
    fn not_in_cascade_initially() {
        let lc = LiquidationCascade::new(10_000, 3);
        assert_eq!(lc.value(), IndicatorValue::Double(0.0, 0.0));
        assert!(!lc.is_ready());
    }

    #[test]
    fn below_threshold_no_cascade() {
        let mut lc = LiquidationCascade::new(10_000, 3);
        lc.update_liquidation(&liq(0));
        lc.update_liquidation(&liq(1_000));
        if let IndicatorValue::Double(flag, cnt) = lc.value() {
            assert_eq!(flag, 0.0);
            assert_eq!(cnt, 2.0);
        }
    }

    #[test]
    fn at_threshold_triggers_cascade() {
        let mut lc = LiquidationCascade::new(10_000, 3);
        lc.update_liquidation(&liq(0));
        lc.update_liquidation(&liq(1_000));
        lc.update_liquidation(&liq(2_000));
        if let IndicatorValue::Double(flag, cnt) = lc.value() {
            assert_eq!(flag, 1.0, "should be in cascade");
            assert_eq!(cnt, 3.0);
        }
    }

    #[test]
    fn cascade_drops_after_events_expire() {
        let mut lc = LiquidationCascade::new(5_000, 3);
        lc.update_liquidation(&liq(0));
        lc.update_liquidation(&liq(1_000));
        lc.update_liquidation(&liq(2_000));
        // cascade active
        if let IndicatorValue::Double(flag, _) = lc.value() {
            assert_eq!(flag, 1.0);
        }
        // new event 6 s later — all old events are outside 5 s window
        lc.update_liquidation(&liq(8_000));
        if let IndicatorValue::Double(flag, cnt) = lc.value() {
            assert_eq!(flag, 0.0, "cascade should end");
            assert_eq!(cnt, 1.0);
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut lc = LiquidationCascade::new(10_000, 3);
        lc.update_liquidation(&liq(0));
        lc.update_liquidation(&liq(1_000));
        lc.update_liquidation(&liq(2_000));
        lc.reset();
        assert!(!lc.is_ready());
        assert_eq!(lc.value(), IndicatorValue::Double(0.0, 0.0));
    }
}
