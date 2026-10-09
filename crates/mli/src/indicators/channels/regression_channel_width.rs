// Regression Channel Width: upper - lower from RegressionChannels

use crate::indicators::channels::regression_channels::{
    RegressionChannelMode, RegressionChannels,
};

#[derive(Debug, Clone)]
pub struct RegressionChannelWidth {
    rc: RegressionChannels,
    value: f64,
}

impl Default for RegressionChannelWidth {
    fn default() -> Self {
        Self::new(14, 2.0)
    }
}

impl RegressionChannelWidth {
    pub fn new(period: usize, mult: f64) -> Self {
        Self {
            rc: RegressionChannels::new(
                period.max(2),
                mult.max(0.1),
                RegressionChannelMode::Standard,
            ),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.rc.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rc.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed a scalar — the resolved close price.
    pub fn feed(&mut self, price: f64) -> f64 {
        let (upper, _mid, lower) = self.rc.feed(price);
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

/// Typed dual-mode config for [`RegressionChannelWidth`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RegressionChannelWidthConfig {
    pub period: Param<usize>,
    pub std_dev_mult: Param<f64>,
    pub source: Param<OhlcvField>,
}

impl Indicator for RegressionChannelWidth {
    const ID: IndicatorId = IndicatorId::Regchanwidth;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Regchan, &[
            IndicatorOutputId::RegchanUpper,
            IndicatorOutputId::RegchanLower,
        ])],
    };
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Regchanwidth)];
    type Config = RegressionChannelWidthConfig;
    type Runtime = RegressionChannelWidth;

    fn create(cfg: RegressionChannelWidthConfig) -> RegressionChannelWidth {
        RegressionChannelWidth::new(cfg.period.resolved(), cfg.std_dev_mult.resolved())
    }

    fn source_fields(cfg: &RegressionChannelWidthConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for RegressionChannelWidthConfig {
    fn defaults() -> Self {
        RegressionChannelWidthConfig {
            period: Param::Solo(20),
            std_dev_mult: Param::Solo(2.0),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1), source→all 8
        s.std_dev_mult = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for RegressionChannelWidth {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Regchanwidth, "Reg Chan Width", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_regchanwidth() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<RegressionChannelWidth as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Regchanwidth(cfg).build_solo().unwrap();
        for i in 1..=30usize {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 0.0,
                close: price,
                volume: 1000.0,
            });
        }
        let v = f.primary();
        assert!(v >= 0.0, "regchanwidth should be >= 0, got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regression_channel_width_creation() {
        let rcw = RegressionChannelWidth::new(20, 2.0);
        assert!(!rcw.is_ready());
        assert_eq!(rcw.value(), 0.0);
    }

    #[test]
    fn test_regression_channel_width_warmup() {
        let mut rcw = RegressionChannelWidth::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rcw.feed(price);
        }
        assert!(rcw.is_ready());
    }

    #[test]
    fn test_regression_channel_width_positive() {
        let mut rcw = RegressionChannelWidth::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = rcw.feed(price);
            assert!(value >= 0.0, "Width should be non-negative");
        }
    }

    #[test]
    fn test_regression_channel_width_reset() {
        let mut rcw = RegressionChannelWidth::new(20, 2.0);
        for i in 0..25 {
            rcw.feed(100.0 + i as f64);
        }
        rcw.reset();
        assert!(!rcw.is_ready());
        assert_eq!(rcw.value(), 0.0);
    }
}
