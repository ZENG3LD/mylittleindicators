// Median Channel Position: (price - lower) / (upper - lower) for MedianChannels (using upper_mad/lower_mad)

use crate::indicators::channels::median_channels::{
    MedianChannels, MedianMode, MedianSource,
};

#[derive(Debug, Clone)]
pub struct MedianChannelPosition {
    mc: MedianChannels,
    value: f64,
}

impl Default for MedianChannelPosition {
    fn default() -> Self {
        Self::new(14)
    }
}

impl MedianChannelPosition {
    pub fn new(period: usize) -> Self {
        Self {
            mc: MedianChannels::new_custom(
                period.max(3),
                MedianMode::Simple,
                MedianSource::Close,
                1.4826,
            ),
            value: 0.5,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.mc.reset();
        self.value = 0.5;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.mc.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed a scalar — the resolved close price.
    pub fn feed(&mut self, price: f64) -> f64 {
        let (_median, upper, lower) = self.mc.feed(price);
        let width = (upper - lower).abs();
        self.value = if width > 0.0 {
            ((price - lower) / width).clamp(0.0, 1.0)
        } else {
            0.5
        };
        self.value
    }
}

// ---- Indicator contract ----

use crate::contract::Param;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Port, SourceAxis, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec, ReferenceLine};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`MedianChannelPosition`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct MedianChannelPositionConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl Indicator for MedianChannelPosition {
    const ID: IndicatorId = IndicatorId::Medchanpos;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Medchan, &[
            IndicatorOutputId::MedchanUpper,
            IndicatorOutputId::MedchanLower,
        ])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Medchanpos)];
    type Config = MedianChannelPositionConfig;
    type Runtime = MedianChannelPosition;

    fn create(cfg: MedianChannelPositionConfig) -> MedianChannelPosition {
        MedianChannelPosition::new(cfg.period.resolved())
    }

    fn source_fields(cfg: &MedianChannelPositionConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for MedianChannelPositionConfig {
    fn defaults() -> Self {
        MedianChannelPositionConfig {
            period: Param::Solo(14),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        Self::machine_defaults_auto() // period→range(2,4048,1), source→all 8 — all axes auto
    }
}


impl Render for MedianChannelPosition {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Medchanpos, "Median Chan Position", Color::hex(0x4CAF50))
            .bounds(0.0, 1.0)
            .reference_line(ReferenceLine::new(0.5, Color::hex(0x9E9E9E)))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_medchanpos() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<MedianChannelPosition as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Medchanpos(cfg).build_solo().unwrap();
        for i in 1..=25usize {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 0.0,
                close: price,
                volume: 1000.0,
            });
        }
        let v = f.primary();
        assert!(v.is_finite(), "medchanpos should be finite, got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_median_channel_position_creation() {
        let mcp = MedianChannelPosition::new(20);
        assert!(!mcp.is_ready());
        assert_eq!(mcp.value(), 0.5);
    }

    #[test]
    fn test_median_channel_position_warmup() {
        let mut mcp = MedianChannelPosition::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            mcp.feed(price);
        }
        assert!(mcp.is_ready());
    }

    #[test]
    fn test_median_channel_position_range() {
        let mut mcp = MedianChannelPosition::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = mcp.feed(price);
            assert!(value >= 0.0 && value <= 1.0, "Position should be in [0, 1]");
        }
    }

    #[test]
    fn test_median_channel_position_reset() {
        let mut mcp = MedianChannelPosition::new(20);
        for i in 0..25 {
            mcp.feed(100.0 + i as f64);
        }
        mcp.reset();
        assert!(!mcp.is_ready());
        assert_eq!(mcp.value(), 0.5);
    }
}
