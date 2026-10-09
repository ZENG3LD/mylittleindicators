//! IvSkew — implied volatility skew from bid/ask IV spread.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::option_greeks_consumer::OptionGreeksConsumer;
use crate::contract::{Family, Indicator, Output, SourceAxis};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::OptionGreeks;

/// Computes the implied volatility skew as `bid_iv - ask_iv`.
///
/// Returns 0.0 when either `bid_iv` or `ask_iv` is `None`.
///
/// Output: `Single(skew)`.
#[derive(Clone, Debug)]
pub struct IvSkew {
    last_skew: f64,
}

impl IvSkew {
    /// Create a new indicator.
    pub fn new() -> Self {
        Self { last_skew: 0.0 }
    }
}

impl Default for IvSkew {
    fn default() -> Self {
        Self::new()
    }
}

impl OptionGreeksConsumer for IvSkew {
    fn update_option_greeks(&mut self, g: &OptionGreeks) {
        self.last_skew = match (g.bid_iv, g.ask_iv) {
            (Some(bid), Some(ask)) => bid - ask,
            _ => 0.0,
        };
    }


    fn reset(&mut self) {
        self.last_skew = 0.0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

/// Unit config for [`IvSkew`] — no parameters.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct IvSkewConfig;

impl Indicator for IvSkew {
    const ID: IndicatorId = IndicatorId::IvSkew;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::OptionGreeks];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::IvSkew)];
    type Config = IvSkewConfig;
    type Runtime = IvSkew;

    fn create(_cfg: IvSkewConfig) -> IvSkew {
        IvSkew::new()
    }
}

impl crate::contract::Config for IvSkewConfig {
    fn defaults() -> Self {
        IvSkewConfig
    }
    fn machine_defaults() -> Self {
        // No Param fields — no axes to sweep.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for IvSkew {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::IvSkew, "IV Skew", Color::hex(0xFF7043))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_greeks(bid_iv: Option<f64>, ask_iv: Option<f64>) -> OptionGreeks {
        OptionGreeks {
            delta: 0.0,
            gamma: 0.0,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
            mark_iv: 0.5,
            bid_iv,
            ask_iv,
            timestamp: 0,
        }
    }

    #[test]
    fn positive_skew_when_bid_above_ask() {
        let mut ind = IvSkew::new();
        ind.update_option_greeks(&make_greeks(Some(0.8), Some(0.6)));
        let s = ind.value();
        assert!((s - 0.2).abs() < 1e-9, "skew should be 0.2, got {s}");
    }

    #[test]
    fn negative_skew_when_ask_above_bid() {
        let mut ind = IvSkew::new();
        ind.update_option_greeks(&make_greeks(Some(0.5), Some(0.7)));
        let s = ind.value();
        assert!((s - (-0.2)).abs() < 1e-9, "skew should be -0.2, got {s}");
    }

    #[test]
    fn zero_when_iv_missing() {
        let mut ind = IvSkew::new();
        ind.update_option_greeks(&make_greeks(None, None));
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn reset_clears() {
        let mut ind = IvSkew::new();
        ind.update_option_greeks(&make_greeks(Some(0.8), Some(0.6)));
        ind.reset();
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_iv_skew() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::IvSkew(IvSkewConfig).build_solo().unwrap();
        // bid_iv=0.8, ask_iv=0.6 → skew = 0.2. gamma=9999.0 to prove only iv fields matter.
        let g = OptionGreeks { delta: 0.0, gamma: 9999.0, vega: 0.0, theta: 0.0, rho: 0.0,
            mark_iv: 0.5, bid_iv: Some(0.8), ask_iv: Some(0.6), timestamp: 0 };
        f.feed(0, MarketSample::OptionGreeks(&g));
        assert!((f.primary() - 0.2).abs() < 1e-9, "skew should be 0.2, got {}", f.primary());
    }
}

impl IvSkew {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_skew
    }
}
