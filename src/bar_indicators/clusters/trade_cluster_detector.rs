//! TradeClusterDetector — detects series of trades at the same price level.
//!
//! Multiple trades at the same price bucket within a time window suggest
//! hidden iceberg orders or large institutional interest at that level.
//!
//! Output: `IndicatorValue::Triple(signal, cluster_price, cluster_size)`
//!   signal: +1.0 = buy cluster, -1.0 = sell cluster, 0.0 = none

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Detects repeated trades at the same price bucket (iceberg / cluster signal).
#[derive(Debug, Clone)]
pub struct TradeClusterDetector {
    /// Price bucket size for rounding (e.g. 0.01 = 1 cent buckets).
    price_bucket: f64,
    /// Minimum ticks at same price level to declare a cluster.
    cluster_threshold: usize,
    /// Time window in milliseconds.
    window_ms: i64,
    /// Ring buffer of (price_bucket_id, timestamp_ms, is_buy, size).
    recent_ticks: VecDeque<(i64, i64, bool, f64)>,
    last_cluster_price: f64,
    last_cluster_size: f64,
    last_signal: f64,
}

impl TradeClusterDetector {
    /// Create detector.
    ///
    /// - `price_bucket`: rounding step for price levels (e.g. 0.01).
    /// - `cluster_threshold`: min ticks at same price bucket to trigger.
    /// - `window_ms`: rolling time window in milliseconds.
    pub fn new(price_bucket: f64, cluster_threshold: usize, window_ms: i64) -> Self {
        let bucket = if price_bucket > 0.0 { price_bucket } else { 0.01 };
        Self {
            price_bucket: bucket,
            cluster_threshold: cluster_threshold.max(2),
            window_ms: window_ms.max(1),
            recent_ticks: VecDeque::with_capacity(256),
            last_cluster_price: 0.0,
            last_cluster_size: 0.0,
            last_signal: 0.0,
        }
    }
}

impl TickConsumer for TradeClusterDetector {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        let bucket_id = (tick.price / self.price_bucket).floor() as i64;
        self.recent_ticks.push_back((bucket_id, tick.time, tick.is_buy, tick.size));

        // Evict ticks outside the time window.
        while let Some(&(_, ts, _, _)) = self.recent_ticks.front() {
            if tick.time - ts > self.window_ms {
                self.recent_ticks.pop_front();
            } else {
                break;
            }
        }

        // Count ticks in current bucket.
        let count = self.recent_ticks.iter()
            .filter(|&&(b, _, _, _)| b == bucket_id)
            .count();

        if count >= self.cluster_threshold {
            self.last_cluster_price = bucket_id as f64 * self.price_bucket;
            self.last_cluster_size = self.recent_ticks.iter()
                .filter(|&&(b, _, _, _)| b == bucket_id)
                .map(|&(_, _, _, s)| s)
                .sum();
            let buy_count = self.recent_ticks.iter()
                .filter(|&&(b, _, is_buy, _)| b == bucket_id && is_buy)
                .count();
            let sell_count = count - buy_count;
            self.last_signal = if buy_count > sell_count {
                1.0
            } else if sell_count > buy_count {
                -1.0
            } else {
                0.0
            };
        } else {
            self.last_signal = 0.0;
        }

        IndicatorValue::Triple(self.last_signal, self.last_cluster_price, self.last_cluster_size)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_signal, self.last_cluster_price, self.last_cluster_size)
    }

    fn reset(&mut self) {
        self.recent_ticks.clear();
        self.last_cluster_price = 0.0;
        self.last_cluster_size = 0.0;
        self.last_signal = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.recent_ticks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Tick;

    fn tick_at(price: f64, is_buy: bool, time_ms: i64) -> Tick {
        Tick::new(time_ms, price, 1.0, is_buy)
    }

    #[test]
    fn cluster_detected_when_threshold_reached() {
        // threshold=3, bucket=1.0, window=10000ms
        let mut det = TradeClusterDetector::new(1.0, 3, 10_000);
        // 3 buys at same price → cluster detected
        for i in 0..3 {
            det.update_tick(&tick_at(100.0, true, i as i64 * 100));
        }
        match det.value() {
            IndicatorValue::Triple(signal, price, size) => {
                assert!((signal - 1.0).abs() < 1e-9, "expected buy signal: {}", signal);
                assert!((price - 100.0).abs() < 1e-9, "expected price 100: {}", price);
                assert!((size - 3.0).abs() < 1e-9, "expected size 3: {}", size);
            }
            other => panic!("expected Triple, got {:?}", other),
        }
    }

    #[test]
    fn no_cluster_below_threshold() {
        let mut det = TradeClusterDetector::new(1.0, 5, 10_000);
        for i in 0..4 {
            det.update_tick(&tick_at(100.0, true, i as i64 * 100));
        }
        // signal should still be 0 — not enough ticks
        assert_eq!(det.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }

    #[test]
    fn old_ticks_evicted_by_time_window() {
        let mut det = TradeClusterDetector::new(1.0, 3, 1_000); // 1s window
        // 3 ticks within window
        det.update_tick(&tick_at(100.0, true, 0));
        det.update_tick(&tick_at(100.0, true, 500));
        det.update_tick(&tick_at(100.0, true, 900));
        assert!((det.value().main() - 1.0).abs() < 1e-9);

        // New tick 2s later — all old ticks evicted
        det.update_tick(&tick_at(100.0, true, 2_100));
        // Only 1 tick left in window — no cluster
        assert_eq!(det.last_signal, 0.0);
    }

    #[test]
    fn reset_clears_state() {
        let mut det = TradeClusterDetector::new(1.0, 3, 10_000);
        for i in 0..3 {
            det.update_tick(&tick_at(100.0, true, i as i64 * 100));
        }
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }
}
