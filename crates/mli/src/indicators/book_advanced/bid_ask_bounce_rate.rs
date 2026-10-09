//! BidAskBounceRate — rolling rate of best bid/ask changes in the order book.
//!
//! Counts "bounces" (changes in best bid or best ask) over a rolling history window
//! and divides by the time span in seconds. If all timestamps are zero, divides
//! by the count of snapshots instead (tick-based rate).
//!
//! Output: `Single(bounce_rate)`.

use std::collections::VecDeque;

use crate::engine::streams::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Rolling rate of best bid / best ask changes.
///
/// A bounce is counted whenever the best bid **or** best ask differs
/// from the previous snapshot's best bid / best ask.
///
/// `rate = bounces / time_span_seconds` when timestamps are non-zero,
/// `rate = bounces / (n - 1)` when all timestamps are zero.
#[derive(Clone, Debug)]
pub struct BidAskBounceRate {
    window: usize,
    history: VecDeque<(i64, f64, f64)>, // (timestamp_ms, best_bid, best_ask)
    last_rate: f64,
}

impl BidAskBounceRate {
    /// Create a new indicator. `window` is clamped to at least 2.
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            history: VecDeque::new(),
            last_rate: 0.0,
        }
    }

    fn compute_rate(&self) -> f64 {
        let n = self.history.len();
        if n < 2 {
            return 0.0;
        }
        let mut bounces = 0usize;
        for i in 1..n {
            let (_, prev_bid, prev_ask) = self.history[i - 1];
            let (_, cur_bid, cur_ask) = self.history[i];
            if (cur_bid - prev_bid).abs() > f64::EPSILON || (cur_ask - prev_ask).abs() > f64::EPSILON {
                bounces += 1;
            }
        }
        let first_ts = self.history[0].0;
        let last_ts = self.history[n - 1].0;
        let time_span = (last_ts - first_ts) as f64 / 1000.0; // convert ms → seconds
        if time_span > 1e-9 {
            bounces as f64 / time_span
        } else {
            // tick-based fallback when timestamps are zero or identical
            bounces as f64 / (n - 1) as f64
        }
    }
}

impl Default for BidAskBounceRate {
    fn default() -> Self {
        Self::new(20)
    }
}

impl OrderBookConsumer for BidAskBounceRate {
    fn update_orderbook(&mut self, book: &OrderBook) {
        let best_bid = book.bids.first().map(|l| l.price).unwrap_or(0.0);
        let best_ask = book.asks.first().map(|l| l.price).unwrap_or(0.0);
        self.history.push_back((book.timestamp, best_bid, best_ask));
        while self.history.len() > self.window {
            self.history.pop_front();
        }
        self.last_rate = self.compute_rate();
    }


    fn reset(&mut self) {
        self.history.clear();
        self.last_rate = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= 2
    }
}

/// Typed configuration for [`BidAskBounceRate`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct BidAskBounceRateConfig {
    /// Rolling window size (number of order-book snapshots).
    pub window: crate::contract::Param<usize>,
}

impl Indicator for BidAskBounceRate {
    const ID: IndicatorId = IndicatorId::BidAskBounceRate;
    /// L2 order-book indicator — rate of best-level price changes.
    const FAMILY: &'static [Family] = &[Family::OrderBook];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderBook];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::BidAskBounceRate)];
    /// O(window) per update (full bounce scan of the deque). One window-depth deque.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = BidAskBounceRateConfig;
    type Runtime = BidAskBounceRate;

    fn create(cfg: BidAskBounceRateConfig) -> BidAskBounceRate {
        BidAskBounceRate::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for BidAskBounceRateConfig {
    fn defaults() -> Self {
        BidAskBounceRateConfig { window: crate::contract::Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // window: Class A (rolling window period) → auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for BidAskBounceRate {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::BidAskBounceRate, "Bounce Rate", Color::hex(0xFF9800), 2.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBook;
    use crate::contract::market_sample::MarketSample;
    use crate::engine::contract_engine::IndicatorOrder;

    fn make_book(best_bid: f64, best_ask: f64, ts: i64) -> OrderBook {
        let bids = [(best_bid, 100.0)];
        let asks = [(best_ask, 100.0)];
        let mut book = OrderBook::from_tuples(&bids, &asks, ts);
        book.timestamp = ts;
        book
    }

    #[test]
    fn all_same_prices_zero_rate() {
        let mut ind = BidAskBounceRate::new(5);
        for i in 0..5 {
            ind.update_orderbook(&make_book(100.0, 101.0, i as i64 * 1000));
        }
        // No bounces → rate = 0
        assert!((ind.value()).abs() < 1e-9, "expected 0.0 rate, got {}", ind.value());
    }

    #[test]
    fn alternating_best_bid_gives_positive_rate() {
        let mut ind = BidAskBounceRate::new(4);
        // t=0ms bid=100, t=1000ms bid=101, t=2000ms bid=100 → 2 bounces / 2s = 1.0
        ind.update_orderbook(&make_book(100.0, 101.0, 0));
        ind.update_orderbook(&make_book(101.0, 101.0, 1000));
        ind.update_orderbook(&make_book(100.0, 101.0, 2000));
        let v = ind.value();
        assert!(v > 0.0, "rate should be positive, got {v}");
        assert!((v - 1.0).abs() < 1e-9, "expected rate=1.0, got {v}");
    }

    #[test]
    fn zero_timestamps_tick_based_fallback() {
        let mut ind = BidAskBounceRate::new(5);
        // All timestamps 0, 3 bounces in 4 transitions → rate = 3/4
        let prices = [(100.0, 101.0), (101.0, 102.0), (100.0, 101.0), (101.0, 102.0), (100.0, 101.0)];
        for (bid, ask) in prices {
            ind.update_orderbook(&make_book(bid, ask, 0));
        }
        assert!(ind.value() > 0.0, "tick-based rate should be positive, got {}", ind.value());
    }

    #[test]
    fn not_ready_with_one_sample() {
        let mut ind = BidAskBounceRate::new(5);
        ind.update_orderbook(&make_book(100.0, 101.0, 0));
        assert!(!ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = BidAskBounceRate::new(4);
        ind.update_orderbook(&make_book(100.0, 101.0, 0));
        ind.update_orderbook(&make_book(101.0, 102.0, 1000));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_bid_ask_bounce_rate() {
        let mut f = IndicatorOrder::BidAskBounceRate(
            <<BidAskBounceRate as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // Feed a single snapshot — not yet ready, value should be 0
        let book = make_book(100.0, 101.0, 0);
        f.feed(0, MarketSample::OrderBook(&book));
        assert_eq!(f.primary(), 0.0);
        // Second snapshot with a changed bid — now we have a bounce
        let book2 = make_book(101.0, 101.0, 1000);
        f.feed(0, MarketSample::OrderBook(&book2));
        assert!(f.primary() > 0.0);
    }
}

impl BidAskBounceRate {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_rate
    }
}
