//! BestLevelVolatility — rolling standard deviation of best bid size and best ask size.
//!
//! Measures instability of the top-of-book. High values indicate the best level
//! is frequently refreshed with large size swings (aggressive quoting).
//!
//! Output: `Triple(std_bid_size, std_ask_size, max(std_bid, std_ask))`

use std::collections::VecDeque;

use crate::engine::streams::order_book_consumer::OrderBookConsumer;
use crate::core::types::OrderBook;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Rolling std-dev of best bid/ask sizes.
#[derive(Clone, Debug)]
pub struct BestLevelVolatility {
    window: usize,
    bid_sizes: VecDeque<f64>,
    ask_sizes: VecDeque<f64>,
    last_std_bid: f64,
    last_std_ask: f64,
    last_max: f64,
}

impl BestLevelVolatility {
    /// Create with rolling window size.
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            bid_sizes: VecDeque::new(),
            ask_sizes: VecDeque::new(),
            last_std_bid: 0.0,
            last_std_ask: 0.0,
            last_max: 0.0,
        }
    }

    fn std_dev(buf: &VecDeque<f64>) -> f64 {
        let n = buf.len();
        if n < 2 {
            return 0.0;
        }
        let mean = buf.iter().sum::<f64>() / n as f64;
        let variance = buf.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        variance.sqrt()
    }
}

impl BestLevelVolatility {
    /// Rolling standard deviation of best bid size.
    pub fn std_bid(&self) -> f64 {
        self.last_std_bid
    }

    /// Rolling standard deviation of best ask size.
    pub fn std_ask(&self) -> f64 {
        self.last_std_ask
    }

    /// max(std_bid, std_ask).
    pub fn max(&self) -> f64 {
        self.last_max
    }
}

impl Default for BestLevelVolatility {
    fn default() -> Self {
        Self::new(20)
    }
}

impl OrderBookConsumer for BestLevelVolatility {
    fn update_orderbook(&mut self, book: &OrderBook) {
        if let Some(b) = book.best_bid() {
            self.bid_sizes.push_back(b.size);
            if self.bid_sizes.len() > self.window {
                self.bid_sizes.pop_front();
            }
        }
        if let Some(a) = book.best_ask() {
            self.ask_sizes.push_back(a.size);
            if self.ask_sizes.len() > self.window {
                self.ask_sizes.pop_front();
            }
        }

        self.last_std_bid = Self::std_dev(&self.bid_sizes);
        self.last_std_ask = Self::std_dev(&self.ask_sizes);
        self.last_max = self.last_std_bid.max(self.last_std_ask);

    }


    fn reset(&mut self) {
        self.bid_sizes.clear();
        self.ask_sizes.clear();
        self.last_std_bid = 0.0;
        self.last_std_ask = 0.0;
        self.last_max = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.bid_sizes.len() >= self.window && self.ask_sizes.len() >= self.window
    }
}

/// Typed configuration for [`BestLevelVolatility`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct BestLevelVolatilityConfig {
    /// Rolling window size (number of order-book snapshots).
    pub window: crate::contract::Param<usize>,
}

impl Indicator for BestLevelVolatility {
    const ID: IndicatorId = IndicatorId::BestLevelVolatility;
    /// L2 order-book indicator — measures top-of-book size volatility.
    const FAMILY: &'static [Family] = &[Family::OrderBook];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderBook];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::magnitude(IndicatorOutputId::BestLevelVolatilityStdBid),
        Output::magnitude(IndicatorOutputId::BestLevelVolatilityStdAsk),
        Output::magnitude(IndicatorOutputId::BestLevelVolatilityMax),
    ];
    /// O(window) per update (std-dev rescans the deque). Two window-depth deques.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[
            Store::window(StoreKind::Deque),
            Store::window(StoreKind::Deque),
        ],
    );
    type Config = BestLevelVolatilityConfig;
    type Runtime = BestLevelVolatility;

    fn create(cfg: BestLevelVolatilityConfig) -> BestLevelVolatility {
        BestLevelVolatility::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for BestLevelVolatilityConfig {
    fn defaults() -> Self {
        BestLevelVolatilityConfig { window: crate::contract::Param::Solo(20) }
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


impl Render for BestLevelVolatility {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::BestLevelVolatilityStdBid, "Std Bid Size", Color::hex(0x2196F3), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::BestLevelVolatilityStdAsk, "Std Ask Size", Color::hex(0xF44336), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::BestLevelVolatilityMax, "Max Std", Color::hex(0xFF9800), 2.0))
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

    fn make_book(bid_size: f64, ask_size: f64) -> OrderBook {
        OrderBook::from_tuples(&[(100.0, bid_size)], &[(101.0, ask_size)], 0)
    }

    #[test]
    fn constant_sizes_give_zero_std() {
        let mut ind = BestLevelVolatility::new(5);
        for _ in 0..5 {
            ind.update_orderbook(&make_book(50.0, 50.0));
        }
        assert!(ind.std_bid().abs() < 1e-10);
        assert!(ind.std_ask().abs() < 1e-10);
        assert!(ind.max().abs() < 1e-10);
    }

    #[test]
    fn varying_bid_gives_nonzero_std() {
        let mut ind = BestLevelVolatility::new(4);
        for size in [10.0_f64, 20.0, 30.0, 40.0] {
            ind.update_orderbook(&make_book(size, 25.0));
        }
        assert!(ind.std_bid() > 0.0);
    }

    #[test]
    fn max_is_larger_of_two_stds() {
        let mut ind = BestLevelVolatility::new(4);
        // bid std small (all same), ask std large
        for size in [10.0_f64, 40.0, 10.0, 40.0] {
            ind.update_orderbook(&make_book(25.0, size));
        }
        assert!(ind.std_ask() > ind.std_bid());
        assert!((ind.max() - ind.std_ask()).abs() < 1e-10);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = BestLevelVolatility::new(3);
        for _ in 0..3 {
            ind.update_orderbook(&make_book(10.0, 20.0));
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.std_bid(), 0.0);
        assert_eq!(ind.std_ask(), 0.0);
        assert_eq!(ind.max(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_best_level_volatility() {
        let mut f = IndicatorOrder::BestLevelVolatility(
            <<BestLevelVolatility as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let book = make_book(10.0, 20.0);
        for _ in 0..5 {
            f.feed(0, MarketSample::OrderBook(&book));
        }
        // After 5 identical updates std should be 0
        assert_eq!(f.primary(), 0.0);
    }
}
