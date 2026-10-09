// Envelope Bandwidth: (Upper - Lower) / Middle for EnvelopeChannels

use crate::indicators::channels::envelope_channels::EnvelopeChannels;

#[derive(Debug, Clone)]
pub struct EnvelopeBandwidth {
    env: EnvelopeChannels,
    value: f64,
}

impl Default for EnvelopeBandwidth {
    fn default() -> Self {
        Self::new(14, 2.5)
    }
}

impl EnvelopeBandwidth {
    pub fn new(period: usize, pct: f64) -> Self {
        use crate::indicators::channels::envelope_channels::EnvelopeMode;
        use crate::engine::contract_engine::SmootherId;
        Self {
            env: EnvelopeChannels::new(
                period.max(1),
                pct.max(0.01),
                EnvelopeMode::Fixed,
                SmootherId::Sma,
            ),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.env.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.env.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
    /// Feed one price scalar (close); returns bandwidth.
    pub fn feed(&mut self, price: f64) -> f64 {
        let (upper, middle, lower) = self.env.feed(price);
        let width = (upper - lower).abs();
        self.value = if middle.abs() > 1e-12 {
            width / middle.abs()
        } else {
            0.0
        };
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
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`EnvelopeBandwidth`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct EnvelopeBandwidthConfig {
    pub period: Param<usize>,
    pub pct: Param<f64>,
}

impl Indicator for EnvelopeBandwidth {
    const ID: IndicatorId = IndicatorId::Envbw;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Close is the sole price source (EnvelopeChannels uses close by default).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Envelope, &[
            IndicatorOutputId::EnvelopeUpper,
            IndicatorOutputId::EnvelopeMiddle,
            IndicatorOutputId::EnvelopeLower,
        ])],
    };
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Envbw)];
    type Config = EnvelopeBandwidthConfig;
    type Runtime = EnvelopeBandwidth;

    fn create(cfg: EnvelopeBandwidthConfig) -> EnvelopeBandwidth {
        EnvelopeBandwidth::new(cfg.period.resolved(), cfg.pct.resolved())
    }

    fn source_fields(cfg: &EnvelopeBandwidthConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for EnvelopeBandwidthConfig {
    fn defaults() -> Self {
        EnvelopeBandwidthConfig {
            period: Param::Solo(14),
            pct: Param::Solo(2.5),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1)
        s.pct = Param::many(sweep_f64(0.0, 1.0, 0.05)); // Class D ratio/percentage
        s
    }
}


impl Render for EnvelopeBandwidth {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Envbw, "Envelope BW", Color::hex(0x009688))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_envbw() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<EnvelopeBandwidth as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Envbw(cfg).build_solo().unwrap();
        for i in 1..=25usize {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,  // SOURCE = Close
                volume: 9999.0,
            });
        }
        let v = f.primary();
        assert!(v >= 0.0, "envelope bandwidth should be >= 0, got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_bandwidth_creation() {
        let eb = EnvelopeBandwidth::new(20, 2.5);
        assert!(!eb.is_ready());
        assert_eq!(eb.value(), 0.0);
    }

    #[test]
    fn test_envelope_bandwidth_warmup() {
        let mut eb = EnvelopeBandwidth::new(20, 2.5);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            eb.feed(price);
        }
        assert!(eb.is_ready());
    }

    #[test]
    fn test_envelope_bandwidth_positive() {
        let mut eb = EnvelopeBandwidth::new(20, 2.5);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = eb.feed(price);
            assert!(value >= 0.0, "Bandwidth should be non-negative");
        }
    }

    #[test]
    fn test_envelope_bandwidth_reset() {
        let mut eb = EnvelopeBandwidth::new(20, 2.5);
        for i in 0..25 {
            eb.feed(100.0 + i as f64);
        }
        eb.reset();
        assert!(!eb.is_ready());
        assert_eq!(eb.value(), 0.0);
    }
}
