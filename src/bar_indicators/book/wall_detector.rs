//! WallDetector — detects anomalously large size levels in the order book.
//!
//! Maintains a rolling history of all level sizes seen across both sides.
//! A "wall" is any level whose size exceeds the Nth percentile of that history.
//!
//! Outputs the strongest bid wall and ask wall above threshold.
//!
//! Output: `IndicatorValue::Triple(bid_wall_price, ask_wall_price, total_wall_size)`

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;

/// Percentile-based order book wall detector.
#[derive(Debug, Clone)]
pub struct WallDetector {
    /// Number of historical size samples to maintain.
    history_window: usize,
    /// Percentile rank to use as threshold (e.g. 95.0).
    percentile_threshold: f64,
    /// Number of levels to sample per book snapshot.
    levels_to_sample: usize,
    size_history: VecDeque<f64>,
    last_bid_wall_price: f64,
    last_bid_wall_size: f64,
    last_ask_wall_price: f64,
    last_ask_wall_size: f64,
}

impl WallDetector {
    /// Create detector.
    ///
    /// - `history_window`: rolling sample capacity (e.g. 500).
    /// - `percentile_threshold`: threshold percentile (e.g. 95.0 = top 5% by size).
    /// - `levels_to_sample`: how many top levels to pull from each side per snapshot.
    pub fn new(history_window: usize, percentile_threshold: f64, levels_to_sample: usize) -> Self {
        let cap = history_window.max(10);
        let pct = percentile_threshold.clamp(50.0, 99.9);
        Self {
            history_window: cap,
            percentile_threshold: pct,
            levels_to_sample: levels_to_sample.max(1),
            size_history: VecDeque::with_capacity(cap),
            last_bid_wall_price: 0.0,
            last_bid_wall_size: 0.0,
            last_ask_wall_price: 0.0,
            last_ask_wall_size: 0.0,
        }
    }

    fn compute_threshold(&self) -> f64 {
        if self.size_history.is_empty() {
            return 0.0;
        }
        let mut sorted: Vec<f64> = self.size_history.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = ((self.percentile_threshold / 100.0) * sorted.len() as f64) as usize;
        sorted.get(idx.min(sorted.len().saturating_sub(1))).copied().unwrap_or(0.0)
    }
}

impl OrderBookConsumer for WallDetector {
    fn update_orderbook(&mut self, book: &OrderBook) -> IndicatorValue {
        // Add level sizes from top N levels of each side.
        for level in book.bids.iter().chain(book.asks.iter()).take(self.levels_to_sample * 2) {
            self.size_history.push_back(level.size);
            while self.size_history.len() > self.history_window {
                self.size_history.pop_front();
            }
        }

        if self.size_history.len() < self.history_window {
            return self.value();
        }

        let threshold = self.compute_threshold();

        // Find the largest bid level above threshold.
        if let Some(bid) = book.bids.iter()
            .take(self.levels_to_sample)
            .filter(|l| l.size >= threshold)
            .max_by(|a, b| a.size.partial_cmp(&b.size).unwrap_or(std::cmp::Ordering::Equal))
        {
            self.last_bid_wall_price = bid.price;
            self.last_bid_wall_size = bid.size;
        }

        // Find the largest ask level above threshold.
        if let Some(ask) = book.asks.iter()
            .take(self.levels_to_sample)
            .filter(|l| l.size >= threshold)
            .max_by(|a, b| a.size.partial_cmp(&b.size).unwrap_or(std::cmp::Ordering::Equal))
        {
            self.last_ask_wall_price = ask.price;
            self.last_ask_wall_size = ask.size;
        }

        IndicatorValue::Triple(
            self.last_bid_wall_price,
            self.last_ask_wall_price,
            self.last_bid_wall_size + self.last_ask_wall_size,
        )
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(
            self.last_bid_wall_price,
            self.last_ask_wall_price,
            self.last_bid_wall_size + self.last_ask_wall_size,
        )
    }

    fn reset(&mut self) {
        self.size_history.clear();
        self.last_bid_wall_price = 0.0;
        self.last_bid_wall_size = 0.0;
        self.last_ask_wall_price = 0.0;
        self.last_ask_wall_size = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.size_history.len() >= self.history_window
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBook;

    fn make_book(bids: &[(f64, f64)], asks: &[(f64, f64)]) -> OrderBook {
        OrderBook::from_tuples(bids, asks, 0)
    }

    fn large_book(bid_wall_size: f64, ask_wall_size: f64) -> OrderBook {
        // Normal levels + one oversized wall level
        make_book(
            &[(100.0, 1.0), (99.0, 1.0), (98.0, bid_wall_size)],
            &[(101.0, 1.0), (102.0, 1.0), (103.0, ask_wall_size)],
        )
    }

    #[test]
    fn not_ready_until_history_full() {
        // small window = 10 samples; levels_to_sample=2 → 4 samples per snapshot
        let mut det = WallDetector::new(20, 80.0, 2);
        let book = make_book(&[(100.0, 1.0), (99.0, 1.0)], &[(101.0, 1.0), (102.0, 1.0)]);
        // 4 samples per call → need 5 calls to fill 20-sample window
        for _ in 0..4 {
            det.update_orderbook(&book);
            assert!(!det.is_ready());
        }
        det.update_orderbook(&book);
        assert!(det.is_ready());
    }

    #[test]
    fn wall_detected_after_warmup() {
        let mut det = WallDetector::new(10, 80.0, 3);
        // Feed uniform small books to fill history
        let normal_book = make_book(
            &[(100.0, 1.0), (99.0, 1.0), (98.0, 1.0)],
            &[(101.0, 1.0), (102.0, 1.0), (103.0, 1.0)],
        );
        // Need 10 samples at 6/call → 2 calls fills it
        det.update_orderbook(&normal_book);
        det.update_orderbook(&normal_book);
        assert!(det.is_ready());

        // Now feed a book with a large wall
        let wall_book = large_book(1000.0, 800.0);
        let v = det.update_orderbook(&wall_book);
        match v {
            IndicatorValue::Triple(bid_p, ask_p, total_wall) => {
                // Wall sizes are huge → should be detected
                assert!(total_wall > 0.0, "expected wall detected: {}", total_wall);
                let _ = bid_p;
                let _ = ask_p;
            }
            other => panic!("expected Triple, got {:?}", other),
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut det = WallDetector::new(10, 95.0, 3);
        let book = make_book(
            &[(100.0, 1.0), (99.0, 1.0), (98.0, 1.0)],
            &[(101.0, 1.0), (102.0, 1.0), (103.0, 1.0)],
        );
        det.update_orderbook(&book);
        det.update_orderbook(&book);
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }

}
