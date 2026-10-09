//! BookPressure — slope momentum of bid/ask depth changes.
//!
//! Tracks rolling N snapshots of bid_depth and ask_depth.
//! `pressure = bid_slope - ask_slope`
//!
//! Positive = bid pressure growing faster (bullish).
//! Negative = ask pressure growing faster (bearish).

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::order_book_consumer::OrderBookConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::OrderBook;

/// Slope momentum of bid vs ask depth changes over a rolling window.
#[derive(Clone, Debug)]
pub struct BookPressure {
    window: usize,
    levels_to_aggregate: usize,
    bid_history: VecDeque<f64>,
    ask_history: VecDeque<f64>,
    last_pressure: f64,
}

impl BookPressure {
    /// Create with given window size and number of price levels to aggregate.
    ///
    /// - `window`: number of snapshots for slope computation (min 2)
    /// - `levels`: number of L2 depth levels to sum
    pub fn new(window: usize, levels: usize) -> Self {
        Self {
            window: window.max(2),
            levels_to_aggregate: levels.max(1),
            bid_history: VecDeque::new(),
            ask_history: VecDeque::new(),
            last_pressure: 0.0,
        }
    }
}

impl Default for BookPressure {
    fn default() -> Self {
        Self::new(10, 5)
    }
}

impl OrderBookConsumer for BookPressure {
    fn update_orderbook(&mut self, book: &OrderBook) {
        let bid = book.bid_depth(self.levels_to_aggregate);
        let ask = book.ask_depth(self.levels_to_aggregate);

        self.bid_history.push_back(bid);
        self.ask_history.push_back(ask);
        if self.bid_history.len() > self.window {
            self.bid_history.pop_front();
            self.ask_history.pop_front();
        }

        if self.bid_history.len() < 2 {
            return;
        }

        let n = self.bid_history.len() as f64;
        let bid_slope = (self.bid_history.back().copied().unwrap_or(0.0)
            - self.bid_history.front().copied().unwrap_or(0.0))
            / n;
        let ask_slope = (self.ask_history.back().copied().unwrap_or(0.0)
            - self.ask_history.front().copied().unwrap_or(0.0))
            / n;

        self.last_pressure = bid_slope - ask_slope;
    }


    fn reset(&mut self) {
        self.bid_history.clear();
        self.ask_history.clear();
        self.last_pressure = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.bid_history.len() >= self.window
    }
}

/// Typed configuration for [`BookPressure`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct BookPressureConfig {
    /// Rolling window size (number of snapshots).
    pub window: crate::contract::Param<usize>,
    /// Number of L2 levels to aggregate per side.
    pub levels: crate::contract::Param<usize>,
}

impl Indicator for BookPressure {
    const ID: IndicatorId = IndicatorId::BookPressure;
    /// L2 order-book pressure indicator.
    const FAMILY: &'static [Family] = &[Family::OrderBook];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderBook];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::BookPressure)];
    /// O(1) per update — window trim + slope from front/back.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[
            Store::window(StoreKind::Deque),
            Store::window(StoreKind::Deque),
        ],
    );
    type Config = BookPressureConfig;
    type Runtime = BookPressure;

    fn create(cfg: BookPressureConfig) -> BookPressure {
        BookPressure::new(cfg.window.resolved(), cfg.levels.resolved())
    }
}

impl crate::contract::Config for BookPressureConfig {
    fn defaults() -> Self {
        BookPressureConfig {
            window: crate::contract::Param::Solo(10),
            levels: crate::contract::Param::Solo(5),
        }
    }
    fn machine_defaults() -> Self {
        use crate::contract::Param;
        let mut s = Self::machine_defaults_auto();
        // window: Class A (snapshot window) — auto range(2,4048,1) is correct.
        // levels: Class B (order-book depth count, 1–50) — correct off auto range(2,4048,1).
        s.levels = Param::range(1, 50, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for BookPressure {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::BookPressure, "Book Pressure", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{OrderBook, OrderBookLevel};
    use crate::contract::market_sample::MarketSample;
    use crate::engine::contract_engine::IndicatorOrder;

    fn make_book(bid_size: f64, ask_size: f64) -> OrderBook {
        OrderBook {
            bids: vec![OrderBookLevel::new(100.0, bid_size)],
            asks: vec![OrderBookLevel::new(101.0, ask_size)],
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn new_not_ready() {
        let bp = BookPressure::new(5, 1);
        assert!(!bp.is_ready());
        assert_eq!(bp.value(), 0.0);
    }

    #[test]
    fn bid_pressure_growing_gives_positive() {
        let mut bp = BookPressure::new(5, 1);
        // bid_depth grows 100 → 200, ask stays 100
        for i in 0..5 {
            let bid = 100.0 + i as f64 * 25.0; // 100, 125, 150, 175, 200
            bp.update_orderbook(&make_book(bid, 100.0));
        }
        assert!(bp.is_ready());
        assert!(bp.value() > 0.0);
    }

    #[test]
    fn ask_pressure_growing_gives_negative() {
        let mut bp = BookPressure::new(5, 1);
        // ask_depth grows 100 → 200, bid stays 100
        for i in 0..5 {
            let ask = 100.0 + i as f64 * 25.0;
            bp.update_orderbook(&make_book(100.0, ask));
        }
        assert!(bp.is_ready());
        assert!(bp.value() < 0.0);
    }

    #[test]
    fn stable_book_gives_zero_pressure() {
        let mut bp = BookPressure::new(5, 1);
        for _ in 0..6 {
            bp.update_orderbook(&make_book(100.0, 100.0));
        }
        assert!(bp.is_ready());
        assert!((bp.value()).abs() < 1e-10);
    }

    #[test]
    fn reset_clears_state() {
        let mut bp = BookPressure::new(3, 1);
        for _ in 0..4 {
            bp.update_orderbook(&make_book(50.0, 50.0));
        }
        assert!(bp.is_ready());
        bp.reset();
        assert!(!bp.is_ready());
        assert_eq!(bp.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_book_pressure() {
        let mut f = IndicatorOrder::BookPressure(
            <<BookPressure as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let book = make_book(100.0, 100.0);
        for _ in 0..12 {
            f.feed(0, MarketSample::OrderBook(&book));
        }
        // Stable book → pressure near zero
        assert!(f.primary().abs() < 1e-10);
    }
}

impl BookPressure {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_pressure
    }
}
