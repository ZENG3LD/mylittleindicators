//! WarningRate — rolling rate of market warning events per minute.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::market_warning_consumer::MarketWarningConsumer;
use crate::core::types::MarketWarning;

/// Rolling rate of market warning events per minute within a sliding time window.
///
/// `warnings_per_min = event_count_in_window / window_minutes`
///
/// Output: `Single(warnings_per_min)`.
///
/// Default window: 5 minutes (300_000 ms).
#[derive(Clone)]
pub struct WarningRate {
    window_ms: i64,
    events: VecDeque<i64>,
    last_rate: f64,
}

impl WarningRate {
    /// Create a new indicator with explicit window in milliseconds.
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            events: VecDeque::new(),
            last_rate: 0.0,
        }
    }

    /// Called by `update_bar` passthrough — returns current rate.
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, _close: f64, _volume: f64) -> IndicatorValue {
        IndicatorValue::Single(self.last_rate)
    }

    fn evict_old(&mut self, current_ts: i64) {
        let cutoff = current_ts - self.window_ms;
        while self.events.front().map_or(false, |&ts| ts < cutoff) {
            self.events.pop_front();
        }
    }

    fn compute_rate(&self) -> f64 {
        let window_minutes = self.window_ms as f64 / 60_000.0;
        self.events.len() as f64 / window_minutes
    }
}

impl Default for WarningRate {
    fn default() -> Self {
        Self::new(300_000)
    }
}

impl MarketWarningConsumer for WarningRate {
    fn update_market_warning(&mut self, w: &MarketWarning) -> IndicatorValue {
        self.evict_old(w.timestamp);
        self.events.push_back(w.timestamp);
        self.last_rate = self.compute_rate();
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

    fn make_warning(timestamp: i64) -> MarketWarning {
        MarketWarning {
            symbol: "BTCUSDT".to_string(),
            warning_kind: "high_volatility".to_string(),
            message: "test".to_string(),
            timestamp,
        }
    }

    #[test]
    fn rate_increases_with_more_events() {
        let window_ms = 60_000i64; // 1 minute
        let mut ind = WarningRate::new(window_ms);
        // 3 events in 1-minute window = 3/min
        ind.update_market_warning(&make_warning(1000));
        ind.update_market_warning(&make_warning(2000));
        let val = ind.update_market_warning(&make_warning(3000));
        if let IndicatorValue::Single(r) = val {
            assert!((r - 3.0).abs() < 1e-9, "rate = {r}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn old_events_evicted() {
        let window_ms = 60_000i64;
        let mut ind = WarningRate::new(window_ms);
        // Add event at t=0
        ind.update_market_warning(&make_warning(0));
        // Add event well outside window (t = 2 minutes later)
        let val = ind.update_market_warning(&make_warning(120_001));
        // Only 1 event should remain in window
        if let IndicatorValue::Single(r) = val {
            // 1 event / 1 min = 1.0 rate
            assert!((r - 1.0).abs() < 1e-9, "rate = {r}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = WarningRate::new(60_000);
        ind.update_market_warning(&make_warning(1000));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::Single(0.0));
    }
}
