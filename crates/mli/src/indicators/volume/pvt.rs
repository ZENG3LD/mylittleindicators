// Price Volume Trend (PVT)

#[derive(Debug, Clone)]
pub struct PriceVolumeTrend {
    prev_close: f64,
    initialized: bool,
    value: f64,
}

impl Default for PriceVolumeTrend {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, SourceLane, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Unit config — PVT has no parameters (cumulative running sum, no period).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PvtConfig;

impl Indicator for PriceVolumeTrend {
    const ID: IndicatorId = IndicatorId::Pvt;
    /// Cumulative volume trend accumulator — not pluggable as an oscillator family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable price field (lane 0) + fixed volume (lane 1).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Lanes(&[
        SourceLane::Config { default: OhlcvField::Close },
        SourceLane::Fixed(OhlcvField::Volume),
    ]));
    const NEEDS_VOLUME: bool = true;
    /// O(1) cumulative sum — only scalar state, no window buffer.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::flow(IndicatorOutputId::Pvt)];
    type Config = PvtConfig;
    type Runtime = PriceVolumeTrend;

    fn create(_cfg: PvtConfig) -> PriceVolumeTrend {
        PriceVolumeTrend::new()
    }

    fn source_fields(cfg: &PvtConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close, OhlcvField::Volume].into_iter().collect()
    }
}

impl crate::contract::Config for PvtConfig {
    fn defaults() -> Self {
        PvtConfig
    }
    fn machine_defaults() -> Self {
        // No fields — unit config, nothing to sweep
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for PriceVolumeTrend {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Pvt, "PVT", Color::hex(0x009688))
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_lanes() {
        let mut f = IndicatorOrder::Pvt(<<PriceVolumeTrend as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        let bar = |close: f64, volume: f64| MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close, volume,
        };
        f.feed(0, bar(100.0, 1000.0));
        f.feed(0, bar(101.0, 1000.0)); // +1% price → +10 PVT
        let v = f.read(IndicatorOutputId::Pvt);
        assert!(v > 0.0, "rising close should give positive PVT, got {v}");
    }
}

impl PriceVolumeTrend {
    pub fn new() -> Self {
        Self {
            prev_close: 0.0,
            initialized: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.prev_close = 0.0;
        self.initialized = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed the resolved `[close, volume]` lanes (in `SOURCE` order). Accumulates the
    /// price-change-weighted volume delta.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let c = lanes[0];
        let v = lanes[1];
        if !self.initialized {
            self.prev_close = c;
            self.initialized = true;
            return self.value;
        }
        let pct_change = if self.prev_close.abs() > 1e-12 {
            (c - self.prev_close) / self.prev_close
        } else {
            0.0
        };
        self.value += pct_change * v;
        self.prev_close = c;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pvt_creation() {
        let pvt = PriceVolumeTrend::new();
        assert!(!pvt.is_ready());
        assert_eq!(pvt.value(), 0.0);
    }

    #[test]
    fn test_pvt_warmup() {
        let mut pvt = PriceVolumeTrend::new();
        pvt.feed(&[100.0, 1000.0]);
        assert!(pvt.is_ready());
    }

    #[test]
    fn test_pvt_accumulation() {
        let mut pvt = PriceVolumeTrend::new();
        // First bar initializes
        pvt.feed(&[100.0, 1000.0]);
        // Price goes up - positive PVT
        let value = pvt.feed(&[101.0, 1000.0]);
        assert!(value > 0.0, "Rising price should add positive PVT");
    }

    #[test]
    fn test_pvt_values_finite() {
        let mut pvt = PriceVolumeTrend::new();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pvt.feed(&[price, 1000.0]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_pvt_reset() {
        let mut pvt = PriceVolumeTrend::new();
        for _i in 0..10 {
            pvt.feed(&[101.0, 1000.0]);
        }
        pvt.reset();
        assert!(!pvt.is_ready());
        assert_eq!(pvt.value(), 0.0);
    }
}
