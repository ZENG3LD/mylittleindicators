// Detrended Synthetic Price (DSP) - simple proxy: price - MA(price)

use crate::engine::contract_engine::SmootherSlot;

/// Detrended Synthetic Price = `price - MA(price)`. PURE core — owns no source; the
/// factory feeds it the resolved scalar via [`DetrendedSyntheticPrice::feed`].
#[derive(Debug, Clone)]
pub struct DetrendedSyntheticPrice {
    period: usize,
    ma: SmootherSlot,
    value: f64,
}

impl DetrendedSyntheticPrice {
    /// Default ctor — price smoothed with SMA.
    pub fn new(period: usize) -> Self {
        Self::from_smoother(period, SmootherId::Sma)
    }

    /// Build from a narrow `SmootherId` for the smoother + period.
    /// Legacy bridge; the contract path goes through `DspConfig`.
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        let p = period.max(2);
        Self {
            period: p,
            ma: SmootherSlot::new(smoother, p),
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ma.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ma.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, value: f64) -> f64 {
        let m = self.ma.feed(value);
        self.value = value - m;
        self.value
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

impl DetrendedSyntheticPrice {
    /// Build a DSP from a smoother CHOICE (kind + follow/own period) at the host `period`.
    pub fn from_choice(choice: SmootherChoice, period: usize) -> Self {
        let p = period.max(2);
        Self {
            period: p,
            ma: choice.build(p),
            value: 0.0,
        }
    }
}

/// Typed contract config for [`DetrendedSyntheticPrice`] — `price - MA(price)`.
///
/// Dual-mode: every field is a `Param`. The `#[slot]` smoother is a `Param<SmootherChoice>`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct DspConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl Indicator for DetrendedSyntheticPrice {
    const ID: IndicatorId = IndicatorId::Dsp;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1): price minus its smoother; the smoother buffer cost lands via the slot.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = DspConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Dsp)];
    type Config = DspConfig;
    type Runtime = DetrendedSyntheticPrice;

    fn create(cfg: DspConfig) -> DetrendedSyntheticPrice {
        let period = cfg.period.resolved();
        DetrendedSyntheticPrice {
            period: period.max(2),
            ma: cfg.ma.resolved().build(period),
            value: 0.0,
        }
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the resolved scalar.
    fn source_fields(cfg: &DspConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &DspConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for DspConfig {
    fn defaults() -> Self {
        DspConfig {
            period: Param::Solo(14),
            source: Param::Solo(OhlcvField::Close),
            ma: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto; source: Class O → auto all-8.
        // ma: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for DetrendedSyntheticPrice {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Dsp, "DSP", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dsp_creation() {
        let dsp = DetrendedSyntheticPrice::new(14);
        assert!(!dsp.is_ready());
        assert_eq!(dsp.value(), 0.0);
        assert_eq!(dsp.period(), 14);
    }

    #[test]
    fn test_dsp_uptrend() {
        let mut dsp = DetrendedSyntheticPrice::new(14);
        for i in 1..=30 {
            dsp.feed(100.0 + i as f64 * 2.0);
        }
        assert!(dsp.is_ready());
        assert!(dsp.value() > 0.0, "DSP should be > 0 in uptrend, got {}", dsp.value());
    }

    #[test]
    fn test_dsp_downtrend() {
        let mut dsp = DetrendedSyntheticPrice::new(14);
        for i in 1..=30 {
            dsp.feed(200.0 - i as f64 * 2.0);
        }
        assert!(dsp.is_ready());
        assert!(dsp.value() < 0.0, "DSP should be < 0 in downtrend, got {}", dsp.value());
    }

    #[test]
    fn test_dsp_finite() {
        let mut dsp = DetrendedSyntheticPrice::from_smoother(14, SmootherId::Ema);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = dsp.feed(price);
            assert!(value.is_finite(), "DSP should always be finite");
        }
    }

    #[test]
    fn test_dsp_reset() {
        let mut dsp = DetrendedSyntheticPrice::new(14);
        for i in 1..=30 {
            dsp.feed(100.0 + i as f64);
        }
        assert!(dsp.is_ready());
        dsp.reset();
        assert!(!dsp.is_ready());
        assert_eq!(dsp.value(), 0.0);
    }

    #[test]
    fn test_dsp_contract_create() {
        let cfg = <<DetrendedSyntheticPrice as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.period.resolved(), 14);
        let mut dsp = <DetrendedSyntheticPrice as Indicator>::create(cfg);
        for i in 1..=30 {
            dsp.feed(100.0 + i as f64 * 2.0);
        }
        assert!(dsp.is_ready());
    }
}
