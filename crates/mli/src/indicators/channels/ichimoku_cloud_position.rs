// Ichimoku Cloud Position: normalized position of price within current cloud (0..1)

use crate::indicators::channels::ichimoku_cloud::IchimokuCloud;

#[derive(Debug, Clone)]
pub struct IchimokuCloudPosition {
    cloud: IchimokuCloud,
    value: f64,
}

impl Default for IchimokuCloudPosition {
    fn default() -> Self {
        Self::new()
    }
}

impl IchimokuCloudPosition {
    pub fn new() -> Self {
        Self {
            cloud: IchimokuCloud::new(),
            value: 0.5,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.cloud.reset();
        self.value = 0.5;
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
        let width = (top - bottom).abs();
        if width > 0.0 {
            self.value = ((close - bottom) / width).clamp(0.0, 1.0);
        } else {
            self.value = 0.5;
        }
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

/// Typed config for [`IchimokuCloudPosition`] — no parameters (Ichimoku uses fixed standard periods).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IchimokuCloudPositionConfig;

impl IchimokuCloudPositionConfig {
    /// Config fingerprint: no params → stable constant.
    pub fn config_hash(&self) -> u64 { 0 }

    /// Paramless config — the sole cube point is `self` (mixed-radix dual of `iter().nth`).
    pub fn axes_decode(&self, _idx: u128) -> Self { self.clone() }
}

impl Indicator for IchimokuCloudPosition {
    const ID: IndicatorId = IndicatorId::Ichimokupos;
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
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Ichimokupos)];
    type Config = IchimokuCloudPositionConfig;
    type Runtime = IchimokuCloudPosition;

    fn create(_cfg: IchimokuCloudPositionConfig) -> IchimokuCloudPosition {
        IchimokuCloudPosition::new()
    }
}

impl crate::contract::Config for IchimokuCloudPositionConfig {
    fn defaults() -> Self {
        IchimokuCloudPositionConfig
    }
    fn cube_size(&self) -> u128 { 1 }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        Box::new(std::iter::once(IchimokuCloudPositionConfig))
    }
    fn machine_defaults() -> Self {
        IchimokuCloudPositionConfig // zero-sized unit struct — no axes to sweep
    }
}


impl Render for IchimokuCloudPosition {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Ichimokupos, "Cloud Position", Color::hex(0x9C27B0))
            .bounds(0.0, 1.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_ichimokupos() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<IchimokuCloudPosition as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Ichimokupos(cfg).build_solo().unwrap();
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
        assert!(v.is_finite(), "ichimokupos should be finite, got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ichimoku_cloud_position_creation() {
        let icp = IchimokuCloudPosition::new();
        assert!(!icp.is_ready());
        assert_eq!(icp.value(), 0.5);
    }

    #[test]
    fn test_ichimoku_cloud_position_warmup() {
        let mut icp = IchimokuCloudPosition::new();
        for i in 0..55 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            icp.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(icp.is_ready());
    }

    #[test]
    fn test_ichimoku_cloud_position_range() {
        let mut icp = IchimokuCloudPosition::new();
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = icp.feed(&[price + 1.0, price - 1.0, price]);
            assert!(value >= 0.0 && value <= 1.0, "Position should be in [0, 1]");
        }
    }

    #[test]
    fn test_ichimoku_cloud_position_reset() {
        let mut icp = IchimokuCloudPosition::new();
        for i in 0..60 {
            icp.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        icp.reset();
        assert!(!icp.is_ready());
        assert_eq!(icp.value(), 0.5);
    }
}
