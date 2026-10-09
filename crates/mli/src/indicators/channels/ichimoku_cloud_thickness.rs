// Ichimoku Cloud Thickness: current_cloud_top - current_cloud_bottom

use crate::indicators::channels::ichimoku_cloud::IchimokuCloud;

#[derive(Debug, Clone)]
pub struct IchimokuCloudThickness {
    cloud: IchimokuCloud,
    value: f64,
}

impl Default for IchimokuCloudThickness {
    fn default() -> Self {
        Self::new()
    }
}

impl IchimokuCloudThickness {
    pub fn new() -> Self {
        Self {
            cloud: IchimokuCloud::new(),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.cloud.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.cloud.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed resolved [High, Low, Close] lanes.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let _ = self.cloud.feed(&[high, low, close]);
        let (top, bottom) = self.cloud.current_cloud();
        self.value = (top - bottom).abs();
        self.value
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Port, SourceAxis, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`IchimokuCloudThickness`] — no parameters (standard Ichimoku periods).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IchimokuCloudThicknessConfig;

impl IchimokuCloudThicknessConfig {
    /// Config fingerprint: no params → stable constant.
    pub fn config_hash(&self) -> u64 { 0 }

    /// Paramless config — the sole cube point is `self` (mixed-radix dual of `iter().nth`).
    pub fn axes_decode(&self, _idx: u128) -> Self { self.clone() }
}

impl Indicator for IchimokuCloudThickness {
    const ID: IndicatorId = IndicatorId::Ichimokuthick;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Ichimoku, &[
            IndicatorOutputId::IchimokuSenkouA,
            IndicatorOutputId::IchimokuSenkouB,
        ])],
    };
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Ichimokuthick)];
    type Config = IchimokuCloudThicknessConfig;
    type Runtime = IchimokuCloudThickness;

    fn create(_cfg: IchimokuCloudThicknessConfig) -> IchimokuCloudThickness {
        IchimokuCloudThickness::new()
    }
}

impl crate::contract::Config for IchimokuCloudThicknessConfig {
    fn defaults() -> Self {
        IchimokuCloudThicknessConfig
    }
    fn cube_size(&self) -> u128 { 1 }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        Box::new(std::iter::once(IchimokuCloudThicknessConfig))
    }
    fn machine_defaults() -> Self {
        IchimokuCloudThicknessConfig // zero-sized unit struct — no axes to sweep
    }
}


impl Render for IchimokuCloudThickness {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Ichimokuthick, "Cloud Thickness", Color::hex(0x2196F3))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_ichimokuthick() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<IchimokuCloudThickness as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Ichimokuthick(cfg).build_solo().unwrap();
        for i in 1..=60usize {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 1000.0,
            });
        }
        let v = f.primary();
        assert!(v >= 0.0, "ichimokuthick should be >= 0, got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ichimoku_cloud_thickness_creation() {
        let ict = IchimokuCloudThickness::new();
        assert!(!ict.is_ready());
        assert_eq!(ict.value(), 0.0);
    }

    #[test]
    fn test_ichimoku_cloud_thickness_warmup() {
        let mut ict = IchimokuCloudThickness::new();
        for i in 0..55 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ict.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ict.is_ready());
    }

    #[test]
    fn test_ichimoku_cloud_thickness_positive() {
        let mut ict = IchimokuCloudThickness::new();
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ict.feed(&[price + 1.0, price - 1.0, price]);
            assert!(value >= 0.0, "Thickness should be non-negative");
        }
    }

    #[test]
    fn test_ichimoku_cloud_thickness_reset() {
        let mut ict = IchimokuCloudThickness::new();
        for i in 0..60 {
            ict.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        ict.reset();
        assert!(!ict.is_ready());
        assert_eq!(ict.value(), 0.0);
    }
}
