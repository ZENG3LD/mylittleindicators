// Central Pivot Range (CPR): TC, BC, Pivot


#[derive(Debug, Clone)]
pub struct CentralPivotRange {
    pivot: f64,
    tc: f64,
    bc: f64,
    ready: bool,
}

impl Default for CentralPivotRange {
    fn default() -> Self {
        Self::new()
    }
}

impl CentralPivotRange {
    pub fn new() -> Self {
        Self {
            pivot: 0.0,
            tc: 0.0,
            bc: 0.0,
            ready: false,
        }
    }
    pub fn reset(&mut self) {
        self.pivot = 0.0;
        self.tc = 0.0;
        self.bc = 0.0;
        self.ready = false;
    }
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    /// Named output getter: brace `bc`.
    #[inline]
    pub fn bc(&self) -> f64 { self.bc }

    /// Named output getter: brace `tc`.
    #[inline]
    pub fn tc(&self) -> f64 { self.tc }

    /// Named output getter: brace `pivot` (the central pivot point).
    #[inline]
    pub fn pivot(&self) -> f64 { self.pivot }

    // classic daily CPR; uses current bar's HLC as placeholder
    fn recompute(&mut self, h: f64, l: f64, c: f64) -> (f64, f64, f64) {
        self.pivot = (h + l + c) / 3.0;
        let tc = self.pivot - (l);
        let bc = (h) - (self.pivot);
        // ensure tc >= bc by convention
        self.tc = self.pivot + tc.abs().max(bc.abs());
        self.bc = self.pivot - tc.abs().max(bc.abs());
        self.ready = true;
        (self.bc, self.pivot, self.tc)
    }

    /// Feed resolved `[high, low, close]` lanes.
    pub fn feed(&mut self, lanes: &[f64]) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        self.recompute(high, low, close);
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Render, RenderOutput, RenderSpec, SourceAxis,
    UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Unit config — CPR has no parameters.
#[derive(Debug, Clone, Copy, mli_contract_macros::ConfigAxes)]
pub struct CprConfig;

impl crate::contract::Config for CprConfig {
    fn defaults() -> Self {
        CprConfig
    }
    fn machine_defaults() -> Self {
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}

impl Indicator for CentralPivotRange {
    const ID: IndicatorId = IndicatorId::Cpr;
    /// Not a pluggable family member — CPR is a structural level producer.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/C fields — pivot = (H+L+C)/3, TC/BC from range.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// Triple: BC, pivot, TC.
    /// O(1) per bar — pure scalar state, no buffers.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::CprBc),
        Output::price(IndicatorOutputId::CprPivot),
        Output::price(IndicatorOutputId::CprTc),
    ];
    type Config = CprConfig;
    type Runtime = CentralPivotRange;

    fn create(_cfg: CprConfig) -> CentralPivotRange {
        CentralPivotRange::new()
    }
}


impl Render for CentralPivotRange {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::CprPivot, "Pivot", Color::hex(0x2196F3), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::CprBc, "BC", Color::hex(0x4CAF50), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::CprTc, "TC", Color::hex(0xF44336), 1.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_cpr() {
        let mut f = IndicatorOrder::Cpr(<<CentralPivotRange as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        f.feed(0, MarketSample::Bar {
            open: 9999.0,
            high: 105.0,
            low: 95.0,
            close: 100.0,
            volume: 9999.0,
        });
        assert!(f.is_ready());
        // pivot = (105+95+100)/3 = 100; bc < pivot, main() = bc
        let v = f.primary();
        assert!(v < 100.0 && v > 80.0, "cpr bc out of range: {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_central_pivot_range_creation() {
        let cpr = CentralPivotRange::new();
        assert!(!cpr.is_ready());
    }

    #[test]
    fn test_central_pivot_range_update() {
        let mut cpr = CentralPivotRange::new();
        let (bc, pivot, tc) = cpr.recompute(105.0, 95.0, 100.0);
        assert!(cpr.is_ready());
        // pivot = (105 + 95 + 100) / 3 = 100
        assert!((pivot - 100.0).abs() < 0.001);
        assert!(bc < pivot);
        assert!(pivot < tc);
    }

    #[test]
    fn test_central_pivot_range_order() {
        let mut cpr = CentralPivotRange::new();
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (bc, pivot, tc) = cpr.recompute(price + 5.0, price - 5.0, price);
            assert!(bc <= pivot, "BC should be <= pivot");
            assert!(pivot <= tc, "Pivot should be <= TC");
        }
    }

    #[test]
    fn test_central_pivot_range_reset() {
        let mut cpr = CentralPivotRange::new();
        cpr.recompute(105.0, 95.0, 100.0);
        cpr.reset();
        assert!(!cpr.is_ready());
    }
}
