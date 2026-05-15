//! TpoSessionBalance — TPO (Time/Price Opportunity) session balance point.
//!
//! Consumer: `TickConsumer`.
//!
//! Logic:
//! - Maintains rolling tick history in a time window.
//! - Buckets prices by `price_bucket` width.
//! - Balance Point = price level (bucket midpoint) with maximum TPO count.
//!
//! Output: `Triple(balance_price, max_tpo_count, total_buckets)`.

use std::collections::HashMap;
use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// TPO session balance point indicator.
///
/// Implements `TickConsumer`.
/// Parameters:
/// - `window_ms`   — rolling time window in milliseconds.
/// - `price_bucket`— bucket width in price units.
#[derive(Clone)]
pub struct TpoSessionBalance {
    window_ms: i64,
    price_bucket: f64,
    events: VecDeque<(i64, f64)>, // (ts, price)
    last_balance: f64,
    last_max_count: f64,
    last_buckets: f64,
}

impl TpoSessionBalance {
    /// Create a new indicator.
    pub fn new(window_ms: i64, price_bucket: f64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            price_bucket: price_bucket.max(f64::EPSILON),
            events: VecDeque::with_capacity(512),
            last_balance: 0.0,
            last_max_count: 0.0,
            last_buckets: 0.0,
        }
    }

    fn price_to_bucket(price: f64, bucket: f64) -> i64 {
        (price / bucket).floor() as i64
    }

    fn recompute(&mut self, fallback_price: f64) {
        if self.events.is_empty() {
            self.last_balance = fallback_price;
            self.last_max_count = 0.0;
            self.last_buckets = 0.0;
            return;
        }

        let mut counts: HashMap<i64, u64> = HashMap::new();
        for &(_, p) in &self.events {
            *counts.entry(Self::price_to_bucket(p, self.price_bucket)).or_insert(0) += 1;
        }

        let (&poc_bucket, &max_count) = counts
            .iter()
            .max_by_key(|(_, &c)| c)
            .unwrap(); // safe: non-empty

        let balance_price = poc_bucket as f64 * self.price_bucket + self.price_bucket / 2.0;
        self.last_balance = balance_price;
        self.last_max_count = max_count as f64;
        self.last_buckets = counts.len() as f64;
    }

    /// Passthrough for bar pipeline — returns current value.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_balance, self.last_max_count, self.last_buckets)
    }

    /// True when at least one tick has been received.
    pub fn indicator_is_ready(&self) -> bool {
        !self.events.is_empty()
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.events.clear();
        self.last_balance = 0.0;
        self.last_max_count = 0.0;
        self.last_buckets = 0.0;
    }
}

impl Default for TpoSessionBalance {
    fn default() -> Self {
        Self::new(3_600_000, 1.0)
    }
}

impl TickConsumer for TpoSessionBalance {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        // Evict stale events
        let cutoff = tick.time - self.window_ms;
        while self.events.front().map_or(false, |(ts, _)| *ts < cutoff) {
            self.events.pop_front();
        }
        self.events.push_back((tick.time, tick.price));
        self.recompute(tick.price);
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

    fn tick(time_ms: i64, price: f64) -> Tick {
        Tick::new(time_ms, price, 1.0, true)
    }

    #[test]
    fn balance_at_dominant_price_bucket() {
        let mut ind = TpoSessionBalance::new(60_000, 10.0);
        // 5 ticks at 100 (bucket 10), 1 at 200 (bucket 20)
        for i in 0..5 {
            ind.update_tick(&tick(i * 100, 105.0)); // bucket 10: midpoint 105
        }
        ind.update_tick(&tick(600, 205.0)); // bucket 20
        if let IndicatorValue::Triple(balance, max_count, buckets) = ind.indicator_value() {
            assert!((balance - 105.0).abs() < 1.0, "balance={balance}");
            assert_eq!(max_count as u64, 5, "max_count={max_count}");
            assert_eq!(buckets as u64, 2, "buckets={buckets}");
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn stale_events_evicted() {
        let mut ind = TpoSessionBalance::new(10_000, 10.0);
        // 5 ticks at 100 at t=0
        for i in 0..5 {
            ind.update_tick(&tick(i * 100, 105.0));
        }
        // new tick 20s later at 200 — old events evicted
        ind.update_tick(&tick(20_000, 205.0));
        if let IndicatorValue::Triple(_, _, buckets) = ind.indicator_value() {
            assert_eq!(buckets as u64, 1, "only 1 bucket should remain");
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = TpoSessionBalance::new(60_000, 10.0);
        ind.update_tick(&tick(1000, 100.0));
        assert!(ind.indicator_is_ready());
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        assert_eq!(ind.indicator_value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }
}
