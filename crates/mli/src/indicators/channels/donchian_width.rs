// Donchian Width: upper - lower band of Donchian Channel

use crate::indicators::channels::donchian_channel::DonchianChannel;

#[derive(Debug, Clone)]
pub struct DonchianWidth {
    dc: DonchianChannel,
    value: f64,
}

impl Default for DonchianWidth {
    fn default() -> Self {
        Self::new(14)
    }
}

impl DonchianWidth {
    pub fn new(period: usize) -> Self {
        Self {
            dc: DonchianChannel::new(period.max(2)),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.dc.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.dc.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed resolved [High, Low] lanes.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let (upper, lower, _mid) = self.dc.feed(&[high, low]);
        self.value = upper - lower;
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
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`DonchianWidth`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DonchianWidthConfig {
    pub period: Param<usize>,
}

impl Indicator for DonchianWidth {
    const ID: IndicatorId = IndicatorId::Dcwidth;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L only — width = upper - lower.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Dc, &[
            IndicatorOutputId::DcUpper,
            IndicatorOutputId::DcLower,
        ])],
    };
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Dcwidth)];
    type Config = DonchianWidthConfig;
    type Runtime = DonchianWidth;

    fn create(cfg: DonchianWidthConfig) -> DonchianWidth {
        DonchianWidth::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for DonchianWidthConfig {
    fn defaults() -> Self {
        DonchianWidthConfig { period: Param::Solo(14) }
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


impl Render for DonchianWidth {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Dcwidth, "DC Width", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_dcwidth() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<DonchianWidth as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Dcwidth(cfg).build_solo().unwrap();
        for i in 1..=25usize {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.0,
                low: price - 1.0,
                close: 9999.0, // wild — not used for width
                volume: 1000.0,
            });
        }
        let v = f.primary();
        assert!(v >= 0.0, "dcwidth should be >= 0, got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_donchian_width_creation() {
        let dw = DonchianWidth::new(20);
        assert!(!dw.is_ready());
        assert_eq!(dw.value(), 0.0);
    }

    #[test]
    fn test_donchian_width_warmup() {
        let mut dw = DonchianWidth::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            dw.feed(&[price + 1.0, price - 1.0]);
        }
        assert!(dw.is_ready());
    }

    #[test]
    fn test_donchian_width_positive() {
        let mut dw = DonchianWidth::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = dw.feed(&[price + 1.0, price - 1.0]);
            if dw.is_ready() {
                assert!(value >= 0.0, "Width should be non-negative");
            }
        }
    }

    #[test]
    fn test_donchian_width_reset() {
        let mut dw = DonchianWidth::new(20);
        for i in 0..25 {
            dw.feed(&[101.0 + i as f64 * 0.0, 99.0]);
        }
        dw.reset();
        assert!(!dw.is_ready());
        assert_eq!(dw.value(), 0.0);
    }
}
