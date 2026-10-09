//! RelativePosition primitive — emits trend state (+1/-1) based on subject
//! vs reference comparison. Holds last non-zero state across bars (does not
//! reset to 0 between transitions, unlike Crossover).
//!
//! This is the "MaCross-style" output: persistent ±1 trend label rather
//! than a one-shot event. Crossover and RelativePosition are complementary:
//! - Crossover emits ±1 only on the crossing bar (event semantics)
//! - RelativePosition emits ±1 continuously while the relation holds (state semantics)
//!
//! Replaces hardcoded MaCross (fast vs slow MA), SSL channel direction, and
//! any "is X above Y right now" question.

use crate::engine::contract_engine::SmootherSlot;
use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{SignalKind, TrendSub};

#[derive(Clone)]
pub struct RelativePosition {
    /// Config-chosen subject MA (e.g. a fast EMA), fed the close. Box-free typed slot.
    subject: SmootherSlot,
    /// Config-chosen reference MA (e.g. a slow SMA), fed the close.
    reference: SmootherSlot,
    /// Last non-zero state (sticky — holds across flat bars where subject == reference).
    last_trend: i8,
    ready: bool,
}

impl std::fmt::Debug for RelativePosition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RelativePosition")
            .field("last_trend", &self.last_trend)
            .field("ready", &self.ready)
            .finish()
    }
}

impl RelativePosition {
    pub fn new(subject: SmootherSlot, reference: SmootherSlot) -> Self {
        Self {
            subject,
            reference,
            last_trend: 0,
            ready: false,
        }
    }

    pub fn feed(&mut self, lanes: &[f64]) {
        let close = lanes[3];
        // Both MA slots are fed the close scalar.
        let s = self.subject.feed(close);
        let r = self.reference.feed(close);

        if self.subject.is_ready() && self.reference.is_ready() {
            let new_trend = if s > r { 1 } else if s < r { -1 } else { 0 };
            // Sticky: only overwrite when new_trend is decisive (±1) AND differs.
            if new_trend != 0 && new_trend != self.last_trend {
                self.last_trend = new_trend;
            }
            self.ready = true;
        }
    }

    pub fn value(&self) -> f64 {
        (self.last_trend) as f64
    }

    /// Feed one bar and return a typed signal reflecting the persistent trend state.
    ///
    /// Maps to `SignalKind::Trend(TrendSub::MaCross)` — subject line position relative
    /// to reference line, maintained as sticky trend direction.
    /// Returns `None` until both inner indicators are ready.
    pub fn detect(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> Option<(SignalKind, Direction)> {
        self.feed(&[open, high, low, close, volume]);
        if !self.ready {
            return None;
        }
        match self.last_trend {
            1 => Some((SignalKind::Trend(TrendSub::MaCross), Direction::Up)),
            -1 => Some((SignalKind::Trend(TrendSub::MaCross), Direction::Down)),
            _ => None,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn reset(&mut self) {
        self.subject.reset();
        self.reference.reset();
        self.last_trend = 0;
        self.ready = false;
    }

    /// Detect relative position from pre-computed values (slice-based hot loop).
    ///
    /// `subject` and `reference` are pre-computed indicator values.
    /// Does NOT touch the inner slot(s).
    /// Maintains sticky trend state across calls.
    pub fn detect_from_values(
        &mut self,
        subject: f64,
        reference: f64,
    ) -> Option<(SignalKind, Direction)> {
        let new_trend = if subject > reference {
            1i8
        } else if subject < reference {
            -1
        } else {
            0
        };
        if new_trend != 0 && new_trend != self.last_trend {
            self.last_trend = new_trend;
        }
        self.ready = true;
        match self.last_trend {
            1 => Some((SignalKind::Trend(TrendSub::MaCross), Direction::Up)),
            -1 => Some((SignalKind::Trend(TrendSub::MaCross), Direction::Down)),
            _ => None,
        }
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice, SmootherId};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, Slot, SourceAxis, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`RelativePosition`] — subject and reference period fields + follow slots.
/// Default: subject EMA(10), reference SMA(30).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct RelativePositionConfig {
    pub subject_period: Param<usize>,
    pub reference_period: Param<usize>,
    #[slot]
    pub subject: Param<SmootherChoice>,
    #[slot]
    pub reference: Param<SmootherChoice>,
}

impl Indicator for RelativePosition {
    const ID: IndicatorId = IndicatorId::RelPosition;
    /// No family — a composite DETECTOR over two config-chosen MA slots, consumed by name.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Full O/H/L/C/V — both MA slots are fed the close.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    /// O(1) outer — compare two scalar outputs; both MAs are priced through their slots.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = RelativePositionConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::RelPosition)];
    type Config = RelativePositionConfig;
    type Runtime = RelativePosition;

    fn create(cfg: RelativePositionConfig) -> RelativePosition {
        let subject_period = cfg.subject_period.resolved();
        let reference_period = cfg.reference_period.resolved();
        RelativePosition::new(
            cfg.subject.resolved().build(subject_period),
            cfg.reference.resolved().build(reference_period),
        )
    }

    fn slot_members(cfg: &RelativePositionConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for RelativePositionConfig {
    fn defaults() -> Self {
        RelativePositionConfig {
            subject_period: Param::Solo(10),
            reference_period: Param::Solo(30),
            subject: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            reference: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn machine_defaults() -> Self {
        // subject_period / reference_period: Class A usize — auto range(2,4048,1).
        // subject / reference (#[slot] SmootherChoice): deferred wave — leave Solo.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for RelativePosition {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::RelPosition, "Relative Position", Color::hex(0x4CAF50))
            .bounds(-1.0, 1.0)
            .zero_baseline()
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::{IndicatorOrder, SmootherChoice};

    fn sma(period: usize) -> SmootherSlot {
        SmootherChoice::follow(SmootherId::Sma).build(period)
    }

    fn ema(period: usize) -> SmootherSlot {
        SmootherChoice::follow(SmootherId::Ema).build(period)
    }

    #[test]
    fn uptrend_state_holds_plus_one() {
        let mut rp = RelativePosition::new(sma(5), sma(20));
        for i in 1..=50 {
            let p = 100.0 + i as f64 * 2.0;
            let _ = rp.feed(&[p, p, p, p, 0.0]);
        }
        assert!(rp.is_ready());
        assert_eq!(rp.value(), 1.0);
    }

    #[test]
    fn downtrend_state_holds_minus_one() {
        let mut rp = RelativePosition::new(sma(5), sma(20));
        for i in 1..=50 {
            let p = 200.0 - i as f64 * 2.0;
            let _ = rp.feed(&[p, p, p, p, 0.0]);
        }
        assert!(rp.is_ready());
        assert_eq!(rp.value(), -1.0);
    }

    #[test]
    fn parity_with_legacy_macross_uptrend() {
        // Legacy MaCross::test_ma_cross_uptrend behaviour: 40 bars of price 100+2i.
        let mut rp = RelativePosition::new(ema(9), ema(21));
        for i in 1..=40 {
            let p = 100.0 + i as f64 * 2.0;
            let _ = rp.feed(&[p, p + 1.0, p - 1.0, p, 1000.0]);
        }
        assert!(rp.is_ready());
        assert_eq!(rp.value(), 1.0, "uptrend → +1 like legacy MaCross");
    }

    #[test]
    fn parity_with_legacy_macross_downtrend() {
        let mut rp = RelativePosition::new(ema(9), ema(21));
        for i in 1..=40 {
            let p = 200.0 - i as f64 * 2.0;
            let _ = rp.feed(&[p, p + 1.0, p - 1.0, p, 1000.0]);
        }
        assert_eq!(rp.value(), -1.0);
    }

    #[test]
    fn state_sticks_across_oscillation() {
        // Oscillating price — once trend established, sticky on small flips.
        let mut rp = RelativePosition::new(sma(5), sma(20));
        for i in 1..=80 {
            let p = 100.0 + (i as f64 * 0.5).sin() * 8.0;
            let _ = rp.feed(&[p, p, p, p, 0.0]);
        }
        assert!(rp.is_ready());
        let v = rp.value();
        assert!(v == 1.0 || v == -1.0, "sticky sign after oscillation, got {v}");
    }

    #[test]
    fn reset_clears_trend_state() {
        let mut rp = RelativePosition::new(sma(5), sma(20));
        for i in 1..=30 {
            let p = 100.0 + i as f64;
            let _ = rp.feed(&[p, p, p, p, 0.0]);
        }
        rp.reset();
        assert!(!rp.is_ready());
        assert_eq!(rp.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_relative_position() {
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::RelPosition(<<RelativePosition as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        // Rising close drives subject EMA above reference SMA; after warmup the detector
        // must emit +1 through the factory (Fields resolves O/H/L/C/V).
        for i in 0..40 {
            let p = 100.0 + i as f64 * 2.0;
            f.feed(0, MarketSample::Bar { open: p, high: p + 1.0, low: p - 1.0, close: p, volume: 1000.0 });
        }
        assert!(f.is_ready(), "RelativePosition must be ready after warmup through the factory");
        assert!(f.primary().is_finite());
    }
}
