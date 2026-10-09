// Price Channel Oscillator - normalized position within Price Channels mapped to [-1, 1]

use crate::indicators::channels::price_channels::{PriceChannelMode, PriceChannels};

#[derive(Debug, Clone)]
pub struct PriceChannelOscillator {
    channels: PriceChannels,
    value: f64,
}

impl Default for PriceChannelOscillator {
    fn default() -> Self {
        Self::new(14)
    }
}

impl PriceChannelOscillator {
    pub fn new(period: usize) -> Self {
        Self {
            channels: PriceChannels::from_smoothers(period.max(2), PriceChannelMode::Raw, None),
            value: 0.0,
        }
    }

    /// Feed HIGH, LOW, CLOSE lanes (const SOURCE order: [High, Low, Close]).
    /// PriceChannels only consumes [H, L]; close is taken separately for position.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let close = lanes[2];
        let (_u, _m, _d) = self.channels.feed(&lanes[..2]);
        let pos = self.channels.position_in_channel(close);
        self.value = 2.0 * pos - 1.0;
        self.value
    }

    /// Legacy bar-level update.

    #[inline] pub fn reset(&mut self) { self.channels.reset(); self.value = 0.0; }
    #[inline] pub fn is_ready(&self) -> bool { self.channels.is_ready() }
    #[inline] pub fn value(&self) -> f64 { self.value }
}

// ─── Contract ────────────────────────────────────────────────────────────────

use crate::contract::Param;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Port, SourceAxis, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::{Color, Render, ReferenceLine, RenderSpec};

/// Typed config for [`PriceChannelOscillator`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct PriceChannelOscillatorConfig {
    pub period: Param<usize>,
}

impl Indicator for PriceChannelOscillator {
    const ID: IndicatorId = IndicatorId::Pchosc;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High, OhlcvField::Low, OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[],
        inner: &[Port::new(IndicatorId::Pricechan, &[
            IndicatorOutputId::PricechanUpper,
            IndicatorOutputId::PricechanMiddle,
            IndicatorOutputId::PricechanLower,
        ])],
    };
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Pchosc)];
    type Config = PriceChannelOscillatorConfig;
    type Runtime = Self;

    fn create(cfg: PriceChannelOscillatorConfig) -> Self {
        PriceChannelOscillator::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for PriceChannelOscillatorConfig {
    fn defaults() -> Self {
        PriceChannelOscillatorConfig { period: Param::Solo(14) }
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


impl Render for PriceChannelOscillator {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Pchosc, "Price Channel Osc", Color::hex(0x9C27B0))
            .bounds(-1.0, 1.0)
            .reference_line(ReferenceLine::new(0.0, Color::hex(0x9E9E9E)))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_channel_oscillator_creation() {
        let pco = PriceChannelOscillator::new(20);
        assert!(!pco.is_ready());
        assert_eq!(pco.value(), 0.0);
    }

    #[test]
    fn test_price_channel_oscillator_warmup() {
        let mut pco = PriceChannelOscillator::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pco.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(pco.is_ready());
    }

    #[test]
    fn test_price_channel_oscillator_range() {
        let mut pco = PriceChannelOscillator::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pco.feed(&[price + 1.0, price - 1.0, price]);
            assert!(value >= -1.0 && value <= 1.0, "Oscillator should be in [-1, 1]");
        }
    }

    #[test]
    fn test_price_channel_oscillator_reset() {
        let mut pco = PriceChannelOscillator::new(20);
        for i in 0..25 {
            pco.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        pco.reset();
        assert!(!pco.is_ready());
        assert_eq!(pco.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_pchosc() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<PriceChannelOscillator as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Pchosc(cfg).build_solo().unwrap();
        for i in 0..25 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: price + 1.0, low: price - 1.0, close: price, volume: 1000.0,
            });
        }
        let v = f.primary();
        assert!(v >= -1.0 && v <= 1.0, "oscillator out of range: {v}");
    }
}
