//! HiddenLiquidityDetector — detects hidden liquidity by comparing trade size
//! against visible order book size at the traded price level.
//!
//! Algorithm (trade-vs-book mismatch):
//! - Tick on price=X with size=Q
//! - Look up visible size at price=X in book (within `price_bucket` tolerance)
//! - If Q > visible_size → hidden = Q - visible_size
//!
//! Output: `Triple(side, last_hidden_vol, cumulative_hidden_vol)`
//!   - side:                +1.0 = buy aggressor hit hidden ask, -1.0 = sell hit hidden bid, 0.0 = no hidden
//!   - last_hidden_vol:     hidden volume on the most recent tick
//!   - cumulative_hidden:   sum of hidden volume over rolling window

use std::collections::VecDeque;

use crate::engine::streams::hybrid_tick_book_consumer::HybridTickBookConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::{OrderBook, Tick};

/// Detects hidden liquidity (iceberg orders) via trade-vs-visible-book mismatch.
#[derive(Debug, Clone)]
pub struct HiddenLiquidityDetector {
    /// Price bucket size for level matching. Two prices are in the same bucket
    /// when `floor(|p1 - p2| / price_bucket) < 1`.
    price_bucket: f64,
    /// Number of most-recent ticks to include in cumulative window.
    rolling_window: usize,
    /// Ring buffer of hidden volume per recent tick.
    hidden_history: VecDeque<f64>,
    last_hidden_vol: f64,
    /// +1 = buy aggressor hit hidden ask, -1 = sell aggressor hit hidden bid, 0 = none
    last_hidden_side: i8,
    cumulative_hidden: f64,
}

impl HiddenLiquidityDetector {
    /// Create with the given price bucket tolerance and rolling window size.
    pub fn new(price_bucket: f64, window: usize) -> Self {
        let w = window.max(1);
        Self {
            price_bucket: price_bucket.max(1e-12),
            rolling_window: w,
            hidden_history: VecDeque::with_capacity(w),
            last_hidden_vol: 0.0,
            last_hidden_side: 0,
            cumulative_hidden: 0.0,
        }
    }
}

impl HiddenLiquidityDetector {
    #[inline]
    pub fn side(&self) -> f64 {
        self.last_hidden_side as f64
    }

    #[inline]
    pub fn last_hidden_vol(&self) -> f64 {
        self.last_hidden_vol
    }

    #[inline]
    pub fn cumulative_hidden_vol(&self) -> f64 {
        self.cumulative_hidden
    }
}

impl Default for HiddenLiquidityDetector {
    fn default() -> Self {
        Self::new(1.0, 50)
    }
}

impl HybridTickBookConsumer for HiddenLiquidityDetector {
    fn update_tick_with_book(&mut self, tick: &Tick, book: &OrderBook) {
        // Buys aggress the ask side; sells aggress the bid side.
        let target_levels = if tick.is_buy { &book.asks } else { &book.bids };

        let visible_size: f64 = target_levels
            .iter()
            .filter(|l| {
                (l.price - tick.price).abs() / self.price_bucket < 1.0
            })
            .map(|l| l.size)
            .sum();

        let hidden = (tick.size - visible_size).max(0.0);

        self.hidden_history.push_back(hidden);
        if self.hidden_history.len() > self.rolling_window {
            self.hidden_history.pop_front();
        }

        self.last_hidden_vol = hidden;
        self.last_hidden_side = if hidden > 0.0 {
            if tick.is_buy { 1 } else { -1 }
        } else {
            0
        };
        self.cumulative_hidden = self.hidden_history.iter().sum();

    }

    fn update_book_only(&mut self, _book: &OrderBook) {
        // No-op — state updates only on trades.
    }


    fn reset(&mut self) {
        self.hidden_history.clear();
        self.last_hidden_vol = 0.0;
        self.last_hidden_side = 0;
        self.cumulative_hidden = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.hidden_history.is_empty()
    }
}

/// Typed configuration for [`HiddenLiquidityDetector`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct HiddenLiquidityDetectorConfig {
    /// Price bucket size for level matching.
    pub price_bucket: crate::contract::Param<f64>,
    /// Rolling window size (number of ticks).
    pub window: crate::contract::Param<usize>,
}

impl Indicator for HiddenLiquidityDetector {
    const ID: IndicatorId = IndicatorId::HiddenLiquidityDetector;
    /// Detects hidden liquidity via trade-vs-visible-book mismatch.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick, StreamKind::OrderBook];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::HiddenLiquidityDetectorSide),
        Output::count(IndicatorOutputId::HiddenLiquidityDetectorLastHiddenVol),
        Output::count(IndicatorOutputId::HiddenLiquidityDetectorCumulativeHiddenVol),
    ];
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = HiddenLiquidityDetectorConfig;
    type Runtime = HiddenLiquidityDetector;

    fn create(cfg: HiddenLiquidityDetectorConfig) -> HiddenLiquidityDetector {
        HiddenLiquidityDetector::new(cfg.price_bucket.resolved(), cfg.window.resolved())
    }
}

impl crate::contract::Config for HiddenLiquidityDetectorConfig {
    fn defaults() -> Self {
        HiddenLiquidityDetectorConfig {
            price_bucket: crate::contract::Param::Solo(1.0),
            window: crate::contract::Param::Solo(50),
        }
    }
    fn machine_defaults() -> Self {
        // price_bucket: PIN (Class I, instrument-relative) — f64 auto leaves Solo; confirm untouched.
        // window: Class A (tick window) — auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for HiddenLiquidityDetector {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::HiddenLiquidityDetectorSide, "Side", Color::hex(0x9C27B0), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::HiddenLiquidityDetectorLastHiddenVol, "Hidden Vol", Color::hex(0xFF9800), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::HiddenLiquidityDetectorCumulativeHiddenVol, "Cum. Hidden", Color::hex(0xF44336), 2.0))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::market_sample::MarketSample;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::core::types::{OrderBook, Tick};

    fn tick(price: f64, size: f64, is_buy: bool) -> Tick {
        Tick::new(0, price, size, is_buy)
    }

    fn book_with_ask(price: f64, size: f64) -> OrderBook {
        OrderBook::from_tuples(
            &[(price - 1.0, 10.0)],   // bid
            &[(price, size)],          // ask at exact price
            0,
        )
    }

    fn book_with_bid(price: f64, size: f64) -> OrderBook {
        OrderBook::from_tuples(
            &[(price, size)],          // bid at exact price
            &[(price + 1.0, 10.0)],   // ask
            0,
        )
    }

    #[test]
    fn no_hidden_when_trade_fits_visible_ask() {
        let mut det = HiddenLiquidityDetector::new(1.0, 10);
        let book = book_with_ask(100.0, 20.0); // visible ask = 20
        det.update_tick_with_book(&tick(100.0, 15.0, true), &book);
        // 15 <= 20 → no hidden
        assert_eq!(det.side(), 0.0);
        assert_eq!(det.last_hidden_vol(), 0.0);
    }

    #[test]
    fn detects_hidden_ask_liquidity_on_buy() {
        let mut det = HiddenLiquidityDetector::new(1.0, 10);
        let book = book_with_ask(100.0, 5.0); // visible ask = 5
        det.update_tick_with_book(&tick(100.0, 20.0, true), &book);
        // 20 > 5 → hidden = 15, side = +1
        assert!((det.side() - 1.0).abs() < 1e-9);
        assert!((det.last_hidden_vol() - 15.0).abs() < 1e-9);
    }

    #[test]
    fn detects_hidden_bid_liquidity_on_sell() {
        let mut det = HiddenLiquidityDetector::new(1.0, 10);
        let book = book_with_bid(100.0, 3.0); // visible bid = 3
        det.update_tick_with_book(&tick(100.0, 10.0, false), &book);
        // 10 > 3 → hidden = 7, side = -1
        assert!((det.side() - (-1.0)).abs() < 1e-9);
        assert!((det.last_hidden_vol() - 7.0).abs() < 1e-9);
    }

    #[test]
    fn cumulative_accumulates_over_window() {
        let mut det = HiddenLiquidityDetector::new(1.0, 3);
        let book = book_with_ask(100.0, 1.0);
        // Each tick: size=6, visible=1 → hidden=5
        for _ in 0..3 {
            det.update_tick_with_book(&tick(100.0, 6.0, true), &book);
        }
        assert!((det.cumulative_hidden_vol() - 15.0).abs() < 1e-9);
    }

    #[test]
    fn window_evicts_old_ticks() {
        let mut det = HiddenLiquidityDetector::new(1.0, 2);
        let book_hidden = book_with_ask(100.0, 1.0);
        let book_no_hidden = book_with_ask(100.0, 100.0);
        // 2 ticks with hidden=5 each
        det.update_tick_with_book(&tick(100.0, 6.0, true), &book_hidden);
        det.update_tick_with_book(&tick(100.0, 6.0, true), &book_hidden);
        // Now 2 more ticks with no hidden → evicts the old ones
        det.update_tick_with_book(&tick(100.0, 1.0, true), &book_no_hidden);
        det.update_tick_with_book(&tick(100.0, 1.0, true), &book_no_hidden);
        assert!((det.cumulative_hidden_vol() - 0.0).abs() < 1e-9, "cum should be 0 after window eviction: {}", det.cumulative_hidden_vol());
    }

    #[test]
    fn reset_clears_state() {
        let mut det = HiddenLiquidityDetector::new(1.0, 5);
        let book = book_with_ask(100.0, 1.0);
        det.update_tick_with_book(&tick(100.0, 10.0, true), &book);
        assert!(det.is_ready());
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.side(), 0.0);
        assert_eq!(det.last_hidden_vol(), 0.0);
        assert_eq!(det.cumulative_hidden_vol(), 0.0);
    }

    #[test]
    fn not_ready_until_first_tick() {
        let det = HiddenLiquidityDetector::new(1.0, 5);
        assert!(!det.is_ready());
    }

    #[test]
    fn factory_feeds_resolved_hidden_liquidity_detector() {
        
        let mut f = IndicatorOrder::HiddenLiquidityDetector(
            <<HiddenLiquidityDetector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // Feed a book-only sample (smokes update_book_only path).
        let book = book_with_ask(100.0, 5.0);
        f.feed(0, MarketSample::OrderBook(&book));
        // After a book-only update the indicator is not yet ready (no tick fired).
        assert_eq!(f.primary(), 0.0);
    }
}
