//! TradeBookAbsorption — detects absorption using real order book state.
//!
//! Similar to `AbsorptionDetector` (tick-only) but uses the actual visible
//! top-of-book size to determine whether a trade was absorbed at the best level.
//!
//! Definition of absorption:
//! - The tick executes at exactly the best ask (buy) or best bid (sell) price
//! - The trade size exceeds the visible top-of-book size at that level
//! - Yet price did not move (the level price matched tick price exactly)
//! → Someone had size beyond what was visible (absorption / iceberg).
//!
//! Output: `Triple(side, last_absorbed_vol, cumulative_absorbed_vol)`
//!   - side:                +1.0 = buy absorption at ask, -1.0 = sell absorption at bid, 0.0 = none
//!   - last_absorbed_vol:   absorbed volume on most recent tick
//!   - cumulative:          sum over rolling window

use std::collections::VecDeque;

use crate::engine::streams::hybrid_tick_book_consumer::HybridTickBookConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::{OrderBook, Tick};

/// Detects absorption using real top-of-book size vs actual trade size.
///
/// Absorption fires when the trade exceeds `ratio` × visible top-of-book size.
/// On deep books (e.g. BTC perps with hundreds of contracts on the top level),
/// a single trade rarely consumes the whole visible level, so a ratio (default
/// 0.5 = 50%) catches meaningful liquidity absorption events instead of only
/// the rare "trade > full top" case.
#[derive(Debug, Clone)]
pub struct TradeBookAbsorption {
    rolling_window: usize,
    /// Fraction of visible top size a trade must exceed to count as absorption.
    ratio: f64,
    /// Ring buffer of (absorbed_volume, side) per recent tick.
    events: VecDeque<(f64, i8)>,
    last_absorbed: f64,
    last_side: i8,
}

impl TradeBookAbsorption {
    /// Create with given rolling window size (number of ticks) and default ratio (0.5).
    pub fn new(window: usize) -> Self {
        Self::with_ratio(window, 0.5)
    }

    /// Create with explicit ratio threshold (0.0..=1.0+; >1.0 means trade must
    /// exceed visible top fully, which mirrors the original strict semantics).
    pub fn with_ratio(window: usize, ratio: f64) -> Self {
        let w = window.max(1);
        Self {
            rolling_window: w,
            ratio: ratio.max(0.0),
            events: VecDeque::with_capacity(w),
            last_absorbed: 0.0,
            last_side: 0,
        }
    }
}

impl TradeBookAbsorption {
    #[inline]
    pub fn side(&self) -> f64 {
        self.last_side as f64
    }

    #[inline]
    pub fn last_absorbed_vol(&self) -> f64 {
        self.last_absorbed
    }

    #[inline]
    pub fn cumulative_absorbed_vol(&self) -> f64 {
        self.events.iter().map(|&(v, _)| v).sum()
    }
}

impl Default for TradeBookAbsorption {
    fn default() -> Self {
        Self::with_ratio(50, 0.5)
    }
}

impl HybridTickBookConsumer for TradeBookAbsorption {
    fn update_tick_with_book(&mut self, tick: &Tick, book: &OrderBook) {
        let (target_level, side) = if tick.is_buy {
            (book.best_ask(), 1i8)
        } else {
            (book.best_bid(), -1i8)
        };

        let visible_size = target_level.map(|l| l.size).unwrap_or(0.0);
        let level_price = target_level.map(|l| l.price).unwrap_or(tick.price);

        // Absorption: tick executes at best level price AND consumes >= ratio of
        // visible size — no price movement. On deep books the strict "tick.size >
        // visible_size" path almost never fires; ratio (default 0.5) catches
        // meaningful liquidity events.
        let price_at_level = (tick.price - level_price).abs() < 1e-9;
        let threshold = visible_size * self.ratio;
        let absorbed = if price_at_level && visible_size > 0.0 && tick.size > threshold {
            (tick.size - threshold).max(0.0)
        } else {
            0.0
        };

        self.events
            .push_back((absorbed, if absorbed > 0.0 { side } else { 0 }));
        if self.events.len() > self.rolling_window {
            self.events.pop_front();
        }

        self.last_absorbed = absorbed;
        self.last_side = if absorbed > 0.0 { side } else { 0 };

        let _cumulative: f64 = self.events.iter().map(|&(v, _)| v).sum();
    }

    fn update_book_only(&mut self, _book: &OrderBook) {}


    fn reset(&mut self) {
        self.events.clear();
        self.last_absorbed = 0.0;
        self.last_side = 0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

/// Typed configuration for [`TradeBookAbsorption`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct TradeBookAbsorptionConfig {
    /// Rolling window size (number of ticks).
    pub window: crate::contract::Param<usize>,
    /// Fraction of visible top size a trade must exceed to count as absorption.
    pub ratio: crate::contract::Param<f64>,
}

impl Indicator for TradeBookAbsorption {
    const ID: IndicatorId = IndicatorId::TradeBookAbsorption;
    /// Detects absorption at the best bid/ask using visible top-of-book size.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick, StreamKind::OrderBook];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::TradeBookAbsorptionSide),
        Output::count(IndicatorOutputId::TradeBookAbsorptionLastAbsorbedVol),
        Output::count(IndicatorOutputId::TradeBookAbsorptionCumulativeAbsorbedVol),
    ];
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = TradeBookAbsorptionConfig;
    type Runtime = TradeBookAbsorption;

    fn create(cfg: TradeBookAbsorptionConfig) -> TradeBookAbsorption {
        TradeBookAbsorption::with_ratio(cfg.window.resolved(), cfg.ratio.resolved())
    }
}

impl crate::contract::Config for TradeBookAbsorptionConfig {
    fn defaults() -> Self {
        TradeBookAbsorptionConfig {
            window: crate::contract::Param::Solo(50),
            ratio: crate::contract::Param::Solo(0.5),
        }
    }
    fn machine_defaults() -> Self {
        use crate::contract::{Param, sweep_f64};
        let mut s = Self::machine_defaults_auto();
        // window: Class A (tick window) — auto range(2,4048,1) is correct.
        // ratio: Class D (fraction 0..1, fraction of visible top size) → sweep_f64(0.0,1.0,0.05).
        s.ratio = Param::many(sweep_f64(0.0, 1.0, 0.05));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for TradeBookAbsorption {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::TradeBookAbsorptionSide, "Side", Color::hex(0x9C27B0), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::TradeBookAbsorptionLastAbsorbedVol, "Absorbed", Color::hex(0xFF9800), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::TradeBookAbsorptionCumulativeAbsorbedVol, "Cum. Absorbed", Color::hex(0xF44336), 2.0))
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

    fn book_ask(price: f64, size: f64) -> OrderBook {
        OrderBook::from_tuples(&[(price - 1.0, 10.0)], &[(price, size)], 0)
    }

    fn book_bid(price: f64, size: f64) -> OrderBook {
        OrderBook::from_tuples(&[(price, size)], &[(price + 1.0, 10.0)], 0)
    }

    #[test]
    fn no_absorption_when_tick_below_ratio() {
        // ratio = 0.5, visible = 20 → threshold = 10; tick = 8 → no absorption.
        let mut det = TradeBookAbsorption::new(10);
        let book = book_ask(100.0, 20.0);
        det.update_tick_with_book(&tick(100.0, 8.0, true), &book);
        assert_eq!(det.side(), 0.0);
        assert_eq!(det.last_absorbed_vol(), 0.0);
    }

    #[test]
    fn absorption_detected_when_trade_exceeds_visible_ask() {
        // visible top = 5, ratio = 0.5 → threshold = 2.5, tick = 20 → absorbed = 17.5
        let mut det = TradeBookAbsorption::new(10);
        let book = book_ask(100.0, 5.0);
        det.update_tick_with_book(&tick(100.0, 20.0, true), &book);
        assert!((det.side() - 1.0).abs() < 1e-9, "expected +1 side");
        assert!((det.last_absorbed_vol() - 17.5).abs() < 1e-9, "absorbed should be 17.5");
    }

    #[test]
    fn absorption_detected_on_sell_side() {
        // visible = 3, ratio = 0.5 → threshold = 1.5, tick = 12 → absorbed = 10.5
        let mut det = TradeBookAbsorption::new(10);
        let book = book_bid(100.0, 3.0);
        det.update_tick_with_book(&tick(100.0, 12.0, false), &book);
        assert!((det.side() - (-1.0)).abs() < 1e-9, "expected -1 side");
        assert!((det.last_absorbed_vol() - 10.5).abs() < 1e-9, "absorbed should be 10.5");
    }

    #[test]
    fn strict_ratio_matches_legacy_semantics() {
        // ratio = 1.0 mirrors the old "tick.size > visible_size" check.
        let mut det = TradeBookAbsorption::with_ratio(10, 1.0);
        let book = book_ask(100.0, 5.0);
        det.update_tick_with_book(&tick(100.0, 20.0, true), &book);
        assert!((det.side() - 1.0).abs() < 1e-9);
        assert!((det.last_absorbed_vol() - 15.0).abs() < 1e-9, "ratio 1.0 → absorbed = tick - visible");
    }

    #[test]
    fn no_absorption_when_tick_price_not_at_best_level() {
        let mut det = TradeBookAbsorption::new(10);
        // Best ask at 101, tick at 100 → price_at_level = false
        let book = book_ask(101.0, 2.0);
        det.update_tick_with_book(&tick(100.0, 20.0, true), &book);
        assert_eq!(det.side(), 0.0);
        assert_eq!(det.last_absorbed_vol(), 0.0);
    }

    #[test]
    fn cumulative_tracks_window() {
        // visible = 1, ratio = 0.5 → threshold = 0.5, tick = 6 → absorbed = 5.5
        let mut det = TradeBookAbsorption::new(3);
        let book = book_ask(100.0, 1.0);
        for _ in 0..3 {
            det.update_tick_with_book(&tick(100.0, 6.0, true), &book);
        }
        assert!((det.cumulative_absorbed_vol() - 16.5).abs() < 1e-9, "cum should be 3 × 5.5 = 16.5");
    }

    #[test]
    fn reset_clears_state() {
        let mut det = TradeBookAbsorption::new(5);
        let book = book_ask(100.0, 1.0);
        det.update_tick_with_book(&tick(100.0, 10.0, true), &book);
        assert!(det.is_ready());
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.side(), 0.0);
        assert_eq!(det.last_absorbed_vol(), 0.0);
        assert_eq!(det.cumulative_absorbed_vol(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_trade_book_absorption() {
        
        let mut f = IndicatorOrder::TradeBookAbsorption(
            <<TradeBookAbsorption as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // Feed a book-only sample (smokes update_book_only path).
        let book = book_ask(100.0, 5.0);
        f.feed(0, MarketSample::OrderBook(&book));
        // factory primary = side
        assert_eq!(f.primary(), 0.0);
    }
}
