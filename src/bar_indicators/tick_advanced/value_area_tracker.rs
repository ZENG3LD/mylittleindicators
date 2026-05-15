//! ValueAreaTracker — rolling Volume Profile Value Area.
//!
//! Maintains a time-windowed volume profile bucketed by price. Computes:
//! - **POC** (Point of Control) — price bucket with maximum volume.
//! - **VAH** (Value Area High) — upper bound of the Value Area.
//! - **VAL** (Value Area Low)  — lower bound of the Value Area.
//!
//! The Value Area is expanded symmetrically from the POC (higher-volume
//! neighbour first) until accumulated volume ≥ `value_area_pct` × total.
//!
//! Output: `IndicatorValue::Triple(poc_price, vah, val)`.

use std::collections::{HashMap, VecDeque};

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Map raw price to a bucket index.
#[inline]
fn bucket(price: f64, bucket_size: f64) -> i64 {
    (price / bucket_size).floor() as i64
}

/// Rolling Volume Profile Value Area tracker.
///
/// Parameters:
/// - `window_ms`       — rolling time window in milliseconds.
/// - `price_bucket`    — bucket width in price units (e.g. `1.0` for BTC).
/// - `value_area_pct`  — fraction of total volume that defines the Value Area (default 0.70).
#[derive(Debug, Clone)]
pub struct ValueAreaTracker {
    window_ms: i64,
    price_bucket: f64,
    value_area_pct: f64,
    /// Circular buffer: `(timestamp_ms, price, qty)`.
    events: VecDeque<(i64, f64, f64)>,
    last_poc: f64,
    last_vah: f64,
    last_val: f64,
}

impl ValueAreaTracker {
    /// Create a new tracker.
    ///
    /// - `window_ms`      — rolling window in milliseconds (clamped ≥ 1).
    /// - `price_bucket`   — bucket width > 0.
    /// - `value_area_pct` — fraction in (0, 1] for the value area (default 0.70).
    pub fn new(window_ms: i64, price_bucket: f64, value_area_pct: f64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            price_bucket: price_bucket.max(f64::EPSILON),
            value_area_pct: value_area_pct.clamp(f64::EPSILON, 1.0),
            events: VecDeque::with_capacity(512),
            last_poc: 0.0,
            last_vah: 0.0,
            last_val: 0.0,
        }
    }

    /// Convenience constructor with 70 % value area.
    pub fn with_window(window_ms: i64, price_bucket: f64) -> Self {
        Self::new(window_ms, price_bucket, 0.70)
    }

    /// Build volume profile and compute POC / VAH / VAL. Returns `(poc, vah, val)`.
    fn compute(
        events: &VecDeque<(i64, f64, f64)>,
        price_bucket: f64,
        value_area_pct: f64,
        fallback_price: f64,
    ) -> (f64, f64, f64) {
        if events.is_empty() {
            return (fallback_price, fallback_price, fallback_price);
        }

        // Build profile: bucket_index → accumulated volume.
        let mut profile: HashMap<i64, f64> = HashMap::new();
        for &(_, p, q) in events {
            *profile.entry(bucket(p, price_bucket)).or_insert(0.0) += q;
        }

        // POC = bucket with maximum volume.
        let (&poc_bucket, &poc_vol) = profile
            .iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap(); // safe: profile is non-empty

        let total: f64 = profile.values().sum();
        let target = total * value_area_pct;

        // Expand from POC outward, always taking the higher-volume neighbour.
        let mut va_buckets: Vec<i64> = vec![poc_bucket];
        let mut accumulated = poc_vol;
        let mut up = poc_bucket + 1;
        let mut down = poc_bucket - 1;

        while accumulated < target {
            let up_vol = profile.get(&up).copied().unwrap_or(0.0);
            let down_vol = profile.get(&down).copied().unwrap_or(0.0);
            if up_vol == 0.0 && down_vol == 0.0 {
                break;
            }
            if up_vol >= down_vol {
                va_buckets.push(up);
                accumulated += up_vol;
                up += 1;
            } else {
                va_buckets.push(down);
                accumulated += down_vol;
                down -= 1;
            }
        }

        let poc_price = poc_bucket as f64 * price_bucket + price_bucket / 2.0;
        let max_bucket = *va_buckets.iter().max().unwrap(); // safe: vec non-empty
        let min_bucket = *va_buckets.iter().min().unwrap();
        let vah = (max_bucket + 1) as f64 * price_bucket;
        let val = min_bucket as f64 * price_bucket;

        (poc_price, vah, val)
    }
}

impl TickConsumer for ValueAreaTracker {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        self.events.push_back((tick.time, tick.price, tick.size));

        // Evict stale events.
        while let Some(&(ts, _, _)) = self.events.front() {
            if tick.time - ts > self.window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }

        let (poc, vah, val) =
            Self::compute(&self.events, self.price_bucket, self.value_area_pct, tick.price);
        self.last_poc = poc;
        self.last_vah = vah;
        self.last_val = val;

        IndicatorValue::Triple(poc, vah, val)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_poc, self.last_vah, self.last_val)
    }

    fn reset(&mut self) {
        self.events.clear();
        self.last_poc = 0.0;
        self.last_vah = 0.0;
        self.last_val = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(time_ms: i64, price: f64, size: f64) -> Tick {
        Tick::new(time_ms, price, size, true)
    }

    #[test]
    fn single_bucket_poc_equals_price() {
        let mut vat = ValueAreaTracker::new(60_000, 10.0, 0.7);
        // All trades at the same price → single bucket.
        vat.update_tick(&tick(0, 100.0, 5.0));
        vat.update_tick(&tick(100, 105.0, 2.0)); // same bucket (100–110)
        let v = vat.update_tick(&tick(200, 102.0, 1.0));
        if let IndicatorValue::Triple(poc, vah, val) = v {
            // VAL ≤ POC ≤ VAH
            assert!(val <= poc && poc <= vah, "val={val} poc={poc} vah={vah}");
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn poc_at_dominant_bucket() {
        // Bucket 0 (0–10): size 100
        // Bucket 1 (10–20): size 1
        // → POC should be in bucket 0
        let mut vat = ValueAreaTracker::new(60_000, 10.0, 0.7);
        for i in 0..10 {
            vat.update_tick(&tick(i * 100, 5.0, 10.0)); // 10 × 10 = 100 in bucket 0
        }
        vat.update_tick(&tick(1_100, 15.0, 1.0)); // bucket 1
        if let IndicatorValue::Triple(poc, vah, val) = vat.value() {
            // POC midpoint of bucket 0 (0.0–10.0) = 5.0
            assert!((poc - 5.0).abs() < 1e-9, "POC expected 5.0, got {poc}");
            assert!(val <= poc && poc <= vah);
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn stale_events_evicted() {
        // Window = 10 s. Insert buys at t=0, then a single tick at t=15_000.
        // Old events should be dropped.
        let mut vat = ValueAreaTracker::new(10_000, 10.0, 0.7);
        for i in 0..5 {
            vat.update_tick(&tick(i * 100, 5.0, 100.0)); // in bucket 0
        }
        // New tick 15 s later — all old events gone.
        vat.update_tick(&tick(15_000, 250.0, 1.0));
        // Only the last tick is in window — POC is in bucket 25 (250/10=25).
        if let IndicatorValue::Triple(poc, _, _) = vat.value() {
            assert!((poc - 255.0).abs() < 1e-9, "POC should be midpoint of bucket 25, got {poc}");
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut vat = ValueAreaTracker::new(60_000, 10.0, 0.7);
        vat.update_tick(&tick(0, 100.0, 5.0));
        assert!(vat.is_ready());
        vat.reset();
        assert!(!vat.is_ready());
        assert_eq!(vat.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }
}
