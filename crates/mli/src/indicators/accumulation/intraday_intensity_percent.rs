// Intraday Intensity Percent (IIP)

#[derive(Debug, Clone)]
pub struct IntradayIntensityPercent {
    value: f64,
}

impl Default for IntradayIntensityPercent {
    fn default() -> Self {
        Self::new()
    }
}

impl IntradayIntensityPercent {
    pub fn new() -> Self {
        Self { value: 0.0 }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed resolved input lanes `[high, low, close, volume]`.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let h = lanes[0];
        let l = lanes[1];
        let c = lanes[2];
        let v = lanes[3];
        let denom = (h - l).abs().max(1e-9);
        self.value = ((2.0 * c - h - l) / denom) * v;
        self.value
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec, ReferenceLine};
use crate::engine::stream_kind::StreamKind;

/// Unit config — IIP has no configurable parameters.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct IipConfig;

impl Indicator for IntradayIntensityPercent {
    const ID: IndicatorId = IndicatorId::Iip;
    /// No family — raw per-bar II value PRODUCER.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed fields: high, low, close, volume.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    /// O(1) pure scalar — no store.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Iip)];
    type Config = IipConfig;
    type Runtime = IntradayIntensityPercent;

    fn create(_cfg: IipConfig) -> IntradayIntensityPercent {
        IntradayIntensityPercent::new()
    }
}

impl crate::contract::Config for IipConfig {
    fn defaults() -> Self {
        IipConfig
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


impl Render for IntradayIntensityPercent {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Iip, "II %", Color::hex(0x9C27B0))
            .bounds(-100.0, 100.0)
            .reference_line(ReferenceLine::new(0.0, Color::hex(0x9E9E9E)))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intraday_intensity_percent_creation() {
        let iip = IntradayIntensityPercent::new();
        assert!(iip.is_ready());
        assert_eq!(iip.value(), 0.0);
    }

    #[test]
    fn test_intraday_intensity_percent_values_finite() {
        let mut iip = IntradayIntensityPercent::new();
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = iip.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_intraday_intensity_percent_reset() {
        let mut iip = IntradayIntensityPercent::new();
        for i in 0..10 {
            iip.feed(&[105.0, 95.0, 100.0 + i as f64, 1000.0]);
        }
        iip.reset();
        assert_eq!(iip.value(), 0.0);
    }

    /// Factory resolves H/L/C/Volume; open ignored (9999 as proof).
    /// Close at midpoint → raw II = 0.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Iip(<<IntradayIntensityPercent as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        f.feed(0, MarketSample::Bar {
            open: 9999.0, high: 110.0, low: 90.0, close: 100.0, volume: 1000.0,
        });
        // close = 100, midpoint = (110+90)/2 = 100 → 2c - h - l = 0 → value = 0
        assert_eq!(f.read(IndicatorOutputId::Iip), 0.0);
        // Bullish: close near high
        f.feed(0, MarketSample::Bar {
            open: 9999.0, high: 110.0, low: 90.0, close: 108.0, volume: 1000.0,
        });
        assert!(f.read(IndicatorOutputId::Iip) > 0.0);
    }
}
