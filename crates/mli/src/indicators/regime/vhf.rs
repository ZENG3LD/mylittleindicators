// High-performance Vertical Horizontal Filter (VHF)
// (c) 2024

#[derive(Debug, Clone)]
pub struct Vhf {
    period: usize,
    buffer: Vec<f64>,
    filled: bool,
    value: f64,
}

impl Vhf {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: Vec::with_capacity(period),
            filled: false,
            value: 0.0,
        }
    }

    /// Feed a single pre-extracted price scalar (const SOURCE = Field{Close}).
    pub fn feed(&mut self, value: f64) -> f64 {
        if self.buffer.len() < self.period {
            self.buffer.push(value);
        } else {
            self.buffer.remove(0);
            self.buffer.push(value);
            self.filled = true;
        }
        if self.buffer.len() < self.period {
            self.value = 0.0;
            return self.value;
        }
        let (min, max) = self.buffer.iter().copied()
            .fold((f64::INFINITY, f64::NEG_INFINITY),
                  |(mn, mx), val| (mn.min(val), mx.max(val)));
        let mut sum = 0.0;
        for i in 1..self.period {
            sum += (self.buffer[i] - self.buffer[i - 1]).abs();
        }
        self.value = if sum.abs() < 1e-12 { 0.0 } else { (max - min) / sum };
        self.value
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.filled = false;
        self.value = 0.0;
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

impl Default for Vhf {
    /// Factory default: period = 14, source = `OhlcvField::Close`.
    fn default() -> Self {
        Self::new(14)
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed contract config for [`Vhf`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VhfConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl Indicator for Vhf {
    const ID: IndicatorId = IndicatorId::Vhf;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Vhf)];
    /// O(period): VHF rescans the full window to compute min/max and sum of abs-diffs.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    type Config = VhfConfig;
    type Runtime = Vhf;

    fn create(cfg: VhfConfig) -> Vhf {
        Vhf::new(cfg.period.resolved())
    }

    fn source_fields(cfg: &VhfConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for VhfConfig {
    fn defaults() -> Self {
        VhfConfig {
            period: Param::Solo(14),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn machine_defaults() -> Self {
        // period: Class A usize — auto range(2,4048,1).
        // source: Class O OhlcvField — auto all 7 price variants.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Vhf {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vhf, "VHF", Color::hex(0x009688))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn test_vhf_creation() {
        let vhf = Vhf::new(14);
        assert!(!vhf.is_ready());
        assert_eq!(vhf.value(), 0.0);
        assert_eq!(vhf.period(), 14);
    }

    #[test]
    fn test_vhf_trending() {
        let mut vhf = Vhf::new(14);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            vhf.feed(price);
        }
        assert!(vhf.is_ready());
        assert!(vhf.value() > 0.0, "VHF should be > 0 in trending market, got {}", vhf.value());
    }

    #[test]
    fn test_vhf_finite_values() {
        let mut vhf = Vhf::new(14);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = vhf.feed(price);
            assert!(value.is_finite(), "VHF should always be finite");
            assert!(value >= 0.0, "VHF should be non-negative, got {}", value);
        }
    }

    #[test]
    fn test_vhf_reset() {
        let mut vhf = Vhf::new(14);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            vhf.feed(price);
        }
        assert!(vhf.is_ready());
        vhf.reset();
        assert!(!vhf.is_ready());
        assert_eq!(vhf.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_vhf() {
        let mut f = IndicatorOrder::Vhf(VhfConfig {
            period: Param::Solo(14),
            source: Param::Solo(OhlcvField::Close),
        })
        .build_solo()
        .unwrap();
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary() > 0.0);
    }
}
