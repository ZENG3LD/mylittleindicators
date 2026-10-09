//! SettlementVsMarkSpread — spread between settlement price and mark price.

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
use crate::engine::streams::SettlementEventConsumer;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::MarkPrice;
use crate::core::types::SettlementEvent;

/// Spread between the latest settlement price and the latest mark price.
///
/// spread = settlement_price − last_mark_price
///
/// Output: `Triple(settlement, last_mark, spread)`.
/// Returns `Triple(0, 0, 0)` until both streams have delivered at least one event.
#[derive(Debug, Clone)]
pub struct SettlementVsMarkSpread {
    last_settlement: f64,
    last_mark: f64,
    has_settlement: bool,
    has_mark: bool,
}

impl SettlementVsMarkSpread {
    /// Create a new indicator with no prior state.
    pub fn new() -> Self {
        Self {
            last_settlement: 0.0,
            last_mark: 0.0,
            has_settlement: false,
            has_mark: false,
        }
    }

    /// Latest settlement price.
    pub fn settlement(&self) -> f64 {
        self.last_settlement
    }

    /// Latest mark price.
    pub fn mark(&self) -> f64 {
        self.last_mark
    }

    /// Spread: `settlement_price - last_mark_price`.
    pub fn spread(&self) -> f64 {
        self.last_settlement - self.last_mark
    }


    /// Check readiness: both streams must have provided at least one event.
    pub fn indicator_is_ready(&self) -> bool {
        self.has_settlement && self.has_mark
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.last_settlement = 0.0;
        self.last_mark = 0.0;
        self.has_settlement = false;
        self.has_mark = false;
    }
}

impl Default for SettlementVsMarkSpread {
    fn default() -> Self {
        Self::new()
    }
}

impl SettlementEventConsumer for SettlementVsMarkSpread {
    fn update_settlement(&mut self, s: &SettlementEvent) {
        self.last_settlement = s.settlement_price;
        self.has_settlement = true;
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl MarkPriceConsumer for SettlementVsMarkSpread {
    fn update_mark(&mut self, mp: &MarkPrice) {
        self.last_mark = mp.mark_price;
        self.has_mark = true;
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

/// Typed configuration for [`SettlementVsMarkSpread`] — no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct SettlementVsMarkSpreadConfig;

impl Indicator for SettlementVsMarkSpread {
    const ID: IndicatorId = IndicatorId::SettlementVsMarkSpread;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Settlement, StreamKind::MarkPrice];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::SettlementVsMarkSpreadSettlement),
        Output::price(IndicatorOutputId::SettlementVsMarkSpreadMark),
        Output::centered(IndicatorOutputId::SettlementVsMarkSpreadSpread),
    ];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = SettlementVsMarkSpreadConfig;
    type Runtime = SettlementVsMarkSpread;

    fn create(_cfg: SettlementVsMarkSpreadConfig) -> SettlementVsMarkSpread {
        SettlementVsMarkSpread::new()
    }
}

impl crate::contract::Config for SettlementVsMarkSpreadConfig {
    fn defaults() -> Self {
        SettlementVsMarkSpreadConfig
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // Unit config — no Param fields. Auto is the full implementation.
        Self::machine_defaults_auto()
    }
}


impl Render for SettlementVsMarkSpread {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::SettlementVsMarkSpreadSettlement, "Settlement Price", Color::hex(0x4CAF50), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::SettlementVsMarkSpreadMark, "Mark Price", Color::hex(0x2196F3), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::SettlementVsMarkSpreadSpread, "Spread", Color::hex(0xFF5722), 1.0))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_settlement(price: f64) -> SettlementEvent {
        SettlementEvent {
            settlement_price: price,
            settlement_time: 0,
            timestamp: 0,
            ..Default::default()
        }
    }

    fn make_mark(price: f64) -> MarkPrice {
        MarkPrice {
            mark_price: price,
            index_price: None,
            funding_rate: None,
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn spread_correct() {
        let mut ind = SettlementVsMarkSpread::new();
        ind.update_mark(&make_mark(29_000.0));
        ind.update_settlement(&make_settlement(30_000.0));
        assert_eq!(ind.settlement(), 30_000.0);
        assert_eq!(ind.mark(), 29_000.0);
        let spread = ind.spread();
        assert!((spread - 1000.0).abs() < 1e-9, "spread = {spread}");
    }

    #[test]
    fn not_ready_before_both_streams() {
        let mut ind = SettlementVsMarkSpread::new();
        assert!(!ind.indicator_is_ready());
        ind.update_mark(&make_mark(29_000.0));
        assert!(!ind.indicator_is_ready());
        ind.update_settlement(&make_settlement(30_000.0));
        assert!(ind.indicator_is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = SettlementVsMarkSpread::new();
        ind.update_mark(&make_mark(100.0));
        ind.update_settlement(&make_settlement(110.0));
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        assert_eq!(ind.settlement(), 0.0);
        assert_eq!(ind.mark(), 0.0);
        assert_eq!(ind.spread(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_settlement_vs_mark_spread() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::SettlementVsMarkSpread(SettlementVsMarkSpreadConfig).build_solo().unwrap();
        let mp = make_mark(29_000.0);
        let se = make_settlement(30_000.0);
        f.feed(0, MarketSample::MarkPrice(&mp));
        f.feed(0, MarketSample::Settlement(&se));
        // f.value() = first output (settlement price)
        let s = f.primary();
        assert_eq!(s, 30_000.0, "settlement={s}");
    }
}
