//! Liquidation Volume Velocity — rolling USD liquidation volume per minute.
//!
//! Measures the rate at which USD volume is being force-liquidated over a
//! rolling time window, normalised to "per minute" for easy comparison
//! across different window sizes.
//!
//! # Algorithm
//! - Accumulates `(timestamp, quote_value)` pairs in a `VecDeque`.
//! - On each `update_liquidation` evicts entries older than `window_ms`.
//! - Velocity = total_quote_value_in_window / (window_ms / 60_000).
//!
//! # Output
//! `Single(usd_per_minute)` — always ≥ 0.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::liquidation_consumer::LiquidationConsumer;
use crate::core::types::Liquidation;

/// Rolling USD liquidation volume per minute.
#[derive(Clone)]
pub struct LiquidationVolumeVelocity {
    /// Rolling window length in milliseconds.
    window_ms: i64,
    /// `(timestamp_ms, quote_value)` pairs still inside the window.
    events: VecDeque<(i64, f64)>,
    /// Sum of quote values currently in `events`.
    total_value: f64,
    /// Cached last velocity (USD / minute).
    last_velocity: f64,
}

impl LiquidationVolumeVelocity {
    /// Create with the given rolling window.
    ///
    /// `window_ms` — window size in milliseconds (e.g. `60_000` for 1 minute).
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            events: VecDeque::new(),
            total_value: 0.0,
            last_velocity: 0.0,
        }
    }

    fn evict(&mut self, now: i64) {
        while let Some(&(ts, val)) = self.events.front() {
            if now - ts > self.window_ms {
                self.events.pop_front();
                self.total_value -= val;
            } else {
                break;
            }
        }
        // Guard against floating-point drift going slightly negative.
        if self.total_value < 0.0 {
            self.total_value = 0.0;
        }
    }
}

impl LiquidationConsumer for LiquidationVolumeVelocity {
    fn update_liquidation(&mut self, liq: &Liquidation) -> IndicatorValue {
        let qv = liq.quote_value();
        self.events.push_back((liq.timestamp, qv));
        self.total_value += qv;
        self.evict(liq.timestamp);
        // window_ms / 60_000 = number of minutes represented by the window
        let window_minutes = self.window_ms as f64 / 60_000.0;
        self.last_velocity = self.total_value / window_minutes;
        IndicatorValue::Single(self.last_velocity)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_velocity)
    }

    fn reset(&mut self) {
        self.events.clear();
        self.total_value = 0.0;
        self.last_velocity = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::TradeSide;

    fn liq(ts: i64, price: f64, qty: f64) -> Liquidation {
        Liquidation { symbol: String::new(), side: TradeSide::Buy, price, quantity: qty, timestamp: ts, value: None }
    }

    #[test]
    fn zero_initially() {
        let lvv = LiquidationVolumeVelocity::new(60_000);
        assert!(!lvv.is_ready());
        assert_eq!(lvv.value(), IndicatorValue::Single(0.0));
    }

    #[test]
    fn single_event_velocity() {
        // 1-minute window, 1 event with $30_000 quote value.
        // velocity = 30_000 / 1 min = 30_000 usd/min.
        let mut lvv = LiquidationVolumeVelocity::new(60_000);
        lvv.update_liquidation(&liq(0, 30_000.0, 1.0));
        assert!(lvv.is_ready());
        if let IndicatorValue::Single(v) = lvv.value() {
            assert!((v - 30_000.0).abs() < 1e-6, "v={v}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn two_events_sum() {
        // Two events of $10_000 each, 60s window → 20_000 / min.
        let mut lvv = LiquidationVolumeVelocity::new(60_000);
        lvv.update_liquidation(&liq(0, 10_000.0, 1.0));
        lvv.update_liquidation(&liq(1_000, 10_000.0, 1.0));
        if let IndicatorValue::Single(v) = lvv.value() {
            assert!((v - 20_000.0).abs() < 1e-6, "v={v}");
        }
    }

    #[test]
    fn old_events_evicted() {
        // 10-second window (10_000 ms).
        // Event at t=0 ($30_000). New event at t=15_000 ($5_000).
        // First event is outside window → velocity = 5_000 / (10_000/60_000) = 30_000.
        let mut lvv = LiquidationVolumeVelocity::new(10_000);
        lvv.update_liquidation(&liq(0, 30_000.0, 1.0));
        lvv.update_liquidation(&liq(15_000, 5_000.0, 1.0));
        let window_minutes = 10_000.0_f64 / 60_000.0;
        let expected = 5_000.0 / window_minutes;
        if let IndicatorValue::Single(v) = lvv.value() {
            assert!((v - expected).abs() < 1e-3, "v={v}, expected={expected}");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut lvv = LiquidationVolumeVelocity::new(60_000);
        lvv.update_liquidation(&liq(0, 30_000.0, 1.0));
        lvv.reset();
        assert!(!lvv.is_ready());
        assert_eq!(lvv.value(), IndicatorValue::Single(0.0));
    }
}
