//! TickerSpreadRatio — normalized bid-ask spread relative to last price.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::ticker_consumer::TickerConsumer;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::core::types::Ticker;
use crate::engine::stream_kind::StreamKind;

/// Normalized bid-ask spread: `(ask - bid) / last_price`.
///
/// Returns 0.0 when bid_price or ask_price is None, or last_price is zero.
///
/// Output: `Single(spread_ratio)`.
#[derive(Debug, Clone, Default)]
pub struct TickerSpreadRatio {
    last_ratio: f64,
}

impl TickerSpreadRatio {
    /// Create a new indicator.
    pub fn new() -> Self {
        Self { last_ratio: 0.0 }
    }
}

/// Unit configuration for [`TickerSpreadRatio`] — no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct TickerSpreadRatioConfig;

impl Indicator for TickerSpreadRatio {
    const ID: IndicatorId = IndicatorId::TickerSpreadRatio;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Ticker];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::TickerSpreadRatio)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = TickerSpreadRatioConfig;
    type Runtime = TickerSpreadRatio;

    fn create(_cfg: TickerSpreadRatioConfig) -> TickerSpreadRatio {
        TickerSpreadRatio::new()
    }
}

impl crate::contract::Config for TickerSpreadRatioConfig {
    fn defaults() -> Self {
        TickerSpreadRatioConfig
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


impl Render for TickerSpreadRatio {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::TickerSpreadRatio, "Ticker Spread Ratio", Color::hex(0x009688))
            .precision(5)
            .build()
    }
}

impl TickerConsumer for TickerSpreadRatio {
    fn update_ticker(&mut self, ticker: &Ticker) {
        self.last_ratio = match (ticker.bid_price, ticker.ask_price) {
            (Some(bid), Some(ask)) if ticker.last_price.abs() > 1e-15 => {
                (ask - bid) / ticker.last_price
            }
            _ => 0.0,
        };
    }


    fn reset(&mut self) {
        self.last_ratio = 0.0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticker(bid: Option<f64>, ask: Option<f64>, last: f64) -> Ticker {
        Ticker {
            last_price: last,
            bid_price: bid,
            ask_price: ask,
            high_24h: None,
            low_24h: None,
            volume_24h: None,
            quote_volume_24h: None,
            price_change_24h: None,
            price_change_percent_24h: None,
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn spread_ratio_computed_correctly() {
        let mut ind = TickerSpreadRatio::new();
        ind.update_ticker(&ticker(Some(99.0), Some(101.0), 100.0));
        // (101 - 99) / 100 = 0.02
        let r = ind.value();
        assert!((r - 0.02).abs() < 1e-12, "spread ratio = {r}");
    }

    #[test]
    fn missing_bid_returns_zero() {
        let mut ind = TickerSpreadRatio::new();
        ind.update_ticker(&ticker(None, Some(101.0), 100.0));
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn missing_ask_returns_zero() {
        let mut ind = TickerSpreadRatio::new();
        ind.update_ticker(&ticker(Some(99.0), None, 100.0));
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn zero_last_price_returns_zero() {
        let mut ind = TickerSpreadRatio::new();
        ind.update_ticker(&ticker(Some(0.0), Some(0.0), 0.0));
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_ticker_spread_ratio() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        use crate::core::types::Ticker as TickerType;
        use super::TickerSpreadRatioConfig;

        let mut f = IndicatorOrder::TickerSpreadRatio(TickerSpreadRatioConfig)
            .build_solo()
            .unwrap();
        let t = TickerType {
            last_price: 100.0,
            bid_price: Some(99.0),
            ask_price: Some(101.0),
            high_24h: Some(9999.0),
            low_24h: Some(9999.0),
            volume_24h: None,
            quote_volume_24h: None,
            price_change_24h: None,
            price_change_percent_24h: None,
            timestamp: 0,
            ..Default::default()
        };
        f.feed(0, MarketSample::Ticker(&t));
        // (101 - 99) / 100 = 0.02
        let r = f.primary();
        assert!((r - 0.02).abs() < 1e-12, "expected 0.02, got {r}");
    }
}

impl TickerSpreadRatio {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_ratio
    }
}
