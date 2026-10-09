//! AuctionLiquidityScore — rolling ratio of current indicative qty to rolling mean.
//!
//! score = current_qty / rolling_mean_qty
//!
//! A score above 1 means more liquidity than usual at the auction.
//! A score below 1 means thinner liquidity.
//!
//! Output: `Single(score)`.

use std::collections::VecDeque;

use crate::engine::streams::auction_event_consumer::AuctionEventConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Render, SourceAxis, RenderSpec, Color, Store, StoreKind, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::AuctionEvent;

/// Rolling liquidity score for auction events.
///
/// `score = current_indicative_qty / rolling_mean_qty`
///
/// Returns 1.0 (neutral) with a single event. Returns 0.0 before any event.
#[derive(Debug, Clone)]
pub struct AuctionLiquidityScore {
    window: usize,
    qty_history: VecDeque<f64>,
    last_score: f64,
}

impl AuctionLiquidityScore {
    /// Create a new indicator. `window` is clamped to at least 1.
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            qty_history: VecDeque::with_capacity(window),
            last_score: 0.0,
        }
    }

    fn rolling_mean(&self) -> f64 {
        if self.qty_history.is_empty() {
            return 0.0;
        }
        self.qty_history.iter().sum::<f64>() / self.qty_history.len() as f64
    }
}

impl Default for AuctionLiquidityScore {
    fn default() -> Self {
        Self::new(20)
    }
}

impl AuctionEventConsumer for AuctionLiquidityScore {
    fn update_auction(&mut self, a: &AuctionEvent) {
        self.qty_history.push_back(a.indicative_qty);
        while self.qty_history.len() > self.window {
            self.qty_history.pop_front();
        }
        let mean = self.rolling_mean();
        self.last_score = if mean > 0.0 {
            a.indicative_qty / mean
        } else {
            1.0
        };
    }


    fn reset(&mut self) {
        self.qty_history.clear();
        self.last_score = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.qty_history.is_empty()
    }
}

use crate::contract::Param;

/// Typed configuration for [`AuctionLiquidityScore`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AuctionLiquidityScoreConfig {
    /// Rolling window size (minimum 1).
    pub window: Param<usize>,
}

impl Indicator for AuctionLiquidityScore {
    const ID: IndicatorId = IndicatorId::AuctionLiquidityScore;
    /// Auction liquidity score — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Auction];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::ratio(IndicatorOutputId::AuctionLiquidityScore)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    type Config = AuctionLiquidityScoreConfig;
    type Runtime = Self;

    fn create(cfg: AuctionLiquidityScoreConfig) -> Self {
        Self::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for AuctionLiquidityScoreConfig {
    fn defaults() -> Self {
        AuctionLiquidityScoreConfig { window: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // window: rolling auction event window — Class A period, auto sweep range(2,4048,1).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for AuctionLiquidityScore {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::AuctionLiquidityScore, "Auction Liquidity Score", Color::hex(0x4CAF50))
            .precision(3)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_auction(qty: f64) -> AuctionEvent {
        AuctionEvent {
            auction_id: "1".to_string(),
            indicative_price: 100.0,
            indicative_qty: qty,
            state: "indicative".to_string(),
            timestamp: 0,
        }
    }

    #[test]
    fn neutral_on_first_event() {
        let mut ind = AuctionLiquidityScore::new(5);
        ind.update_auction(&make_auction(100.0));
        let v = ind.value();
        assert!((v - 1.0).abs() < 1e-9, "first event should be neutral (1.0), got {v}");
    }

    #[test]
    fn spike_gives_score_above_one() {
        let mut ind = AuctionLiquidityScore::new(5);
        for _ in 0..5 {
            ind.update_auction(&make_auction(100.0));
        }
        // Now spike to 300 — rolling mean still mostly 100
        ind.update_auction(&make_auction(300.0));
        let v = ind.value();
        assert!(v > 1.0, "spike should give score > 1.0, got {v}");
    }

    #[test]
    fn low_qty_gives_score_below_one() {
        let mut ind = AuctionLiquidityScore::new(5);
        for _ in 0..5 {
            ind.update_auction(&make_auction(100.0));
        }
        ind.update_auction(&make_auction(10.0));
        let v = ind.value();
        assert!(v < 1.0, "low qty should give score < 1.0, got {v}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = AuctionLiquidityScore::new(5);
        ind.update_auction(&make_auction(100.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_auction() {
        use crate::engine::contract_engine::IndicatorOrder;
        
        // NOTE: MarketSample::Auction does not exist yet — minimal smoke build only.
        let f = IndicatorOrder::AuctionLiquidityScore(
            <<AuctionLiquidityScore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // Value is 0.0 before any auction event.
        assert_eq!(f.primary(), 0.0);
    }
}

impl AuctionLiquidityScore {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_score
    }
}
