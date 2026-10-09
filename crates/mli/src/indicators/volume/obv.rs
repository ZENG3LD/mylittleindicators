//! On-Balance Volume (OBV) indicator.

use crate::engine::ohlcv_field::OhlcvField;

/// On-Balance Volume (OBV) - cumulative volume-based momentum indicator.
///
/// OBV = Previous OBV + Volume (if close > previous close)
/// OBV = Previous OBV - Volume (if close < previous close)
/// OBV = Previous OBV (if close = previous close)
///
/// Developed by Joseph Granville. OBV shows the cumulative buying and selling
/// pressure by adding volume on up days and subtracting on down days.
///
/// Interpretation:
/// - Rising OBV: Accumulation (buying pressure)
/// - Falling OBV: Distribution (selling pressure)
/// - OBV divergence with price: Potential trend reversal
/// - OBV trend confirmation: Validates price trends
///
/// # Implementation
///
/// Cumulative calculation. O(1) per update.
#[derive(Debug, Clone)]
pub struct Obv {
    value: f64,
    prev_close: f64,
    ready: bool,
    count: usize,
}

impl Obv {
    /// Creates a new OBV indicator.
    pub fn new() -> Self {
        Self {
            value: 0.0,
            prev_close: 0.0,
            ready: false,
            count: 0,
        }
    }

    /// Feed the resolved input lanes — `[price, volume]`. The factory resolves the
    /// configured price source (lane 0) + volume (lane 1); the core gates the cumulative
    /// volume by the price tick. Knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let price = lanes[0];
        let volume = lanes[1];
        if self.count == 0 {
            self.prev_close = price;
            self.count += 1;
            return self.value;
        }
        if price > self.prev_close {
            self.value += volume;
        } else if price < self.prev_close {
            self.value -= volume;
        }
        self.prev_close = price;
        self.count += 1;
        self.ready = self.count > 1;
        self.value
    }

    /// Returns the current OBV value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the OBV has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Resets the OBV to its initial state.
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.prev_close = 0.0;
        self.ready = false;
        self.count = 0;
    }
}

impl Default for Obv {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, SourceLane, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`Obv`]: only the price source whose up/down ticks gate the
/// cumulative volume flow. OBV has NO period — it is a running cumulative sum.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ObvConfig {
    pub source: Param<OhlcvField>,
}

impl Indicator for Obv {
    const ID: IndicatorId = IndicatorId::Obv;
    /// Not a pluggable family member — OBV is a standalone cumulative volume flow,
    /// not interchangeable with other "volume" indicators in a slot.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Two lanes: a configurable price field (its up/down tick gates direction) + the fixed
    /// volume lane (the magnitude added/subtracted).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Lanes(&[
        SourceLane::Config { default: OhlcvField::Close },
        SourceLane::Fixed(OhlcvField::Volume),
    ]));
    /// O(1) running cumulative sum — no window buffer, the cheapest leaf.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::flow(IndicatorOutputId::Obv)];
    /// OBV is volume-driven: the running sum adds/subtracts the bar volume.
    const NEEDS_VOLUME: bool = true;
    type Config = ObvConfig;
    type Runtime = Obv;

    fn create(_cfg: ObvConfig) -> Obv {
        Obv::new()
    }

    /// Splice the configured price field (lane 0) ahead of the fixed volume lane (lane 1) —
    /// the runtime resolution of `const SOURCE`. The factory extracts both and feeds the pair.
    fn source_fields(cfg: &ObvConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved(), OhlcvField::Volume].into_iter().collect()
    }
}

impl crate::contract::Config for ObvConfig {
    fn defaults() -> Self {
        ObvConfig { source: Param::Solo(OhlcvField::Close) }
    }
    fn machine_defaults() -> Self {
        // source: Class O OhlcvField — auto sets all 8 variants (including Volume)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Obv {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Obv, "OBV", Color::hex(0x2196F3))
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_obv_basic_calculation() {
        let mut obv = Obv::new();

        // First bar: [price, volume]
        obv.feed(&[100.0, 1000.0]);

        // Price up - should add volume
        obv.feed(&[105.0, 500.0]);
        assert!(obv.is_ready());
        assert_eq!(obv.value(), 500.0);

        // Price up again - should add more volume
        obv.feed(&[110.0, 300.0]);
        assert_eq!(obv.value(), 800.0);

        // Price down - should subtract volume
        obv.feed(&[105.0, 200.0]);
        assert_eq!(obv.value(), 600.0);
    }

    #[test]
    fn test_obv_unchanged_price() {
        let mut obv = Obv::new();

        obv.feed(&[100.0, 1000.0]);
        obv.feed(&[105.0, 500.0]);
        let prev_obv = obv.value();

        // Price unchanged - OBV should stay same
        obv.feed(&[105.0, 300.0]);
        assert_eq!(obv.value(), prev_obv);
    }

    #[test]
    fn test_obv_downtrend() {
        let mut obv = Obv::new();

        obv.feed(&[100.0, 1000.0]);

        // Consecutive down days
        for i in 1..=5 {
            obv.feed(&[100.0 - i as f64, 100.0]);
        }

        assert!(obv.is_ready());
        // OBV should be negative after downtrend
        assert!(obv.value() < 0.0, "OBV should be negative in downtrend");
    }

    #[test]
    fn test_obv_reset() {
        let mut obv = Obv::new();

        obv.feed(&[100.0, 1000.0]);
        obv.feed(&[105.0, 500.0]);
        assert!(obv.is_ready());

        obv.reset();
        assert!(!obv.is_ready());
        assert_eq!(obv.value(), 0.0);
    }

    /// The factory resolves the price + volume lanes and feeds the pair; OBV accumulates
    /// end-to-end (not from a raw bar).
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Obv(<<Obv as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        let bar = |close: f64, volume: f64| MarketSample::Bar {
            open: 0.0, high: 0.0, low: 0.0, close, volume,
        };
        f.feed(0, bar(100.0, 1000.0));
        f.feed(0, bar(105.0, 500.0)); // up -> +500
        assert_eq!(f.read(IndicatorOutputId::Obv), 500.0, "factory OBV must add volume on up-close");
    }
}


















