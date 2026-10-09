//! AuctionImbalance — rolling imbalance of indicative auction quantities.

use std::collections::VecDeque;

use crate::engine::streams::auction_event_consumer::AuctionEventConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Render, SourceAxis, RenderSpec, Color, Store, StoreKind, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::AuctionEvent;

/// Rolling imbalance of auction indicative quantities relative to their rolling average.
///
/// `imbalance = current_qty / rolling_avg_qty`
///
/// Returns 1.0 (neutral) until the window fills. Returns 0.0 before any event.
///
/// Output: `Single(imbalance)`.
#[derive(Debug, Clone)]
pub struct AuctionImbalance {
    window_size: usize,
    qty_history: VecDeque<f64>,
    last_imbalance: f64,
}

impl AuctionImbalance {
    /// Create a new indicator. `window_size` is clamped to at least 1.
    pub fn new(window_size: usize) -> Self {
        let window_size = window_size.max(1);
        Self {
            window_size,
            qty_history: VecDeque::with_capacity(window_size),
            last_imbalance: 0.0,
        }
    }

    fn rolling_avg(&self) -> f64 {
        if self.qty_history.is_empty() {
            return 0.0;
        }
        self.qty_history.iter().sum::<f64>() / self.qty_history.len() as f64
    }
}

impl Default for AuctionImbalance {
    fn default() -> Self {
        Self::new(20)
    }
}

impl AuctionEventConsumer for AuctionImbalance {
    fn update_auction(&mut self, a: &AuctionEvent) {
        self.qty_history.push_back(a.indicative_qty);
        while self.qty_history.len() > self.window_size {
            self.qty_history.pop_front();
        }
        let avg = self.rolling_avg();
        self.last_imbalance = if avg != 0.0 {
            a.indicative_qty / avg
        } else {
            1.0
        };
    }


    fn reset(&mut self) {
        self.qty_history.clear();
        self.last_imbalance = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.qty_history.is_empty()
    }
}

use crate::contract::Param;

/// Typed configuration for [`AuctionImbalance`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AuctionImbalanceConfig {
    /// Rolling window size (minimum 1).
    pub window_size: Param<usize>,
}

impl Indicator for AuctionImbalance {
    const ID: IndicatorId = IndicatorId::AuctionImbalance;
    /// Auction imbalance score — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Auction];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::ratio(IndicatorOutputId::AuctionImbalance)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    type Config = AuctionImbalanceConfig;
    type Runtime = Self;

    fn create(cfg: AuctionImbalanceConfig) -> Self {
        Self::new(cfg.window_size.resolved())
    }
}

impl crate::contract::Config for AuctionImbalanceConfig {
    fn defaults() -> Self {
        AuctionImbalanceConfig { window_size: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // window_size: rolling auction event window — Class A period, auto sweep range(2,4048,1).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for AuctionImbalance {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::AuctionImbalance, "Auction Imbalance", Color::hex(0x00BCD4))
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
    fn single_event_imbalance_is_one() {
        let mut ind = AuctionImbalance::new(5);
        ind.update_auction(&make_auction(100.0));
        // avg of 1 element = 100, imbalance = 100/100 = 1.0
        let v = ind.value();
        assert!((v - 1.0).abs() < 1e-9, "imbalance = {v}");
    }

    #[test]
    fn higher_qty_gives_imbalance_above_one() {
        let mut ind = AuctionImbalance::new(5);
        for _ in 0..5 {
            ind.update_auction(&make_auction(100.0));
        }
        // avg = 100, now spike
        ind.update_auction(&make_auction(200.0));
        let v = ind.value();
        assert!(v > 1.0, "imbalance should be > 1, got {v}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = AuctionImbalance::new(5);
        ind.update_auction(&make_auction(100.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_auction() {
        use crate::engine::contract_engine::IndicatorOrder;
        
        // NOTE: MarketSample::Auction does not exist yet — minimal smoke build only.
        let f = IndicatorOrder::AuctionImbalance(
            <<AuctionImbalance as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // Value is 0.0 before any auction event.
        assert_eq!(f.primary(), 0.0);
    }
}

impl AuctionImbalance {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_imbalance
    }
}
