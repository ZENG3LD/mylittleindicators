// Donchian Position: (Close - Lower) / (Upper - Lower)

use crate::indicators::channels::donchian_channel::DonchianChannel;

#[derive(Debug, Clone)]
pub struct DonchianPosition {
    dc: DonchianChannel,
    value: f64,
}

impl Default for DonchianPosition {
    fn default() -> Self {
        Self::new(14)
    }
}

impl DonchianPosition {
    pub fn new(period: usize) -> Self {
        Self {
            dc: DonchianChannel::new(period.max(2)),
            value: 0.5,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.dc.reset();
        self.value = 0.5;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.dc.is_ready()
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
        let (upper, lower, _mid) = self.dc.feed(&[high, low]);
        let width = (upper - lower).max(0.0);
        self.value = if width > 0.0 {
            (close - lower) / width
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

/// Typed config for [`DonchianPosition`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DonchianPositionConfig {
    pub period: Param<usize>,
}

impl Indicator for DonchianPosition {
    const ID: IndicatorId = IndicatorId::Dcpos;
    /// Not a pluggable family — a position/oscillator derived from a channel, not a channel.
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
        inner: &[Port::new(IndicatorId::Dc, &[
            IndicatorOutputId::DcUpper,
            IndicatorOutputId::DcLower,
        ])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Dcpos)];
    type Config = DonchianPositionConfig;
    type Runtime = DonchianPosition;

    fn create(cfg: DonchianPositionConfig) -> DonchianPosition {
        DonchianPosition::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for DonchianPositionConfig {
    fn defaults() -> Self {
        DonchianPositionConfig { period: Param::Solo(14) }
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


impl Render for DonchianPosition {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Dcpos, "DC Position", Color::hex(0x2196F3))
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
    fn factory_feeds_resolved_dcpos() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<DonchianPosition as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Dcpos(cfg).build_solo().unwrap();
        for i in 1..=25usize {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 1000.0,
            });
        }
        let v = f.primary();
        assert!(v >= 0.0 && v <= 1.0, "dcpos should be in [0,1], got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_donchian_position_creation() {
        let dp = DonchianPosition::new(20);
        assert!(!dp.is_ready());
        assert_eq!(dp.value(), 0.5);
    }

    #[test]
    fn test_donchian_position_warmup() {
        let mut dp = DonchianPosition::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            dp.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(dp.is_ready());
    }

    #[test]
    fn test_donchian_position_range() {
        let mut dp = DonchianPosition::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = dp.feed(&[price + 1.0, price - 1.0, price]);
            assert!(value.is_finite(), "Position should be finite");
        }
    }

    #[test]
    fn test_donchian_position_reset() {
        let mut dp = DonchianPosition::new(20);
        for i in 0..25 {
            dp.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        dp.reset();
        assert!(!dp.is_ready());
        assert_eq!(dp.value(), 0.5);
    }
}
