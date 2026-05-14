//! Liquidation Rate — rolling count of liquidation events per second.
//!
//! Tracks how many liquidation events occurred within the last `window_ms`
//! milliseconds and expresses the density as events per second.
//!
//! Output: `Single(rate)` — events per second over the rolling window.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::liquidation_consumer::LiquidationConsumer;
use crate::core::types::Liquidation;

/// Rolling liquidation event rate (events / second).
#[derive(Clone)]
pub struct LiquidationRate {
    /// Rolling window length in milliseconds.
    window_ms: i64,
    /// Timestamps of events still inside the window.
    events: VecDeque<i64>,
    /// Cached last computed rate.
    last_rate: f64,
}

impl LiquidationRate {
    /// Create with the given rolling window.
    ///
    /// `window_ms` — window size in milliseconds (e.g. `60_000` for 1 minute).
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            events: VecDeque::new(),
            last_rate: 0.0,
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

impl LiquidationConsumer for LiquidationRate {
    fn update_liquidation(&mut self, liq: &Liquidation) -> IndicatorValue {
        self.events.push_back(liq.timestamp);
        self.evict(liq.timestamp);
        let span_seconds = self.window_ms as f64 / 1_000.0;
        self.last_rate = self.events.len() as f64 / span_seconds;
        IndicatorValue::Single(self.last_rate)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_rate)
    }

    fn reset(&mut self) {
        self.events.clear();
        self.last_rate = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
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
    fn rate_zero_initially() {
        let lr = LiquidationRate::new(60_000);
        assert!(!lr.is_ready());
        assert_eq!(lr.value(), IndicatorValue::Single(0.0));
    }

    #[test]
    fn rate_after_single_event() {
        let mut lr = LiquidationRate::new(60_000);
        lr.update_liquidation(&liq(0));
        assert!(lr.is_ready());
        // 1 event in 60 s = 1/60 ≈ 0.01667 events/s
        if let IndicatorValue::Single(r) = lr.value() {
            assert!((r - 1.0 / 60.0).abs() < 1e-9);
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn old_events_evicted() {
        let mut lr = LiquidationRate::new(10_000); // 10 s window
        lr.update_liquidation(&liq(0));
        // event at t=0; update at t=15000 — event is outside 10 s window
        lr.update_liquidation(&liq(15_000));
        // only the t=15_000 event should remain
        if let IndicatorValue::Single(r) = lr.value() {
            let expected = 1.0 / 10.0; // 1 event in 10 s
            assert!((r - expected).abs() < 1e-9, "rate={r}");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut lr = LiquidationRate::new(60_000);
        lr.update_liquidation(&liq(0));
        lr.reset();
        assert!(!lr.is_ready());
        assert_eq!(lr.value(), IndicatorValue::Single(0.0));
    }
}
