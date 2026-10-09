//! Force Index (FI) — Elder's force-of-buyers/sellers measure.
//! Force Index = Volume × (Close - Previous Close), smoothed.

use crate::engine::contract_engine::{SmootherSlot, SmootherId};

/// Force Index indicator — momentum×volume product, smoothed.
#[derive(Debug, Clone)]
pub struct ForceIndex {
    prev_close: f64,
    smoothed_force: SmootherSlot,
    force_raw: f64,
    force_smooth: f64,
    bars_count: usize,
    is_ready: bool,
}

impl Default for ForceIndex {
    /// Factory default: EMA period = 13.
    fn default() -> Self {
        Self::from_smoothers(SmootherId::Ema, 13)
    }
}

impl ForceIndex {
    pub fn new() -> Self {
        Self::from_smoothers(SmootherId::Ema, 1)
    }

    pub fn with_ema(ema_period: usize) -> Self {
        Self::from_smoothers(SmootherId::Ema, ema_period)
    }

    pub fn from_smoothers(id: SmootherId, smoothing_period: usize) -> Self {
        let p = smoothing_period.max(1);
        Self {
            prev_close: 0.0,
            smoothed_force: SmootherSlot::new(id, p),
            force_raw: 0.0,
            force_smooth: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// lanes = [close, volume]
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let close  = lanes[0];
        let volume = lanes[1];
        self.bars_count += 1;

        if self.bars_count == 1 {
            self.prev_close = close;
            return self.force_smooth;
        }

        let price_change = close - self.prev_close;
        self.force_raw = volume * price_change;
        self.force_smooth = self.smoothed_force.feed(self.force_raw);
        self.prev_close = close;

        if self.bars_count >= 2 {
            self.is_ready = true;
        }

        self.force_smooth
    }

    pub fn value(&self) -> f64 {
        self.force_smooth
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    pub fn reset(&mut self) {
        self.smoothed_force.reset();
        self.prev_close = 0.0;
        self.force_raw = 0.0;
        self.force_smooth = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Config for [`ForceIndex`] — period field + single smoother slot. Default: EMA(13).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct ForceIndexConfig {
    pub period: Param<usize>,
    #[slot]
    pub ma: Param<SmootherSlotOrder>,
}

impl Indicator for ForceIndex {
    const ID: IndicatorId = IndicatorId::Fi;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed: Close then Volume. FI = Volume × ΔClose — both are required, non-configurable.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = ForceIndexConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Fi)];
    type Config = ForceIndexConfig;
    type Runtime = ForceIndex;

    fn create(cfg: ForceIndexConfig) -> ForceIndex {
        ForceIndex {
            prev_close: 0.0,
            smoothed_force: cfg.ma.resolved().into_slot(),
            force_raw: 0.0,
            force_smooth: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    fn slot_members(cfg: &ForceIndexConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for ForceIndexConfig {
    fn defaults() -> Self {
        ForceIndexConfig {
            period: Param::Solo(13),
            ma: Param::Solo(SmootherSlotOrder::Ema(PeriodConfig { period: 13 })),
        }
    }
    fn machine_defaults() -> Self {
        // period: Class A (EMA smoothing window) → auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for ForceIndex {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Fi, "Force Index", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn test_force_index_creation() {
        let fi = ForceIndex::new();
        assert!(!fi.is_ready());
        assert_eq!(fi.value(), 0.0);
    }

    #[test]
    fn test_force_index_with_ema() {
        let fi = ForceIndex::with_ema(13);
        assert!(!fi.is_ready());
    }

    #[test]
    fn test_force_index_warmup() {
        let mut fi = ForceIndex::new();
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            fi.feed(&[price, 1000.0]);
        }
        assert!(fi.is_ready());
    }

    #[test]
    fn test_force_index_values_finite() {
        let mut fi = ForceIndex::with_ema(13);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = fi.feed(&[price, 1000.0]);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_force_index_reset() {
        let mut fi = ForceIndex::new();
        for i in 0..10 {
            fi.feed(&[100.0 + i as f64, 1000.0]);
        }
        fi.reset();
        assert!(!fi.is_ready());
        assert_eq!(fi.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_fi() {
        let mut f = IndicatorOrder::Fi(<<ForceIndex as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 1000.0,
            });
        }
        assert!(f.read(IndicatorOutputId::Fi).is_finite());
    }
}
