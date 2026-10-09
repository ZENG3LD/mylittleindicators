//! PriceVsIndexSpread — spread between mark price and current index price.
//!
//! Both legs arrive as independent event streams.
//! Output: `Triple(mark_price, index_price, spread)` where `spread = mark - index`.
//! Returns `Triple(NAN, NAN, NAN)` until both streams have delivered at least one event.

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::index_price_consumer::IndexPriceConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::core::types::IndexPrice;
use crate::core::types::MarkPrice;
use crate::engine::stream_kind::StreamKind;

/// Tracks the spread between the mark price and the index price.
///
/// Output: `Triple(mark, index, spread)` where `spread = mark - index`.
/// Returns `Triple(NAN, NAN, NAN)` until both streams have delivered at least one event.
#[derive(Debug, Clone)]
pub struct PriceVsIndexSpread {
    last_price: f64,
    last_index: f64,
}

impl PriceVsIndexSpread {
    /// Create a new indicator with no data.
    pub fn new() -> Self {
        Self {
            last_price: f64::NAN,
            last_index: f64::NAN,
        }
    }

    /// Latest mark price.
    pub fn price(&self) -> f64 {
        self.last_price
    }

    /// Latest index price.
    pub fn index(&self) -> f64 {
        self.last_index
    }

    /// Spread: `mark_price - index_price`.
    pub fn spread(&self) -> f64 {
        if self.last_price.is_finite() && self.last_index.is_finite() {
            self.last_price - self.last_index
        } else {
            f64::NAN
        }
    }


    /// True once both mark price and index price have been seen.
    pub fn indicator_is_ready(&self) -> bool {
        self.last_price.is_finite() && self.last_index.is_finite()
    }

    /// Reset all latched state.
    pub fn indicator_reset(&mut self) {
        self.last_price = f64::NAN;
        self.last_index = f64::NAN;
    }
}

impl Default for PriceVsIndexSpread {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkPriceConsumer for PriceVsIndexSpread {
    fn update_mark(&mut self, mp: &MarkPrice) {
        self.last_price = mp.mark_price;
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl IndexPriceConsumer for PriceVsIndexSpread {
    fn update_index_price(&mut self, ip: &IndexPrice) {
        self.last_index = ip.price;
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

/// Typed configuration for [`PriceVsIndexSpread`] — no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct PriceVsIndexSpreadConfig;

impl Indicator for PriceVsIndexSpread {
    const ID: IndicatorId = IndicatorId::PriceVsIndexSpread;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::MarkPrice, StreamKind::IndexPrice];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::PriceVsIndexSpreadPrice),
        Output::price(IndicatorOutputId::PriceVsIndexSpreadIndex),
        Output::centered(IndicatorOutputId::PriceVsIndexSpreadSpread),
    ];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = PriceVsIndexSpreadConfig;
    type Runtime = PriceVsIndexSpread;

    fn create(_cfg: PriceVsIndexSpreadConfig) -> PriceVsIndexSpread {
        PriceVsIndexSpread::new()
    }
}

impl crate::contract::Config for PriceVsIndexSpreadConfig {
    fn defaults() -> Self {
        PriceVsIndexSpreadConfig
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


impl Render for PriceVsIndexSpread {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(
                IndicatorOutputId::PriceVsIndexSpreadPrice,
                "Mark Price",
                Color::hex(0x2196F3),
                1.5,
            ))
            .output(RenderOutput::line(
                IndicatorOutputId::PriceVsIndexSpreadIndex,
                "Index Price",
                Color::hex(0x4CAF50),
                1.5,
            ))
            .output(RenderOutput::line(
                IndicatorOutputId::PriceVsIndexSpreadSpread,
                "Spread",
                Color::hex(0xFF5722),
                1.0,
            ))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_mark(price: f64) -> MarkPrice {
        MarkPrice {
            mark_price: price,
            index_price: None,
            funding_rate: None,
            timestamp: 0,
            ..Default::default()
        }
    }

    fn make_ip(price: f64) -> IndexPrice {
        IndexPrice { price, timestamp: 0, ..Default::default()}
    }

    #[test]
    fn spread_computed_after_both_updates() {
        let mut ind = PriceVsIndexSpread::new();
        ind.update_mark(&make_mark(100.0));
        ind.update_index_price(&make_ip(98.0));
        assert!((ind.price() - 100.0).abs() < 1e-9);
        assert!((ind.index() - 98.0).abs() < 1e-9);
        assert!((ind.spread() - 2.0).abs() < 1e-9);
    }

    #[test]
    fn not_ready_until_both_set() {
        let mut ind = PriceVsIndexSpread::new();
        assert!(!ind.indicator_is_ready());
        ind.update_mark(&make_mark(100.0));
        assert!(!ind.indicator_is_ready());
        ind.update_index_price(&make_ip(98.0));
        assert!(ind.indicator_is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = PriceVsIndexSpread::new();
        ind.update_mark(&make_mark(100.0));
        ind.update_index_price(&make_ip(98.0));
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        assert!(ind.price().is_nan());
        assert!(ind.index().is_nan());
        assert!(ind.spread().is_nan());
    }

    #[test]
    fn factory_feeds_resolved_price_vs_index_spread() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::PriceVsIndexSpread(PriceVsIndexSpreadConfig).build_solo().unwrap();
        let mp = make_mark(50_000.0);
        let ip = make_ip(49_800.0);
        f.feed(0, MarketSample::MarkPrice(&mp));
        f.feed(0, MarketSample::IndexPrice(&ip));
        let p = f.primary();
        assert!((p - 50_000.0).abs() < 1e-9);
    }
}
