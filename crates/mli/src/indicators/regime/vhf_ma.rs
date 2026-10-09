// VhfMa: модифицированный VHF с MA в знаменателе (аналог Nautilus, но на ArrayVec и с нашими MA)
// (c) 2024

use crate::engine::contract_engine::{SmootherId, SmootherSlot};

#[derive(Debug, Clone)]
pub struct VhfMa {
    period: usize,
    buffer: Vec<f64>,
    filled: bool,
    value: f64,
    ma: SmootherSlot,
    prev_close: Option<f64>,
}

impl VhfMa {
    pub fn new(period: usize, smoother: SmootherId) -> Self {
        Self {
            period,
            buffer: Vec::with_capacity(period),
            filled: false,
            value: 0.0,
            ma: SmootherSlot::new(smoother, period),
            prev_close: None,
        }
    }

    /// Модифицированный VHF: знаменатель — MA от abs разностей.
    /// Feeds ONE pre-extracted scalar (const SOURCE = Field{Close}).
    pub fn feed(&mut self, value: f64) -> f64 {
        if self.buffer.len() < self.period {
            self.buffer.push(value);
        } else {
            self.buffer.remove(0);
            self.buffer.push(value);
            self.filled = true;
        }
        // Обновляем MA по модулю разности
        if let Some(prev) = self.prev_close {
            let abs_diff = (value - prev).abs();
            self.ma.feed(abs_diff);
        }
        self.prev_close = Some(value);
        if self.buffer.len() < self.period || !self.filled {
            self.value = 0.0;
            return self.value;
        }
        let max = self.buffer.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min = self.buffer.iter().copied().fold(f64::INFINITY, f64::min);
        let ma_val = self.ma.value();
        if ma_val.abs() < 1e-12 {
            self.value = 0.0;
        } else {
            self.value = (max - min) / (self.period as f64 * ma_val);
        }
        self.value
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.filled && self.ma.is_ready()
    }
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.filled = false;
        self.value = 0.0;
        self.ma.reset();
        self.prev_close = None;
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

impl Default for VhfMa {
    /// Factory default: period = 14, smoother = SMA, source = Close.
    fn default() -> Self {
        Self::new(14, SmootherId::Sma)
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed contract config for [`VhfMa`] — VHF with MA-denominator.
/// Single configurable source field + one smoother slot controlling the MA shape.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct VhfMaConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl Indicator for VhfMa {
    const ID: IndicatorId = IndicatorId::VhfMa;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(period): VhfMa rescans the window for min/max; MA is O(1) through the slot.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const SLOTS: &'static [Slot] = VhfMaConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::VhfMa)];
    type Config = VhfMaConfig;
    type Runtime = VhfMa;

    fn create(cfg: VhfMaConfig) -> VhfMa {
        let period = cfg.period.resolved();
        VhfMa {
            period,
            buffer: Vec::with_capacity(period),
            filled: false,
            value: 0.0,
            ma: cfg.ma.resolved().build(period),
            prev_close: None,
        }
    }

    fn source_fields(cfg: &VhfMaConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &VhfMaConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for VhfMaConfig {
    fn defaults() -> Self {
        VhfMaConfig {
            period: Param::Solo(14),
            source: Param::Solo(OhlcvField::Close),
            ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn machine_defaults() -> Self {
        // period: Class A usize — auto range(2,4048,1).
        // source: Class O OhlcvField — auto all 7 price variants.
        // ma (#[slot] SmootherChoice): deferred wave — leave Solo.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VhfMa {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::VhfMa, "VHF MA", Color::hex(0x009688))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;

    #[test]
    fn test_vhf_ma_creation() {
        let vhf = VhfMa::new(14, SmootherId::Ema);
        assert!(!vhf.is_ready());
        assert_eq!(vhf.value(), 0.0);
        assert_eq!(vhf.period(), 14);
    }

    #[test]
    fn test_vhf_ma_trending() {
        let mut vhf = VhfMa::new(14, SmootherId::Ema);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            vhf.feed(price);
        }
        assert!(vhf.is_ready());
        assert!(vhf.value() > 0.0, "VHF-MA should be > 0 in trending market, got {}", vhf.value());
    }

    #[test]
    fn test_vhf_ma_finite_values() {
        let mut vhf = VhfMa::new(14, SmootherId::Sma);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = vhf.feed(price);
            assert!(value.is_finite(), "VHF-MA should always be finite");
            assert!(value >= 0.0, "VHF-MA should be non-negative, got {}", value);
        }
    }

    #[test]
    fn test_vhf_ma_reset() {
        let mut vhf = VhfMa::new(14, SmootherId::Ema);
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
    fn factory_feeds_resolved_vhf_ma() {
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::VhfMa(<<VhfMa as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 0.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary() > 0.0);
    }
}






















