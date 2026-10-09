// Elder Impulse System: combination of EMA slope and MACD histogram sign
//
// Implementation: EMA(ema_period) slope direction + MACD(fast=12, slow=26, signal=9) histogram sign.
// Both conditions bullish (+1) or both bearish (-1) → impulse. Mixed → neutral (0).
//
// The main trend EMA shape is user-configurable (SmootherSlot).
// MACD internals are hardcoded EMA (not exposed as slots).

use crate::engine::contract_engine::{SmootherSlot, SmootherId};
use crate::engine::ohlcv_field::OhlcvField;

#[derive(Debug, Clone)]
pub struct ElderImpulseSystem {
    ema_period: usize,
    source: OhlcvField,
    /// Main trend EMA (typically 13)
    ema: SmootherSlot,
    prev_ema: f64,
    /// MACD components: fast EMA (12), slow EMA (26), signal EMA (9) — hardcoded EMA shape
    macd_fast: SmootherSlot,
    macd_slow: SmootherSlot,
    macd_signal: SmootherSlot,
    prev_macd_hist: f64,
    macd_hist: f64,
    value: i8,
}

impl ElderImpulseSystem {
    /// Creates a new Elder Impulse System with default EMA trend MA.
    pub fn new(ema_period: usize) -> Self {
        Self::from_smoothers(SmootherId::Ema, ema_period)
    }

    /// Build from a narrow SmootherId for the main trend MA.
    pub fn from_smoothers(id: SmootherId, ema_period: usize) -> Self {
        let period = ema_period.max(2);
        Self {
            ema_period: period,
            source: OhlcvField::Close,
            ema: SmootherSlot::new(id, period),
            prev_ema: 0.0,
            macd_fast: SmootherSlot::new(SmootherId::Ema, 12),
            macd_slow: SmootherSlot::new(SmootherId::Ema, 26),
            macd_signal: SmootherSlot::new(SmootherId::Ema, 9),
            prev_macd_hist: 0.0,
            macd_hist: 0.0,
            value: 0,
        }
    }

    /// Creates a new Elder Impulse System with custom source field.
    pub fn with_source(ema_period: usize, source: OhlcvField) -> Self {
        let mut ei = Self::new(ema_period);
        ei.source = source;
        ei
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ema = SmootherSlot::new(SmootherId::Ema, self.ema_period);
        self.prev_ema = 0.0;
        self.macd_fast = SmootherSlot::new(SmootherId::Ema, 12);
        self.macd_slow = SmootherSlot::new(SmootherId::Ema, 26);
        self.macd_signal = SmootherSlot::new(SmootherId::Ema, 9);
        self.prev_macd_hist = 0.0;
        self.macd_hist = 0.0;
        self.value = 0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ema.is_ready()
    }

    #[inline]
    pub fn value(&self) -> f64 {
        (self.value) as f64
    }

    /// Feed a pre-extracted scalar — the core computation.
    pub fn feed(&mut self, price: f64) -> i8 {
        let ema_now = self.ema.feed(price);
        let ema_slope_up = ema_now > self.prev_ema + 1e-12;
        self.prev_ema = ema_now;
        // Real MACD histogram: fast_ema - slow_ema - signal_ema
        let fast = self.macd_fast.feed(price);
        let slow = self.macd_slow.feed(price);
        let macd_line = fast - slow;
        let signal_line = self.macd_signal.feed(macd_line);
        self.prev_macd_hist = self.macd_hist;
        self.macd_hist = macd_line - signal_line;
        // Positive histogram = bullish momentum; negative = bearish
        let macd_up = self.macd_hist > 0.0;
        self.value = match (ema_slope_up, macd_up) {
            (true, true) => 1,
            (false, false) => -1,
            _ => 0,
        };
        self.value
    }

    pub fn value_signal(&self) -> i8 {
        self.value
    }

    /// Returns the source field used for calculation.
    #[inline]
    pub fn get_source(&self) -> OhlcvField {
        self.source
    }

    /// Sets the source field and resets the indicator.
    pub fn set_source(&mut self, source: OhlcvField) {
        if self.source != source {
            self.source = source;
            self.reset();
        }
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec, Slot};

/// Typed contract config for [`ElderImpulseSystem`].
/// Only the main trend MA is user-configurable; MACD internals are hardcoded EMA.
///
/// Dual-mode: every field is a `Param`. The `#[slot]` trend smoother is a
/// `Param<SmootherChoice>` that follows the `period` field.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct ElderImpulseConfig {
    pub source: Param<OhlcvField>,
    pub period: Param<usize>,
    #[slot]
    pub trend_ma: Param<SmootherChoice>,
}

impl Indicator for ElderImpulseSystem {
    const ID: IndicatorId = IndicatorId::ElderImpulse;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = ElderImpulseConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::ordinal(IndicatorOutputId::ElderImpulse)];
    type Config = ElderImpulseConfig;
    type Runtime = ElderImpulseSystem;

    fn create(cfg: ElderImpulseConfig) -> ElderImpulseSystem {
        let period = cfg.period.resolved().max(2);
        ElderImpulseSystem {
            ema_period: period,
            source: cfg.source.resolved(),
            ema: cfg.trend_ma.resolved().build(period),
            prev_ema: 0.0,
            macd_fast: SmootherSlot::new(SmootherId::Ema, 12),
            macd_slow: SmootherSlot::new(SmootherId::Ema, 26),
            macd_signal: SmootherSlot::new(SmootherId::Ema, 9),
            prev_macd_hist: 0.0,
            macd_hist: 0.0,
            value: 0,
        }
    }

    fn source_fields(cfg: &ElderImpulseConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &ElderImpulseConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for ElderImpulseConfig {
    fn defaults() -> Self {
        ElderImpulseConfig {
            source: Param::Solo(OhlcvField::Close),
            period: Param::Solo(13),
            trend_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
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
        // trend_ma: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for ElderImpulseSystem {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::ElderImpulse, "Impulse", Color::hex(0x4CAF50))
            .precision(0)
            .build()
    }
}

impl Default for ElderImpulseSystem {
    fn default() -> Self {
        Self::new(13)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elder_impulse_creation() {
        let ei = ElderImpulseSystem::new(13);
        assert!(!ei.is_ready());
        assert_eq!(ei.value_signal(), 0);
    }

    #[test]
    fn test_elder_impulse_uptrend() {
        let mut ei = ElderImpulseSystem::new(10);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            ei.feed(price);
        }
        assert!(ei.is_ready());
        assert_eq!(ei.value_signal(), 1, "Elder Impulse should signal 1 in uptrend");
    }

    #[test]
    fn test_elder_impulse_downtrend() {
        let mut ei = ElderImpulseSystem::new(10);
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            ei.feed(price);
        }
        assert!(ei.is_ready());
        assert_eq!(ei.value_signal(), -1, "Elder Impulse should signal -1 in downtrend");
    }

    #[test]
    fn test_elder_impulse_reset() {
        let mut ei = ElderImpulseSystem::new(10);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            ei.feed(price);
        }
        assert!(ei.is_ready());
        ei.reset();
        assert!(!ei.is_ready());
        assert_eq!(ei.value_signal(), 0);
    }

    #[test]
    fn test_elder_impulse_signal_range() {
        let mut ei = ElderImpulseSystem::new(10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let signal = ei.feed(price);
            assert!(signal >= -1 && signal <= 1, "Elder Impulse should be -1, 0, or 1, got {}", signal);
        }
    }

    #[test]
    fn test_elder_impulse_value_types() {
        let ei = ElderImpulseSystem::new(13);
        assert_eq!(ei.value() as i8, 0);
    }

    #[test]
    fn test_elder_impulse_with_sma() {
        let mut ei = ElderImpulseSystem::from_smoothers(SmootherId::Sma, 10);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            ei.feed(price);
        }
        assert!(ei.is_ready());
        assert_eq!(ei.value_signal(), 1, "Elder Impulse with SMA should signal 1 in uptrend");
    }

    #[test]
    fn test_elder_impulse_with_source() {
        let ei = ElderImpulseSystem::with_source(10, OhlcvField::High);
        assert_eq!(ei.get_source(), OhlcvField::High);
    }

    #[test]
    fn test_elder_impulse_set_source() {
        let mut ei = ElderImpulseSystem::new(10);
        for i in 1..=15 {
            let price = 100.0 + i as f64;
            ei.feed(price);
        }
        assert!(ei.is_ready());
        ei.set_source(OhlcvField::HL2);
        assert_eq!(ei.get_source(), OhlcvField::HL2);
        assert!(!ei.is_ready());
    }

    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<ElderImpulseSystem as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::ElderImpulse(cfg).build_solo().unwrap();
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            // high=9999.0 proves only close is used
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.is_ready());
        // uptrend -> signal 1
        assert_eq!(f.primary() as i8, 1);
    }
}
