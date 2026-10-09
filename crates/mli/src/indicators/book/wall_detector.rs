//! WallDetector — detects anomalously large size levels in the order book.
//!
//! Maintains a rolling history of all level sizes seen across both sides.
//! A "wall" is any level whose size exceeds the Nth percentile of that history.
//!
//! Outputs the strongest bid wall and ask wall above threshold.
//!
//! Outputs: `bid_wall_price`, `ask_wall_price`, `total_wall_size`

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::order_book_consumer::OrderBookConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;
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

impl Default for WallDetector {
    fn default() -> Self {
        Self::new(200, 95.0, 20)
    }
}

impl WallDetector {
    /// Price of the strongest detected bid wall.
    pub fn bid_price(&self) -> f64 {
        self.last_bid_wall_price
    }

    /// Price of the strongest detected ask wall.
    pub fn ask_price(&self) -> f64 {
        self.last_ask_wall_price
    }

    /// Combined size of the detected bid + ask walls.
    pub fn total_size(&self) -> f64 {
        self.last_bid_wall_size + self.last_ask_wall_size
    }
}

impl OrderBookConsumer for WallDetector {
    fn update_orderbook(&mut self, book: &OrderBook) {
        // Add level sizes from top N levels of each side.
        for level in book.bids.iter().chain(book.asks.iter()).take(self.levels_to_sample * 2) {
            self.size_history.push_back(level.size);
            while self.size_history.len() > self.history_window {
                self.size_history.pop_front();
            }
        }

        if self.size_history.len() < self.history_window {
            return;
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

/// Typed configuration for [`WallDetector`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct WallDetectorConfig {
    /// Rolling history capacity (number of size samples).
    pub history_window: crate::contract::Param<usize>,
    /// Percentile threshold for wall detection (50–99.9).
    pub percentile_threshold: crate::contract::Param<f64>,
    /// Number of book levels to sample per snapshot per side.
    pub levels_to_sample: crate::contract::Param<usize>,
}

impl Indicator for WallDetector {
    const ID: IndicatorId = IndicatorId::WallDetector;
    /// L2 order-book wall detector.
    const FAMILY: &'static [Family] = &[Family::OrderBook];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderBook];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::WallDetectorBidPrice),
        Output::price(IndicatorOutputId::WallDetectorAskPrice),
        Output::count(IndicatorOutputId::WallDetectorTotalSize),
    ];
    /// O(window * log window) per update — sort for percentile computation.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = WallDetectorConfig;
    type Runtime = WallDetector;

    fn create(cfg: WallDetectorConfig) -> WallDetector {
        WallDetector::new(cfg.history_window.resolved(), cfg.percentile_threshold.resolved(), cfg.levels_to_sample.resolved())
    }
}

impl crate::contract::Config for WallDetectorConfig {
    fn defaults() -> Self {
        WallDetectorConfig {
            history_window: crate::contract::Param::Solo(200),
            percentile_threshold: crate::contract::Param::Solo(95.0),
            levels_to_sample: crate::contract::Param::Solo(20),
        }
    }
    fn machine_defaults() -> Self {
        use crate::contract::{Param, sweep_f64};
        let mut s = Self::machine_defaults_auto();
        // history_window: Class A (rolling sample window) — auto range(2,4048,1) is correct.
        // percentile_threshold: Class D sub-case, 0–100 scale → sweep_f64(50.0, 99.0, 1.0).
        s.percentile_threshold = Param::many(sweep_f64(50.0, 99.0, 1.0));
        // levels_to_sample: Class B (order-book depth count, 1–50) — correct off auto range(2,4048,1).
        s.levels_to_sample = Param::range(1, 50, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for WallDetector {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::WallDetectorBidPrice, "Bid Wall Price", Color::hex(0x4CAF50), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::WallDetectorAskPrice, "Ask Wall Price", Color::hex(0xF44336), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::WallDetectorTotalSize, "Total Wall Size", Color::hex(0xFF9800), 2.0))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBook;
    use crate::contract::market_sample::MarketSample;
    use crate::engine::contract_engine::IndicatorOrder;

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
        det.update_orderbook(&wall_book);
        assert!(det.total_size() > 0.0, "expected wall detected: {}", det.total_size());
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
        assert_eq!(det.bid_price(), 0.0);
        assert_eq!(det.ask_price(), 0.0);
        assert_eq!(det.total_size(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_wall_detector() {
        // Build with small history to get it ready quickly
        let cfg = WallDetectorConfig {
            history_window: crate::contract::Param::Solo(10),
            percentile_threshold: crate::contract::Param::Solo(80.0),
            levels_to_sample: crate::contract::Param::Solo(3),
        };
        let mut f = IndicatorOrder::WallDetector(cfg).build_solo().unwrap();
        let book = make_book(
            &[(100.0, 1.0), (99.0, 1.0), (98.0, 1.0)],
            &[(101.0, 1.0), (102.0, 1.0), (103.0, 1.0)],
        );
        for _ in 0..2 {
            f.feed(0, MarketSample::OrderBook(&book));
        }
        // After warmup, factory primary = bid_price; returns f64
        let _ = f.primary();
    }
}
