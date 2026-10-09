//! MarkPriceVsLast — deviation of mark price from the last traded price.
//!
//! Measures the premium or discount between the exchange mark price and
//! the last traded price. Useful for detecting funding arbitrage setups.
//!
//! Output: `Double(deviation, deviation_pct)`
//!   - deviation:      mark_price - last_traded_price (absolute)
//!   - deviation_pct:  deviation / last_traded_price * 100 (percentage)
//!
//! `is_ready` once at least one mark price and one traded price have been seen.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
use crate::engine::streams::ticker_consumer::TickerConsumer;
use crate::contract::{Family, Indicator, Output, SourceAxis};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::MarkPrice;
use crate::core::types::Ticker;

/// Tracks deviation between mark price and last traded price.
#[derive(Debug, Clone)]
pub struct MarkPriceVsLast {
    last_mark: f64,
    last_traded: f64,
    has_mark: bool,
    has_traded: bool,
}

impl MarkPriceVsLast {
    pub fn new() -> Self {
        Self {
            last_mark: 0.0,
            last_traded: 0.0,
            has_mark: false,
            has_traded: false,
        }
    }

    /// Absolute deviation: `mark_price - last_traded_price`.
    pub fn deviation(&self) -> f64 {
        if !self.has_mark || !self.has_traded || self.last_traded < 1e-12 {
            return 0.0;
        }
        self.last_mark - self.last_traded
    }

    /// Percentage deviation: `(mark_price - last_traded_price) / last_traded_price * 100`.
    pub fn deviation_pct(&self) -> f64 {
        if !self.has_mark || !self.has_traded || self.last_traded < 1e-12 {
            return 0.0;
        }
        (self.last_mark - self.last_traded) / self.last_traded * 100.0
    }

    /// Update the last traded price (call with each new trade or bar close).
    pub fn set_last_traded(&mut self, price: f64) {
        if price > 0.0 {
            self.last_traded = price;
            self.has_traded = true;
        }
    }



    /// Inherent readiness the factory reads (both streams seen).
    pub fn indicator_is_ready(&self) -> bool {
        self.has_mark && self.has_traded
    }

    /// Inherent reset the factory calls.
    pub fn indicator_reset(&mut self) {
        self.last_mark = 0.0;
        self.last_traded = 0.0;
        self.has_mark = false;
        self.has_traded = false;
    }
}

impl Default for MarkPriceVsLast {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed configuration for [`MarkPriceVsLast`]. No parameters — unit config.
///
/// `is_ready` requires both a mark price and a last-traded price. The factory
/// routes `MarkPrice` -> [`MarkPriceConsumer`] and `Ticker` -> [`TickerConsumer`]
/// (last_price), so the indicator reaches readiness through the contract alone.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct MarkPriceVsLastConfig;

impl Indicator for MarkPriceVsLast {
    const ID: IndicatorId = IndicatorId::MarkPriceVsLast;
    /// Mark-price indicator — not a pluggable family.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::MarkPrice, StreamKind::Ticker];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::MarkPriceVsLastDeviation),
        Output::centered(IndicatorOutputId::MarkPriceVsLastDeviationPct),
    ];
    type Config = MarkPriceVsLastConfig;
    type Runtime = MarkPriceVsLast;

    fn create(_cfg: MarkPriceVsLastConfig) -> MarkPriceVsLast {
        MarkPriceVsLast::new()
    }
}

impl crate::contract::Config for MarkPriceVsLastConfig {
    fn defaults() -> Self {
        MarkPriceVsLastConfig
    }
    fn machine_defaults() -> Self {
        // Unit config — no Param fields. Auto is the full implementation.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for MarkPriceVsLast {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::MarkPriceVsLastDeviation, "Mark−Last (abs)", Color::hex(0x26C6DA))
            .line_output(IndicatorOutputId::MarkPriceVsLastDeviationPct, "Mark−Last (%)", Color::hex(0xFF7043))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

impl MarkPriceConsumer for MarkPriceVsLast {
    fn update_mark(&mut self, mp: &MarkPrice) {
        self.last_mark = mp.mark_price;
        self.has_mark = true;
    }


    fn reset(&mut self) {
        self.last_mark = 0.0;
        self.last_traded = 0.0;
        self.has_mark = false;
        self.has_traded = false;
    }

    fn is_ready(&self) -> bool {
        self.has_mark && self.has_traded
    }
}

impl TickerConsumer for MarkPriceVsLast {
    fn update_ticker(&mut self, t: &Ticker) {
        self.set_last_traded(t.last_price);
    }


    fn reset(&mut self) {
        self.last_mark = 0.0;
        self.last_traded = 0.0;
        self.has_mark = false;
        self.has_traded = false;
    }

    fn is_ready(&self) -> bool {
        self.has_mark && self.has_traded
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_mark_price() {
        let mut f = IndicatorOrder::MarkPriceVsLast(<<MarkPriceVsLast as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        let sample = MarkPrice {
            mark_price: 50_100.0,
            index_price: None,
            funding_rate: None,
            timestamp: 1,
            ..Default::default()
        };
        f.feed(0, MarketSample::MarkPrice(&sample));
        // Without set_last_traded the output is 0.0 (not ready yet); f.value() = first output (deviation)
        assert_eq!(f.primary(), 0.0);
    }

    fn mp(mark: f64) -> MarkPrice {
        MarkPrice {
            mark_price: mark,
            index_price: None,
            funding_rate: None,
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn not_ready_without_traded_price() {
        let mut ind = MarkPriceVsLast::new();
        ind.update_mark(&mp(50_000.0));
        assert!(!ind.indicator_is_ready(), "needs traded price too");
    }

    #[test]
    fn not_ready_without_mark_price() {
        let mut ind = MarkPriceVsLast::new();
        ind.set_last_traded(50_000.0);
        assert!(!ind.indicator_is_ready(), "needs mark price too");
    }

    #[test]
    fn computes_deviation() {
        let mut ind = MarkPriceVsLast::new();
        ind.set_last_traded(50_000.0);
        ind.update_mark(&mp(50_100.0));
        assert!(ind.indicator_is_ready());
        let dev = ind.deviation();
        let dev_pct = ind.deviation_pct();
        assert!((dev - 100.0).abs() < 1e-9);
        assert!((dev_pct - 0.2).abs() < 1e-6);
    }

    #[test]
    fn negative_deviation_mark_below_last() {
        let mut ind = MarkPriceVsLast::new();
        ind.set_last_traded(50_000.0);
        ind.update_mark(&mp(49_900.0));
        let dev = ind.deviation();
        assert!(dev < 0.0, "mark below last → negative deviation");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = MarkPriceVsLast::new();
        ind.set_last_traded(50_000.0);
        ind.update_mark(&mp(50_100.0));
        assert!(ind.indicator_is_ready());
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
    }
}
