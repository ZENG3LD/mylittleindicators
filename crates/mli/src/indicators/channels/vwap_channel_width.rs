// VWAP Channel Width: (Upper - Lower) from VwapChannels

use crate::indicators::channels::vwap_channels::{VwapChannelMode, VwapChannels};

#[derive(Debug, Clone)]
pub struct VwapChannelWidth {
    vc: VwapChannels,
    value: f64,
}

impl Default for VwapChannelWidth {
    fn default() -> Self {
        Self::new(14, 2.0)
    }
}

impl VwapChannelWidth {
    pub fn new(period: usize, mult: f64) -> Self {
        Self {
            vc: VwapChannels::new(period.max(1), mult.max(0.1), VwapChannelMode::Standard),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.vc.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.vc.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed resolved [High, Low, Close, Volume] lanes.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let (upper, _mid, lower) = self.vc.feed(lanes);
        self.value = (upper - lower).abs();
        self.value
    }
}

// ---- Indicator contract ----

use crate::contract::{Param, sweep_f64};
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Port, SourceAxis, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`VwapChannelWidth`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VwapChannelWidthConfig {
    pub period: Param<usize>,
    pub std_dev_mult: Param<f64>,
}

impl Indicator for VwapChannelWidth {
    const ID: IndicatorId = IndicatorId::Vwapchanwidth;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/C/Volume lanes — mirrors VwapChannels source.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const NEEDS_VOLUME: bool = true;
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Vwapchan, &[
            IndicatorOutputId::VwapchanUpper,
            IndicatorOutputId::VwapchanLower,
        ])],
    };
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Vwapchanwidth)];
    type Config = VwapChannelWidthConfig;
    type Runtime = VwapChannelWidth;

    fn create(cfg: VwapChannelWidthConfig) -> VwapChannelWidth {
        VwapChannelWidth::new(cfg.period.resolved(), cfg.std_dev_mult.resolved())
    }
}

impl crate::contract::Config for VwapChannelWidthConfig {
    fn defaults() -> Self {
        VwapChannelWidthConfig {
            period: Param::Solo(20),
            std_dev_mult: Param::Solo(2.0),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1)
        s.std_dev_mult = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for VwapChannelWidth {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vwapchanwidth, "VWAP Chan Width", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_vwapchanwidth() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<VwapChannelWidth as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Vwapchanwidth(cfg).build_solo().unwrap();
        for i in 1..=30usize {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 1000.0,
            });
        }
        let v = f.primary();
        assert!(v >= 0.0, "vwapchanwidth should be >= 0, got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vwap_channel_width_creation() {
        let vcw = VwapChannelWidth::new(20, 2.0);
        assert!(!vcw.is_ready());
        assert_eq!(vcw.value(), 0.0);
    }

    #[test]
    fn test_vwap_channel_width_warmup() {
        let mut vcw = VwapChannelWidth::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vcw.feed(&[price + 1.0, price - 1.0, price, 1000.0]);
        }
        assert!(vcw.is_ready());
    }

    #[test]
    fn test_vwap_channel_width_positive() {
        let mut vcw = VwapChannelWidth::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vcw.feed(&[price + 2.0, price - 2.0, price, 1000.0]);
            assert!(value >= 0.0, "Width should be non-negative");
        }
    }

    #[test]
    fn test_vwap_channel_width_reset() {
        let mut vcw = VwapChannelWidth::new(20, 2.0);
        for i in 0..25 {
            vcw.feed(&[101.0, 99.0, 100.0 + i as f64, 1000.0]);
        }
        vcw.reset();
        assert!(!vcw.is_ready());
        assert_eq!(vcw.value(), 0.0);
    }
}
