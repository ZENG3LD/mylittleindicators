/// Balance of Power (BOP) = (Close - Open) / (High - Low)
#[derive(Debug, Clone)]
pub struct Bop {
    value: f64,
}

impl Default for Bop {
    fn default() -> Self {
        Self::new()
    }
}

impl Bop {
    pub fn new() -> Self {
        Self { value: 0.0 }
    }
    /// Feed the resolved input lanes — `[open, high, low, close]` (the factory resolves the
    /// fixed OHLC slice). `(close - open) / (high - low)`; knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let denom = (lanes[1] - lanes[2]).abs().max(1e-12);
        self.value = (lanes[3] - lanes[0]) / denom;
        self.value
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        true
    }
    pub fn reset(&mut self) {
        self.value = 0.0;
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`Bop`] — periodless, stateless. The catalog declares a
/// `period` constraint for UI uniformity, but the algorithm is a per-bar ratio
/// with no window; there is nothing to configure.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct BopConfig;

impl Indicator for Bop {
    const ID: IndicatorId = IndicatorId::Bop;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to the full OHLC bar — `(close - open) / (high - low)`; it reads all
    /// four price fields at once, so the source is fixed, not a single field.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// O(1) stateless ratio — one cached scalar, no window buffer, no sub-deps.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Bop)];

    type Config = BopConfig;
    type Runtime = Bop;

    fn create(_cfg: BopConfig) -> Bop {
        Bop::new()
    }
}

impl crate::contract::Config for BopConfig {
    fn defaults() -> Self {
        BopConfig
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // BopConfig has no Param fields — zero-dimensional config. Auto suffices.
        Self::machine_defaults_auto()
    }
}


impl Render for Bop {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Bop, "Balance of Power", Color::hex(0x2196F3))
            .bounds(-1.0, 1.0)
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bop_creation() {
        let bop = Bop::new();
        assert!(bop.is_ready()); // BOP always ready (stateless)
        assert_eq!(bop.value(), 0.0);
    }

    #[test]
    fn test_bop_bullish_bar() {
        let mut bop = Bop::new();
        // Close > Open = bullish; lanes = [open, high, low, close]
        let value = bop.feed(&[100.0, 110.0, 95.0, 108.0]);
        // (108 - 100) / (110 - 95) = 8 / 15 ≈ 0.533
        assert!(value > 0.0, "BOP should be positive for bullish bar");
        assert!((value - 8.0 / 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_bop_bearish_bar() {
        let mut bop = Bop::new();
        // Close < Open = bearish
        let value = bop.feed(&[100.0, 105.0, 90.0, 92.0]);
        // (92 - 100) / (105 - 90) = -8 / 15 ≈ -0.533
        assert!(value < 0.0, "BOP should be negative for bearish bar");
        assert!((value - (-8.0 / 15.0)).abs() < 1e-10);
    }

    #[test]
    fn test_bop_doji() {
        let mut bop = Bop::new();
        // Close == Open
        let value = bop.feed(&[100.0, 105.0, 95.0, 100.0]);
        assert_eq!(value, 0.0, "BOP should be 0 for doji (close == open)");
    }

    #[test]
    fn test_bop_full_range_bullish() {
        let mut bop = Bop::new();
        // Close = High, Open = Low
        let value = bop.feed(&[90.0, 110.0, 90.0, 110.0]);
        // (110 - 90) / (110 - 90) = 1.0
        assert!((value - 1.0).abs() < 1e-10, "BOP should be 1.0 for full bullish bar");
    }

    #[test]
    fn test_bop_full_range_bearish() {
        let mut bop = Bop::new();
        // Close = Low, Open = High
        let value = bop.feed(&[110.0, 110.0, 90.0, 90.0]);
        // (90 - 110) / (110 - 90) = -1.0
        assert!((value - (-1.0)).abs() < 1e-10, "BOP should be -1.0 for full bearish bar");
    }

    #[test]
    fn test_bop_reset() {
        let mut bop = Bop::new();
        bop.feed(&[100.0, 110.0, 95.0, 105.0]);
        assert!(bop.value() != 0.0);
        bop.reset();
        assert_eq!(bop.value(), 0.0);
    }

    #[test]
    fn test_bop_sequence() {
        let mut bop = Bop::new();
        // Alternating bullish/bearish bars
        let v1 = bop.feed(&[100.0, 110.0, 95.0, 108.0]);
        assert!(v1 > 0.0);
        let v2 = bop.feed(&[108.0, 112.0, 100.0, 102.0]);
        assert!(v2 < 0.0);
        let v3 = bop.feed(&[102.0, 115.0, 100.0, 113.0]);
        assert!(v3 > 0.0);
    }

    #[test]
    fn test_bop_range_bounds() {
        let mut bop = Bop::new();
        // BOP is bounded between -1 and 1
        for i in 0..100 {
            let o = 100.0 + (i % 20) as f64;
            let c = 100.0 + ((i + 10) % 20) as f64;
            let h = o.max(c) + 5.0;
            let l = o.min(c) - 5.0;
            let value = bop.feed(&[o, h, l, c]);
            assert!(value >= -1.0 && value <= 1.0, "BOP should be in [-1, 1], got {}", value);
        }
    }

    /// The factory resolves the fixed OHLC lanes from `const SOURCE` and feeds the four
    /// scalars; BOP computes the ratio end-to-end.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Bop(<<Bop as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        f.feed(0, MarketSample::Bar {
            open: 100.0, high: 110.0, low: 95.0, close: 108.0, volume: 1000.0,
        });
        // (108 - 100) / (110 - 95) = 8/15
        assert!((f.primary() - 8.0 / 15.0).abs() < 1e-10, "factory BOP = 8/15, got {}", f.primary());
    }
}
