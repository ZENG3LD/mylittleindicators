//! HighLowRangeRatio — intraday volatility proxy from 24-hour high/low range.
//!
//! Computes (high_24h - low_24h) / last_price.
//! A dimensionless ratio: 0 = no range, higher = wider daily swing.
//!
//! Output: `Single(ratio)`.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::ticker_consumer::TickerConsumer;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::core::types::Ticker;
use crate::engine::stream_kind::StreamKind;

/// (high_24h - low_24h) / last_price as a volatility proxy.
#[derive(Debug, Clone)]
pub struct HighLowRangeRatio {
    last_ratio: f64,
    ready: bool,
}

impl HighLowRangeRatio {
    /// Create a new instance. No parameters needed — ratio is instantaneous.
    pub fn new() -> Self {
        Self { last_ratio: 0.0, ready: false }
    }
}

impl Default for HighLowRangeRatio {
    fn default() -> Self {
        Self::new()
    }
}

/// Unit configuration for [`HighLowRangeRatio`] — no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct HighLowRangeRatioConfig;

impl Indicator for HighLowRangeRatio {
    const ID: IndicatorId = IndicatorId::HighLowRangeRatio;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Ticker];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::HighLowRangeRatio)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = HighLowRangeRatioConfig;
    type Runtime = HighLowRangeRatio;

    fn create(_cfg: HighLowRangeRatioConfig) -> HighLowRangeRatio {
        HighLowRangeRatio::new()
    }
}

impl crate::contract::Config for HighLowRangeRatioConfig {
    fn defaults() -> Self {
        HighLowRangeRatioConfig
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


impl Render for HighLowRangeRatio {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::HighLowRangeRatio, "High/Low Range Ratio", Color::hex(0x4CAF50))
            .precision(4)
            .build()
    }
}

impl TickerConsumer for HighLowRangeRatio {
    fn update_ticker(&mut self, ticker: &Ticker) {
        let ratio = match (ticker.high_24h, ticker.low_24h) {
            (Some(h), Some(l)) if ticker.last_price > 0.0 => {
                (h - l) / ticker.last_price
            }
            _ => 0.0,
        };
        self.last_ratio = ratio;
        self.ready = true;
    }


    fn reset(&mut self) {
        self.last_ratio = 0.0;
        self.ready = false;
    }

    fn is_ready(&self) -> bool {
        self.ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticker(last: f64, high: Option<f64>, low: Option<f64>) -> Ticker {
        Ticker {
            last_price: last,
            bid_price: None,
            ask_price: None,
            high_24h: high,
            low_24h: low,
            volume_24h: None,
            quote_volume_24h: None,
            price_change_24h: None,
            price_change_percent_24h: None,
            timestamp: 1000,
            ..Default::default()
        }
    }

    #[test]
    fn not_ready_initially() {
        let ind = HighLowRangeRatio::new();
        assert!(!ind.is_ready());
    }

    #[test]
    fn correct_ratio() {
        let mut ind = HighLowRangeRatio::new();
        // range = 2000, last = 50000 → ratio = 0.04
        ind.update_ticker(&ticker(50000.0, Some(51000.0), Some(49000.0)));
        assert!(ind.is_ready());
        let r = ind.value();
        assert!((r - 0.04).abs() < 1e-12, "expected 0.04, got {}", r);
    }

    #[test]
    fn missing_high_low_returns_zero() {
        let mut ind = HighLowRangeRatio::new();
        ind.update_ticker(&ticker(50000.0, None, None));
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn zero_last_price_returns_zero() {
        let mut ind = HighLowRangeRatio::new();
        ind.update_ticker(&ticker(0.0, Some(100.0), Some(90.0)));
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = HighLowRangeRatio::new();
        ind.update_ticker(&ticker(50000.0, Some(51000.0), Some(49000.0)));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_high_low_range_ratio() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        use crate::core::types::Ticker as TickerType;
        use super::HighLowRangeRatioConfig;

        let mut f = IndicatorOrder::HighLowRangeRatio(HighLowRangeRatioConfig)
            .build_solo()
            .unwrap();
        let t = TickerType {
            last_price: 50000.0,
            bid_price: None,
            ask_price: None,
            high_24h: Some(51000.0),
            low_24h: Some(49000.0),
            volume_24h: Some(9999.0),
            quote_volume_24h: None,
            price_change_24h: None,
            price_change_percent_24h: None,
            timestamp: 0,
            ..Default::default()
        };
        f.feed(0, MarketSample::Ticker(&t));
        // (51000 - 49000) / 50000 = 0.04
        let r = f.primary();
        assert!((r - 0.04).abs() < 1e-12, "expected 0.04, got {r}");
    }
}

impl HighLowRangeRatio {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_ratio
    }
}
