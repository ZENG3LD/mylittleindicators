//! Iceberg Detector — tracks level replenishment events to detect hidden iceberg orders.
//!
//! An iceberg order repeatedly shows a small visible size at a price level.
//! When a level disappears (size=0) and reappears (size>0), that is one
//! replenishment cycle. A high replenishment count at one price suggests
//! a large hidden order being worked there.
//!
//! Output: `Triple(side, price, count)` where:
//! - `side`: +1.0 = bid-side iceberg, -1.0 = ask-side iceberg, 0.0 = none detected
//! - `price`: price of the most recently detected iceberg level
//! - `count`: replenishment count at that level (above threshold)

use std::collections::HashMap;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::orderbook_delta_consumer::OrderbookDeltaConsumer;
use crate::core::types::OrderbookDelta;

/// State for a single price level tracked by the detector.
#[derive(Default, Clone)]
struct LevelState {
    /// Last observed size (0.0 = removed).
    last_size: f64,
    /// How many times this level was replenished (removed then appeared again).
    replenishment_count: u32,
}

/// Detects iceberg orders by tracking level replenishment patterns in delta updates.
#[derive(Clone)]
pub struct IcebergDetector {
    /// Price bucket granularity for grouping nearby levels.
    price_bucket: f64,
    /// Minimum replenishment count to declare an iceberg.
    replenishment_threshold: u32,
    /// Per-bucket bid state.
    bid_levels: HashMap<i64, LevelState>,
    /// Per-bucket ask state.
    ask_levels: HashMap<i64, LevelState>,
    /// Side of last detected iceberg (+1 bid, -1 ask, 0 none).
    last_side: f64,
    /// Price of last detected iceberg level.
    last_price: f64,
    /// Replenishment count of last detected iceberg.
    last_count: f64,
    /// Whether at least one delta has been processed.
    has_data: bool,
}

impl IcebergDetector {
    /// Create a new iceberg detector.
    ///
    /// - `price_bucket`: bucket size for grouping price levels (e.g. 1.0 for whole numbers)
    /// - `replenishment_threshold`: min replenishments to flag as iceberg (e.g. 3)
    pub fn new(price_bucket: f64, replenishment_threshold: u32) -> Self {
        Self {
            price_bucket: price_bucket.max(1e-9),
            replenishment_threshold: replenishment_threshold.max(1),
            bid_levels: HashMap::new(),
            ask_levels: HashMap::new(),
            last_side: 0.0,
            last_price: 0.0,
            last_count: 0.0,
            has_data: false,
        }
    }

    #[inline]
    fn bucket(&self, price: f64) -> i64 {
        (price / self.price_bucket).floor() as i64
    }

    fn process_side(
        levels: &mut HashMap<i64, LevelState>,
        price: f64,
        size: f64,
        bucket: i64,
        threshold: u32,
        side: f64,
        last_side: &mut f64,
        last_price: &mut f64,
        last_count: &mut f64,
    ) {
        let entry = levels.entry(bucket).or_default();
        if size == 0.0 {
            // Level removed
            entry.last_size = 0.0;
        } else if entry.last_size == 0.0 && size > 0.0 {
            // Level reappeared — replenishment event
            entry.replenishment_count += 1;
            entry.last_size = size;
            if entry.replenishment_count >= threshold {
                *last_side = side;
                *last_price = price;
                *last_count = entry.replenishment_count as f64;
            }
        } else {
            // Size update (not a removal/replenishment)
            entry.last_size = size;
        }
    }
}

impl OrderbookDeltaConsumer for IcebergDetector {
    fn update_delta(&mut self, delta: &OrderbookDelta) -> IndicatorValue {
        self.has_data = true;

        for bid in &delta.bids {
            let bucket = self.bucket(bid.price);
            Self::process_side(
                &mut self.bid_levels,
                bid.price,
                bid.size,
                bucket,
                self.replenishment_threshold,
                1.0,
                &mut self.last_side,
                &mut self.last_price,
                &mut self.last_count,
            );
        }

        for ask in &delta.asks {
            let bucket = self.bucket(ask.price);
            Self::process_side(
                &mut self.ask_levels,
                ask.price,
                ask.size,
                bucket,
                self.replenishment_threshold,
                -1.0,
                &mut self.last_side,
                &mut self.last_price,
                &mut self.last_count,
            );
        }

        IndicatorValue::Triple(self.last_side, self.last_price, self.last_count)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_side, self.last_price, self.last_count)
    }

    fn reset(&mut self) {
        self.bid_levels.clear();
        self.ask_levels.clear();
        self.last_side = 0.0;
        self.last_price = 0.0;
        self.last_count = 0.0;
        self.has_data = false;
    }

    fn is_ready(&self) -> bool {
        self.has_data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBookLevel;

    fn make_delta(
        bids: &[(f64, f64)],
        asks: &[(f64, f64)],
        ts: i64,
    ) -> OrderbookDelta {
        OrderbookDelta {
            bids: bids.iter().map(|&(p, s)| OrderBookLevel::new(p, s)).collect(),
            asks: asks.iter().map(|&(p, s)| OrderBookLevel::new(p, s)).collect(),
            timestamp: ts,
            first_update_id: None,
            last_update_id: None,
            prev_update_id: None,
            ..Default::default()
        }
    }

    #[test]
    fn not_ready_initially() {
        let det = IcebergDetector::new(1.0, 3);
        assert!(!det.is_ready());
    }

    #[test]
    fn ready_after_first_delta() {
        let mut det = IcebergDetector::new(1.0, 3);
        det.update_delta(&make_delta(&[(100.0, 10.0)], &[], 1000));
        assert!(det.is_ready());
    }

    #[test]
    fn detects_bid_iceberg_after_replenishments() {
        let mut det = IcebergDetector::new(1.0, 3);
        // Level appears, disappears, reappears — 3 replenishments
        for i in 0..6 {
            if i % 2 == 0 {
                det.update_delta(&make_delta(&[(100.0, 5.0)], &[], i as i64 * 100));
            } else {
                det.update_delta(&make_delta(&[(100.0, 0.0)], &[], i as i64 * 100));
            }
        }
        let v = det.value();
        if let IndicatorValue::Triple(side, price, count) = v {
            assert!((side - 1.0).abs() < 1e-9, "expected bid side +1");
            assert!((price - 100.0).abs() < 1e-9);
            assert!(count >= 3.0);
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn no_detection_below_threshold() {
        let mut det = IcebergDetector::new(1.0, 5);
        // Only 2 replenishments — below threshold of 5
        for i in 0..4 {
            if i % 2 == 0 {
                det.update_delta(&make_delta(&[(100.0, 5.0)], &[], i as i64 * 100));
            } else {
                det.update_delta(&make_delta(&[(100.0, 0.0)], &[], i as i64 * 100));
            }
        }
        if let IndicatorValue::Triple(side, _, _) = det.value() {
            assert!((side - 0.0).abs() < 1e-9, "no iceberg below threshold");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut det = IcebergDetector::new(1.0, 2);
        det.update_delta(&make_delta(&[(100.0, 5.0)], &[], 1000));
        assert!(det.is_ready());
        det.reset();
        assert!(!det.is_ready());
        if let IndicatorValue::Triple(side, price, count) = det.value() {
            assert_eq!(side, 0.0);
            assert_eq!(price, 0.0);
            assert_eq!(count, 0.0);
        }
    }
}
