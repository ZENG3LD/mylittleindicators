// Standard Deviation Channel Width: upper_2sigma - lower_2sigma

use crate::indicators::channels::standard_deviation_channels::{
    RegressionSource, StandardDeviationChannels, StandardDeviationMode,
};

#[derive(Debug, Clone)]
pub struct StdDevChannelWidth {
    sdc: StandardDeviationChannels,
    value: f64,
}

impl Default for StdDevChannelWidth {
    fn default() -> Self {
        Self::new(14, 2.0)
    }
}

impl StdDevChannelWidth {
    pub fn new(period: usize, mult: f64) -> Self {
        let sdc = StandardDeviationChannels::new_custom(
            period.max(2),
            mult.max(0.1),
            StandardDeviationMode::Simple,
            RegressionSource::Close,
        );
        Self { sdc, value: 0.0 }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.sdc.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.sdc.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed a scalar — the resolved close price.
    pub fn feed(&mut self, price: f64) -> f64 {
        let (upper, _mid, lower) = self.sdc.feed(price);
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

/// Typed dual-mode config for [`StdDevChannelWidth`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct StdDevChannelWidthConfig {
    pub period: Param<usize>,
    pub std_multiplier: Param<f64>,
    pub source: Param<OhlcvField>,
}

impl Indicator for StdDevChannelWidth {
    const ID: IndicatorId = IndicatorId::Stddevwidth;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Stddevchan, &[
            IndicatorOutputId::StddevchanUpper,
            IndicatorOutputId::StddevchanLower,
        ])],
    };
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Stddevwidth)];
    type Config = StdDevChannelWidthConfig;
    type Runtime = StdDevChannelWidth;

    fn create(cfg: StdDevChannelWidthConfig) -> StdDevChannelWidth {
        StdDevChannelWidth::new(cfg.period.resolved(), cfg.std_multiplier.resolved())
    }

    fn source_fields(cfg: &StdDevChannelWidthConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for StdDevChannelWidthConfig {
    fn defaults() -> Self {
        StdDevChannelWidthConfig {
            period: Param::Solo(14),
            std_multiplier: Param::Solo(2.0),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1), source→all 8
        s.std_multiplier = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for StdDevChannelWidth {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Stddevwidth, "StdDev Width", Color::hex(0x2196F3))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_stddevwidth() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<StdDevChannelWidth as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Stddevwidth(cfg).build_solo().unwrap();
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
        assert!(v >= 0.0, "stddevwidth should be >= 0, got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stddev_channel_width_creation() {
        let scw = StdDevChannelWidth::new(20, 2.0);
        assert!(!scw.is_ready());
        assert_eq!(scw.value(), 0.0);
    }

    #[test]
    fn test_stddev_channel_width_warmup() {
        let mut scw = StdDevChannelWidth::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            scw.feed(price);
        }
        assert!(scw.is_ready());
    }

    #[test]
    fn test_stddev_channel_width_positive() {
        let mut scw = StdDevChannelWidth::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = scw.feed(price);
            assert!(value >= 0.0, "Width should be non-negative");
        }
    }

    #[test]
    fn test_stddev_channel_width_reset() {
        let mut scw = StdDevChannelWidth::new(20, 2.0);
        for i in 0..25 {
            scw.feed(100.0 + i as f64);
        }
        scw.reset();
        assert!(!scw.is_ready());
        assert_eq!(scw.value(), 0.0);
    }
}
