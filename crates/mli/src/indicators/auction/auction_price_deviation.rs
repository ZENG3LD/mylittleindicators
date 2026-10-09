//! AuctionPriceDeviation — percentage deviation of indicative auction price from last close.
//!
//! NOTE: The `last_close` field is initialised to 0.0. In the contracted path (Auction
//! stream only), there is no bar-update mechanism. The deviation will be 0 until the
//! exchange-sent `AuctionEvent` carries a price that differs from the unset close; in
//! practice this indicator is useful only when a companion close feed is provided outside
//! the contract engine. FLAG: consider MULTI (Bar + Auction) in a future wave.

use crate::engine::streams::auction_event_consumer::AuctionEventConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Render, SourceAxis, RenderSpec, Color, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::AuctionEvent;

/// Percentage deviation of the auction indicative price from the last bar close.
///
/// `deviation_pct = (indicative_price - last_close) / last_close * 100`
///
/// Returns 0.0 until `last_close` has been set. In the Auction-only contract path,
/// `last_close` starts at 0 and `has_close` remains false, so output is always 0.0
/// unless the consumer calls `set_close` on the inherent API directly. See module
/// note for future MULTI (Bar + Auction) promotion.
///
/// Output: `Single(deviation_pct)`.
#[derive(Debug, Clone)]
pub struct AuctionPriceDeviation {
    last_close: f64,
    last_deviation: f64,
    has_close: bool,
}

impl AuctionPriceDeviation {
    /// Create a new indicator with no prior state.
    pub fn new() -> Self {
        Self {
            last_close: 0.0,
            last_deviation: 0.0,
            has_close: false,
        }
    }

    /// Manually set the reference close price (for consumers outside the contract engine).
    /// In the contracted `Auction` stream path, close is not updated automatically.
    pub fn set_close(&mut self, close: f64) {
        self.last_close = close;
        self.has_close = true;
    }
}

impl Default for AuctionPriceDeviation {
    fn default() -> Self {
        Self::new()
    }
}

impl AuctionEventConsumer for AuctionPriceDeviation {
    fn update_auction(&mut self, a: &AuctionEvent) {
        if self.has_close && self.last_close != 0.0 {
            self.last_deviation = (a.indicative_price - self.last_close) / self.last_close * 100.0;
        }
    }


    fn reset(&mut self) {
        self.last_close = 0.0;
        self.last_deviation = 0.0;
        self.has_close = false;
    }

    fn is_ready(&self) -> bool {
        self.has_close
    }
}

/// Typed configuration for [`AuctionPriceDeviation`]. No parameters — unit struct.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct AuctionPriceDeviationConfig;

impl Indicator for AuctionPriceDeviation {
    const ID: IndicatorId = IndicatorId::AuctionPriceDeviation;
    /// Auction price deviation — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Auction];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::AuctionPriceDeviation)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = AuctionPriceDeviationConfig;
    type Runtime = Self;

    fn create(_cfg: AuctionPriceDeviationConfig) -> Self {
        Self::new()
    }
}

impl crate::contract::Config for AuctionPriceDeviationConfig {
    fn defaults() -> Self {
        AuctionPriceDeviationConfig
    }
    fn machine_defaults() -> Self {
        // No Param fields — unit struct. Auto returns Self.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for AuctionPriceDeviation {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::AuctionPriceDeviation, "Auction Price Deviation %", Color::hex(0xFF5722))
            .zero_baseline()
            .precision(3)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_auction(indicative_price: f64) -> AuctionEvent {
        AuctionEvent {
            auction_id: "1".to_string(),
            indicative_price,
            indicative_qty: 100.0,
            state: "indicative".to_string(),
            timestamp: 0,
        }
    }

    #[test]
    fn deviation_computed_correctly() {
        let mut ind = AuctionPriceDeviation::new();
        ind.set_close(100.0);
        ind.update_auction(&make_auction(105.0));
        let d = ind.value();
        assert!((d - 5.0).abs() < 1e-9, "deviation = {d}");
    }

    #[test]
    fn negative_deviation_when_below_close() {
        let mut ind = AuctionPriceDeviation::new();
        ind.set_close(100.0);
        ind.update_auction(&make_auction(95.0));
        let d = ind.value();
        assert!(d < 0.0, "deviation should be negative, got {d}");
    }

    #[test]
    fn no_close_no_deviation() {
        let mut ind = AuctionPriceDeviation::new();
        ind.update_auction(&make_auction(105.0));
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = AuctionPriceDeviation::new();
        ind.set_close(100.0);
        ind.update_auction(&make_auction(110.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_auction() {
        use crate::engine::contract_engine::IndicatorOrder;
        
        // NOTE: MarketSample::Auction does not exist yet — minimal smoke build only.
        let f = IndicatorOrder::AuctionPriceDeviation(
            <<AuctionPriceDeviation as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // No close set → deviation stays 0.
        assert_eq!(f.primary(), 0.0);
    }
}

impl AuctionPriceDeviation {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_deviation
    }
}
