// Price Channel Width: upper - lower from PriceChannels

use crate::indicators::channels::price_channels::{PriceChannelMode, PriceChannels};

#[derive(Debug, Clone)]
pub struct PriceChannelWidth {
    pc: PriceChannels,
    value: f64,
}

impl Default for PriceChannelWidth {
    fn default() -> Self {
        Self::new(14)
    }
}

impl PriceChannelWidth {
    pub fn new(period: usize) -> Self {
        Self {
            pc: PriceChannels::from_smoothers(period.max(2), PriceChannelMode::Raw, None),
            value: 0.0,
        }
    }

    /// Feed HIGH, LOW lanes (const SOURCE order: [High, Low]).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let (upper, _mid, lower) = self.pc.feed(lanes);
        self.value = upper - lower;
        self.value
    }

    /// Legacy bar-level update.

    #[inline] pub fn reset(&mut self) { self.pc.reset(); self.value = 0.0; }
    #[inline] pub fn is_ready(&self) -> bool { self.pc.is_ready() }
    #[inline] pub fn value(&self) -> f64 { self.value }
}

// ─── Contract ────────────────────────────────────────────────────────────────

use crate::contract::Param;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Port, SourceAxis, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::{Color, Render, RenderSpec};

/// Typed config for [`PriceChannelWidth`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PriceChannelWidthConfig {
    pub period: Param<usize>,
}

impl Indicator for PriceChannelWidth {
    const ID: IndicatorId = IndicatorId::Pchwidth;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High, OhlcvField::Low,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[],
        inner: &[Port::new(IndicatorId::Pricechan, &[
            IndicatorOutputId::PricechanUpper,
            IndicatorOutputId::PricechanLower,
        ])],
    };
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Pchwidth)];
    type Config = PriceChannelWidthConfig;
    type Runtime = Self;

    fn create(cfg: PriceChannelWidthConfig) -> Self {
        PriceChannelWidth::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for PriceChannelWidthConfig {
    fn defaults() -> Self {
        PriceChannelWidthConfig { period: Param::Solo(14) }
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


impl Render for PriceChannelWidth {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Pchwidth, "Price Chan Width", Color::hex(0xF44336))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_channel_width_creation() {
        let pcw = PriceChannelWidth::new(20);
        assert!(!pcw.is_ready());
        assert_eq!(pcw.value(), 0.0);
    }

    #[test]
    fn test_price_channel_width_warmup() {
        let mut pcw = PriceChannelWidth::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pcw.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(pcw.is_ready());
    }

    #[test]
    fn test_price_channel_width_positive() {
        let mut pcw = PriceChannelWidth::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pcw.feed(&[price + 2.0, price - 2.0, price]);
            assert!(value >= 0.0, "Width should be non-negative");
        }
    }

    #[test]
    fn test_price_channel_width_reset() {
        let mut pcw = PriceChannelWidth::new(20);
        for i in 0..25 {
            pcw.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        pcw.reset();
        assert!(!pcw.is_ready());
        assert_eq!(pcw.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_pchwidth() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<PriceChannelWidth as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Pchwidth(cfg).build_solo().unwrap();
        for i in 0..25 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: price + 2.0, low: price - 2.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.primary() >= 0.0);
    }
}
