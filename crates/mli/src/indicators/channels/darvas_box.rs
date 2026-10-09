// Darvas Box - simple box breakout structure


#[derive(Debug, Clone)]
pub struct DarvasBox {
    lookback: usize,
    high_box: f64,
    low_box: f64,
    ready: bool,
}

impl Default for DarvasBox {
    fn default() -> Self {
        Self::new(5)
    }
}

impl DarvasBox {
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback: lookback.max(2),
            high_box: 0.0,
            low_box: 0.0,
            ready: false,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.high_box = 0.0;
        self.low_box = 0.0;
        self.ready = false;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    #[inline]
    pub fn value_tuple(&self) -> (f64, f64) {
        (self.high_box, self.low_box)
    }
    pub fn lookback(&self) -> usize {
        self.lookback
    }

    #[inline]
    pub fn high(&self) -> f64 {
        self.high_box
    }

    #[inline]
    pub fn low(&self) -> f64 {
        self.low_box
    }

    /// Feed resolved High/Low lanes — the contract feed path (no `update_bar`).
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        if !self.ready {
            self.high_box = high;
            self.low_box = low;
            self.ready = true;
        } else {
            self.high_box = self.high_box.max(high);
            self.low_box = self.low_box.min(low);
        }
        (self.high_box, self.low_box)
    }
}

// ---- Indicator contract ----

use crate::contract::Param;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`DarvasBox`] — lookback period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DarvasConfig {
    pub period: Param<usize>,
}

impl Indicator for DarvasBox {
    const ID: IndicatorId = IndicatorId::Darvas;
    /// No family — a box DETECTOR (cumulative high/low envelope), not a pluggable channel.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    /// O(1) — accumulates running max/min, no window scan. Scalar state only.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::fixed(StoreKind::Scalar, 2)]);
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::DarvasHigh),
        Output::price(IndicatorOutputId::DarvasLow),
    ];
    type Config = DarvasConfig;
    type Runtime = DarvasBox;

    fn create(cfg: DarvasConfig) -> DarvasBox {
        DarvasBox::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for DarvasConfig {
    fn defaults() -> Self {
        DarvasConfig { period: Param::Solo(5) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        Self::machine_defaults_auto() // period→range(2,4048,1) — only axis
    }
}


impl Render for DarvasBox {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::DarvasHigh, "Box Top", Color::hex(0x4CAF50), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::DarvasLow, "Box Bottom", Color::hex(0xF44336), 2.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_darvas_box_creation() {
        let db = DarvasBox::new(20);
        assert!(!db.is_ready());
        assert_eq!(db.lookback(), 20);
    }

    #[test]
    fn test_darvas_box_warmup() {
        let mut db = DarvasBox::new(20);
        let (high, low) = db.feed(&[101.0, 99.0]);
        assert!(db.is_ready());
        assert_eq!(high, 101.0);
        assert_eq!(low, 99.0);
    }

    #[test]
    fn test_darvas_box_expansion() {
        let mut db = DarvasBox::new(20);
        db.feed(&[101.0, 99.0]);
        let (high, low) = db.feed(&[105.0, 95.0]);
        assert_eq!(high, 105.0);
        assert_eq!(low, 95.0);
    }

    #[test]
    fn test_darvas_box_reset() {
        let mut db = DarvasBox::new(20);
        db.feed(&[101.0, 99.0]);
        db.reset();
        assert!(!db.is_ready());
    }

    /// Factory resolves fixed High/Low; wild close/open/volume are ignored.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<DarvasBox as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Darvas(cfg).build_solo().unwrap();
        f.feed(0, MarketSample::Bar {
            open: 9999.0, high: 110.0, low: 90.0, close: 9999.0, volume: 9999.0,
        });
        f.feed(0, MarketSample::Bar {
            open: 9999.0, high: 115.0, low: 85.0, close: 9999.0, volume: 9999.0,
        });
        assert!(f.is_ready());
        // Double(high_box, low_box) -> main() == high_box == 115.0
        assert!((f.primary() - 115.0).abs() < 1e-9, "expected 115.0, got {}", f.primary());
    }
}
