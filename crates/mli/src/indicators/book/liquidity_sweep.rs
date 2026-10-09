//! LiquiditySweep — detects large orders consuming multiple price levels.
//!
//! Tracks best bid/ask movement between snapshots.
//! A "buy sweep" occurs when best_ask moves up (asks were eaten).
//! A "sell sweep" occurs when best_bid moves down (bids were eaten).
//!
//! Outputs: `direction`, `magnitude` where
//! - `direction` = +1.0 (buy sweep), -1.0 (sell sweep), 0.0 (no sweep)
//! - `magnitude` = price distance swept (always >= 0)

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::order_book_consumer::OrderBookConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::OrderBook;

/// Detects liquidity sweep events from consecutive orderbook snapshots.
#[derive(Clone, Debug)]
pub struct LiquiditySweep {
    prev_best_bid: f64,
    prev_best_ask: f64,
    has_prev: bool,
    last_sweep_direction: i8,   // +1 = buy sweep, -1 = sell sweep, 0 = none
    last_sweep_magnitude: f64,  // price distance swept (>= 0)
}

impl LiquiditySweep {
    /// Create a new LiquiditySweep detector.
    pub fn new() -> Self {
        Self {
            prev_best_bid: 0.0,
            prev_best_ask: 0.0,
            has_prev: false,
            last_sweep_direction: 0,
            last_sweep_magnitude: 0.0,
        }
    }
}

impl LiquiditySweep {
    /// Sweep direction: +1.0 = buy sweep, -1.0 = sell sweep, 0.0 = none.
    pub fn direction(&self) -> f64 {
        self.last_sweep_direction as f64
    }

    /// Price distance swept (always >= 0).
    pub fn magnitude(&self) -> f64 {
        self.last_sweep_magnitude
    }
}

impl Default for LiquiditySweep {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderBookConsumer for LiquiditySweep {
    fn update_orderbook(&mut self, book: &OrderBook) {
        let (curr_bid, curr_ask) = match (book.best_bid(), book.best_ask()) {
            (Some(b), Some(a)) => (b.price, a.price),
            _ => return,
        };

        if !self.has_prev {
            self.prev_best_bid = curr_bid;
            self.prev_best_ask = curr_ask;
            self.has_prev = true;
            return;
        }

        let mut direction = 0i8;
        let mut magnitude = 0.0f64;

        // Buy sweep: best_ask moved up (asks were eaten)
        if curr_ask > self.prev_best_ask {
            direction = 1;
            magnitude = curr_ask - self.prev_best_ask;
        }
        // Sell sweep: best_bid moved down (bids were eaten)
        else if curr_bid < self.prev_best_bid {
            direction = -1;
            magnitude = self.prev_best_bid - curr_bid;
        }

        self.last_sweep_direction = direction;
        self.last_sweep_magnitude = magnitude;
        self.prev_best_bid = curr_bid;
        self.prev_best_ask = curr_ask;

    }


    fn reset(&mut self) {
        self.has_prev = false;
        self.last_sweep_direction = 0;
        self.last_sweep_magnitude = 0.0;
        self.prev_best_bid = 0.0;
        self.prev_best_ask = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.has_prev
    }
}

/// Typed configuration for [`LiquiditySweep`]. No parameters — pure snapshot diff.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LiquiditySweepConfig;

impl Indicator for LiquiditySweep {
    const ID: IndicatorId = IndicatorId::LiquiditySweep;
    /// L2 order-book liquidity sweep detector.
    const FAMILY: &'static [Family] = &[Family::OrderBook];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderBook];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::LiquiditySweepDirection),
        Output::magnitude(IndicatorOutputId::LiquiditySweepMagnitude),
    ];
    /// O(1) — scalar state only.
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::fixed(StoreKind::Scalar, 4)],
    );
    type Config = LiquiditySweepConfig;
    type Runtime = LiquiditySweep;

    fn create(_cfg: LiquiditySweepConfig) -> LiquiditySweep {
        LiquiditySweep::new()
    }
}

impl crate::contract::Config for LiquiditySweepConfig {
    fn defaults() -> Self {
        LiquiditySweepConfig
    }
    fn machine_defaults() -> Self {
        // No Param fields — unit struct, nothing to sweep.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for LiquiditySweep {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::LiquiditySweepDirection, "Sweep Direction", Color::hex(0xFF9800), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::LiquiditySweepMagnitude, "Sweep Magnitude", Color::hex(0x9C27B0), 1.5))
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

    fn make_book(bids: &[(f64, f64)], asks: &[(f64, f64)]) -> OrderBook {
        OrderBook {
            bids: bids.iter().map(|&(p, s)| OrderBookLevel::new(p, s)).collect(),
            asks: asks.iter().map(|&(p, s)| OrderBookLevel::new(p, s)).collect(),
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn new_not_ready() {
        let ls = LiquiditySweep::new();
        assert!(!ls.is_ready());
        assert_eq!(ls.direction(), 0.0);
        assert_eq!(ls.magnitude(), 0.0);
    }

    #[test]
    fn first_update_initializes_and_no_sweep() {
        let mut ls = LiquiditySweep::new();
        let book = make_book(&[(100.0, 10.0)], &[(101.0, 10.0)]);
        ls.update_orderbook(&book);
        assert!(ls.is_ready());
        // First update sets baseline, direction stays 0
        assert_eq!(ls.direction(), 0.0);
        assert_eq!(ls.magnitude(), 0.0);
    }

    #[test]
    fn buy_sweep_detected() {
        let mut ls = LiquiditySweep::new();
        // First snapshot: bid=95, ask=100
        let book1 = make_book(&[(95.0, 10.0)], &[(100.0, 10.0)]);
        ls.update_orderbook(&book1);
        // Second snapshot: ask moved up to 105 (buy sweep)
        let book2 = make_book(&[(95.0, 10.0)], &[(105.0, 10.0)]);
        ls.update_orderbook(&book2);
        assert!((ls.direction() - 1.0).abs() < 1e-10);
        assert!((ls.magnitude() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn sell_sweep_detected() {
        let mut ls = LiquiditySweep::new();
        let book1 = make_book(&[(95.0, 10.0)], &[(100.0, 10.0)]);
        ls.update_orderbook(&book1);
        let book2 = make_book(&[(90.0, 10.0)], &[(100.0, 10.0)]);
        ls.update_orderbook(&book2);
        assert!((ls.direction() - (-1.0)).abs() < 1e-10);
        assert!((ls.magnitude() - 5.0).abs() < 1e-10);
    }

    #[test]
    fn no_sweep_when_stable() {
        let mut ls = LiquiditySweep::new();
        let book = make_book(&[(95.0, 10.0)], &[(100.0, 10.0)]);
        ls.update_orderbook(&book);
        ls.update_orderbook(&book);
        assert_eq!(ls.direction(), 0.0);
        assert_eq!(ls.magnitude(), 0.0);
    }

    #[test]
    fn reset_clears_state() {
        let mut ls = LiquiditySweep::new();
        let book = make_book(&[(95.0, 10.0)], &[(100.0, 10.0)]);
        ls.update_orderbook(&book);
        assert!(ls.is_ready());
        ls.reset();
        assert!(!ls.is_ready());
        assert_eq!(ls.direction(), 0.0);
        assert_eq!(ls.magnitude(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_liquidity_sweep() {
        let mut f = IndicatorOrder::LiquiditySweep(
            <<LiquiditySweep as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let book = make_book(&[(95.0, 10.0)], &[(100.0, 10.0)]);
        f.feed(0, MarketSample::OrderBook(&book));
        f.feed(0, MarketSample::OrderBook(&book));
        // Stable book → no sweep → direction=0
        assert_eq!(f.primary(), 0.0);
    }
}
