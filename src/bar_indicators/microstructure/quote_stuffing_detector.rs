//! QuoteStuffingDetector — rolling L2 orderbook delta event rate anomaly.
//!
//! Counts the number of orderbook delta events within a rolling `window_ms`
//! window and computes the rate (events per second). If the rate exceeds
//! `rate_threshold`, a quote-stuffing signal is emitted.
//!
//! Output: `IndicatorValue::Double(rate_per_sec, is_stuffing)`.
//! - `rate_per_sec`: rolling event rate.
//! - `is_stuffing`:  `1.0` when rate > threshold, `0.0` otherwise.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::orderbook_delta_consumer::OrderbookDeltaConsumer;
use crate::core::types::OrderbookDelta;

/// Rolling L2 orderbook delta event rate / quote-stuffing detector.
///
/// Parameters:
/// - `window_ms`      — rolling time window in milliseconds (clamped ≥ 1).
/// - `rate_threshold` — events-per-second threshold above which stuffing is
///                      signalled (default 100 eps).
#[derive(Debug, Clone)]
pub struct QuoteStuffingDetector {
    window_ms: i64,
    rate_threshold: f64,
    /// Circular buffer of event timestamps (milliseconds).
    timestamps: VecDeque<i64>,
    last_rate: f64,
    last_signal: f64,
}

impl QuoteStuffingDetector {
    /// Create a new detector.
    ///
    /// - `window_ms`      — rolling window in milliseconds.
    /// - `rate_threshold` — events/sec threshold (clamped ≥ 0).
    pub fn new(window_ms: i64, rate_threshold: f64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            rate_threshold: rate_threshold.max(0.0),
            timestamps: VecDeque::with_capacity(1024),
            last_rate: 0.0,
            last_signal: 0.0,
        }
    }

    /// Convenience constructor with 100 eps default threshold.
    pub fn with_window(window_ms: i64) -> Self {
        Self::new(window_ms, 100.0)
    }
}

impl OrderbookDeltaConsumer for QuoteStuffingDetector {
    fn update_delta(&mut self, delta: &OrderbookDelta) -> IndicatorValue {
        let now = delta.timestamp;
        self.timestamps.push_back(now);

        // Evict events outside the rolling window.
        while let Some(&ts) = self.timestamps.front() {
            if now - ts > self.window_ms {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }

        let count = self.timestamps.len() as f64;
        // Window in seconds (avoid divide-by-zero; window_ms ≥ 1 so safe).
        let window_sec = self.window_ms as f64 / 1_000.0;
        self.last_rate = count / window_sec;
        self.last_signal = if self.last_rate > self.rate_threshold {
            1.0
        } else {
            0.0
        };

        IndicatorValue::Double(self.last_rate, self.last_signal)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.last_rate, self.last_signal)
    }

    fn reset(&mut self) {
        self.timestamps.clear();
        self.last_rate = 0.0;
        self.last_signal = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.timestamps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delta(timestamp: i64) -> OrderbookDelta {
        OrderbookDelta {
            bids: vec![],
            asks: vec![],
            timestamp,
            first_update_id: None,
            last_update_id: None,
            prev_update_id: None,
            ..Default::default()
        }
    }

    #[test]
    fn low_rate_no_signal() {
        // 5 events over 1 second → rate = 5 eps < 100 threshold.
        let mut det = QuoteStuffingDetector::new(1_000, 100.0);
        for i in 0..5 {
            det.update_delta(&delta(i * 200)); // 200 ms apart → 5 eps
        }
        if let IndicatorValue::Double(rate, signal) = det.value() {
            assert!(rate < 100.0, "rate {rate} should be < 100");
            assert!((signal).abs() < 1e-9, "signal should be 0 at low rate");
        } else {
            panic!("expected Double");
        }
    }

    #[test]
    fn high_rate_triggers_signal() {
        // 200 events in 1 second → rate = 200 eps > 100 threshold.
        let mut det = QuoteStuffingDetector::new(1_000, 100.0);
        for i in 0..200 {
            det.update_delta(&delta(i as i64 * 5)); // 5 ms apart → 200 eps
        }
        if let IndicatorValue::Double(rate, signal) = det.value() {
            assert!(rate > 100.0, "rate {rate} should be > 100");
            assert!((signal - 1.0).abs() < 1e-9, "signal should be 1.0 at high rate");
        } else {
            panic!("expected Double");
        }
    }

    #[test]
    fn stale_events_evicted() {
        let mut det = QuoteStuffingDetector::new(1_000, 100.0);
        // Insert 200 events at t=0..999 ms.
        for i in 0..200 {
            det.update_delta(&delta(i * 5));
        }
        // New event 2 s later — all old ones evicted.
        if let IndicatorValue::Double(rate, signal) = det.update_delta(&delta(2_000)) {
            // Only 1 event in window → rate = 1/1s = 1 eps.
            assert!(rate < 100.0, "rate after eviction should be low, got {rate}");
            assert!((signal).abs() < 1e-9, "signal should be 0 after eviction");
        } else {
            panic!("expected Double");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut det = QuoteStuffingDetector::new(1_000, 100.0);
        det.update_delta(&delta(0));
        assert!(det.is_ready());
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.value(), IndicatorValue::Double(0.0, 0.0));
    }
}
