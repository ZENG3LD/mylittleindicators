//! Relative Strength Index (RSI) indicator.

use crate::engine::contract_engine::{SmootherId, SmootherSlot};
use crate::engine::ohlcv_field::OhlcvField;

/// RSI calculation mode for backward compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RsiMode {
    /// SMA-based RSI (Cutler's RSI)
    Classic,
    /// RMA-based RSI (Wilder's original)
    Wilder,
}

/// Relative Strength Index (RSI) - momentum oscillator measuring speed and change of price movements.
///
/// RSI = 100 - 100 / (1 + RS)
///
/// where RS = Average Gain / Average Loss over the specified period.
///
/// RSI oscillates between 0 and 100. Traditional interpretation:
/// - Above 70: Overbought condition
/// - Below 30: Oversold condition
///
/// # Parameters
/// - `period`: Lookback period (default: 14)
/// - `ma_type`: Moving average type for smoothing gains/losses
///
/// # Implementation
///
/// Uses configurable moving average for gain/loss smoothing. O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Rsi {
    period: usize,
    prev: f64,
    has_inputs: bool,
    gain_ma: SmootherSlot,
    loss_ma: SmootherSlot,
    value: f64,
    count: usize,
    initialized: bool,
}

impl Rsi {
    /// Creates a new RSI with the specified period using Wilder's RMA smoothing.
    ///
    /// # Arguments
    /// * `period` - Lookback period (typically 14)
    /// Default ctor — gains/losses smoothed with RMA (Wilder) over close.
    pub fn new(period: usize) -> Self {
        Self::from_smoother(period, SmootherId::Rma)
    }

    /// Build with legacy mode selection (Classic = SMA, Wilder = RMA).
    pub fn with_mode(period: usize, mode: RsiMode) -> Self {
        let sid = match mode {
            RsiMode::Classic => SmootherId::Sma,
            RsiMode::Wilder => SmootherId::Rma,
        };
        Self::from_smoother(period, sid)
    }

    /// Feed ONE resolved scalar (the configured source field). The factory extracts
    /// `cfg.source` from the bar; the core knows no transport. Returns RSI 0-100.
    pub fn feed(&mut self, value: f64) -> f64 {
        if !self.has_inputs {
            self.prev = value;
            self.has_inputs = true;
            return self.value;
        }

        let diff = value - self.prev;
        let gain = if diff > 0.0 { diff } else { 0.0 };
        let loss = if diff < 0.0 { -diff } else { 0.0 };
        self.prev = value;
        self.count += 1;

        let _gain_value = self.gain_ma.feed(gain);
        let _loss_value = self.loss_ma.feed(loss);

        if self.gain_ma.is_ready() && self.loss_ma.is_ready() {
            self.initialized = true;
            let avg_gain = self.gain_ma.value();
            let avg_loss = self.loss_ma.value();

            if avg_loss.abs() < 1e-12 {
                self.value = 100.0; // 100 if no losses
            } else {
                let rs = avg_gain / avg_loss;
                self.value = 100.0 * (1.0 - (1.0 / (1.0 + rs)));
            }
        }

        self.value
    }

    /// Returns the current RSI value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the current RSI value as `f64` (the scalar-slot interface).
    #[inline]
    pub fn value_f64(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the RSI has received enough bars to produce a valid value.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.initialized
    }

    /// Resets the RSI to its initial state.
    pub fn reset(&mut self) {
        self.prev = 0.0;
        self.has_inputs = false;
        self.gain_ma.reset();
        self.loss_ma.reset();
        self.value = 0.0;
        self.count = 0;
        self.initialized = false;
    }

    /// Returns the number of bars processed.
    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns the period of this RSI.
    #[inline]
    pub fn period(&self) -> usize {
        self.period
    }
}

impl crate::contract::Oscillator for Rsi {
    type Params = crate::indicators::average::moving_average::PeriodConfig;
    fn from_params(p: Self::Params) -> Self {
        Rsi::new(p.period)
    }
    fn params_period(p: &Self::Params) -> usize {
        p.period
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::contract_engine::SmootherChoice;
use crate::contract::{Cost, Family, Slot, Indicator, Output, UpdateComplexity};
use crate::contract::Param;
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderSpec};
use crate::engine::stream_kind::StreamKind;

impl Rsi {
    /// Build an RSI from a smoother CHOICE (kind + follow/own period) at the host `period`.
    /// Both gain/loss smoothers are the same choice; `Follow` resolves to `period`.
    pub fn from_choice(choice: SmootherChoice, period: usize) -> Self {
        Self {
            period,
            prev: 0.0,
            has_inputs: false,
            gain_ma: choice.build(period),
            loss_ma: choice.build(period),
            value: 0.0,
            count: 0,
            initialized: false,
        }
    }

    /// Build an RSI whose two gain/loss smoothers are the given [`SmootherId`] member.
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        Self {
            period,
            prev: 0.0,
            has_inputs: false,
            gain_ma: SmootherSlot::new(smoother, period),
            loss_ma: SmootherSlot::new(smoother, period),
            value: 0.0,
            count: 0,
            initialized: false,
        }
    }
}

/// Typed config for [`Rsi`]: period + source + the gain/loss smoother CHOICE.
///
/// Dual-mode: every field is a `Param` — `Solo` = one value, `Many` = a swept set. The
/// `#[derive(ConfigAxes)]` reads them for `cube_size`/`iter`. The `#[slot]` smoother is a
/// `Param<SmootherChoice>`: which kind + follow/own period.
///
/// `period`, `source`, and the smoother KIND are three orthogonal sweep axes. The smoother's
/// PERIOD is NOT a separate axis: by default the choice is `Follow` (the gain/loss smoother runs
/// at RSI's own `period`); an `Own(p)` choice overrides it ("sometimes interesting"). So a
/// `Follow` slot rides the `period` axis — no duplication, no dead axis.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct RsiConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    #[slot]
    pub smoother: Param<SmootherChoice>,
}

impl Indicator for Rsi {
    const ID: IndicatorId = IndicatorId::Rsi;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// RSI's OWN state is O(1) (the running gain/loss diff). Its two gain/loss
    /// smoothers share ONE config knob — one `#[slot]` field, one cost entry.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = RsiConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Rsi)];

    type Config = RsiConfig;
    type Runtime = Rsi;

    fn create(cfg: RsiConfig) -> Rsi {
        Rsi::from_choice(cfg.smoother.resolved(), cfg.period.resolved())
    }

    fn slot_members(cfg: &RsiConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the
    /// resolved scalar — the core ingests via `feed(f64)`, knowing no OHLCV fields.
    fn source_fields(cfg: &RsiConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for RsiConfig {
    /// The cold-start default — every axis `Solo`. Standard RSI: period 14, close source,
    /// Wilder's RMA smoothing that FOLLOWS the period.
    fn defaults() -> Self {
        RsiConfig {
            period: Param::Solo(14),
            source: Param::Solo(OhlcvField::Close),
            smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1); source: Class O → auto all-8.
        // smoother: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for Rsi {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Rsi, "RSI", Color::hex(0x9C27B0))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(70.0, Color::hex(0xFF5722)))
            .reference_line(ReferenceLine::new(30.0, Color::hex(0x4CAF50)))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_rsi_basic_calculation() {
        let mut rsi = Rsi::new(14);

        // Feed uptrend data - RSI should be high
        for i in 1..=20 {
            rsi.feed(100.0 + i as f64);
        }

        assert!(rsi.is_ready());
        let val = rsi.value();
        assert!(val > 50.0, "RSI in uptrend should be above 50, got {}", val);
    }

    #[test]
    fn test_rsi_downtrend() {
        let mut rsi = Rsi::new(14);

        // Feed downtrend data - RSI should be low
        for i in 1..=20 {
            rsi.feed(200.0 - i as f64);
        }

        assert!(rsi.is_ready());
        let val = rsi.value();
        assert!(val < 50.0, "RSI in downtrend should be below 50, got {}", val);
    }

    #[test]
    fn test_rsi_range() {
        let mut rsi = Rsi::new(14);

        // Feed mixed data
        for i in 1..=30 {
            let price = if i % 2 == 0 { 100.0 + i as f64 } else { 100.0 - i as f64 };
            rsi.feed(price);
        }

        if rsi.is_ready() {
            let val = rsi.value();
            assert!(val >= 0.0 && val <= 100.0, "RSI should be in [0, 100], got {}", val);
        }
    }

    #[test]
    fn test_rsi_with_mode_classic() {
        let mut rsi = Rsi::with_mode(14, RsiMode::Classic);

        for i in 1..=20 {
            rsi.feed(100.0 + i as f64);
        }

        assert!(rsi.is_ready());
    }

    #[test]
    fn test_rsi_with_ma_type() {
        let mut rsi = Rsi::from_smoother(14, SmootherId::Ema);

        for i in 1..=20 {
            rsi.feed(100.0 + i as f64);
        }

        assert!(rsi.is_ready());
    }

    #[test]
    fn test_rsi_reset() {
        let mut rsi = Rsi::new(14);

        for i in 1..=20 {
            rsi.feed(100.0 + i as f64);
        }
        assert!(rsi.is_ready());

        rsi.reset();
        assert!(!rsi.is_ready());
        assert_eq!(rsi.count(), 0);
    }

    #[test]
    fn test_rsi_period_getter() {
        let rsi = Rsi::new(14);
        assert_eq!(rsi.period(), 14);

        let rsi2 = Rsi::new(21);
        assert_eq!(rsi2.period(), 21);
    }

    #[test]
    fn test_rsi_all_gains() {
        let mut rsi = Rsi::new(5);

        // Strictly increasing prices - no losses
        for i in 1..=10 {
            rsi.feed(i as f64 * 10.0);
        }

        if rsi.is_ready() {
            let val = rsi.value();
            assert!((val - 100.0).abs() < 1.0, "RSI with all gains should be ~100, got {}", val);
        }
    }

    // =========================================================================
    // Dual-mode config (the pilot): one config works as a single value AND a range.
    // =========================================================================

    #[test]
    fn config_dual_mode_cube_and_iter() {
        use crate::contract::{Config, Indicator, Param};
        use crate::engine::contract_engine::SmootherId;

        // Default (cold-start) = every field Solo → one point.
        let d = <<Rsi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(d.cube_size(), 1);
        assert_eq!(d.iter().count(), 1);
        assert!(d.period.is_solo() && d.source.is_solo() && d.smoother.is_solo());

        // Three ORTHOGONAL axes. The smoother is FOLLOW, so it has no own period axis — it rides
        // `period`:  period {5,10,15,20,25} (5) × source {Close,High} (2) × kind {Rma,Sma} (2) = 20.
        let swept = RsiConfig {
            period: Param::range(5, 25, 5),
            source: Param::many(vec![OhlcvField::Close, OhlcvField::High]),
            smoother: Param::many(vec![
                SmootherChoice::follow(SmootherId::Rma),
                SmootherChoice::follow(SmootherId::Sma),
            ]),
        };
        assert_eq!(swept.cube_size(), 20);

        // The FACTORY pattern: iter() yields N resolved CONFIGS; mapping create() over them builds
        // N instances (the indicator builds ONE each; the Vec is the caller's). Prove it here.
        let resolved: Vec<RsiConfig> = swept.iter().collect();
        assert_eq!(resolved.len(), 20);
        let instances: Vec<Rsi> = resolved.iter().map(|c| <Rsi as Indicator>::create(c.clone())).collect();
        assert_eq!(instances.len(), 20);
        for (c, mut rsi) in resolved.iter().zip(instances) {
            assert_eq!(c.cube_size(), 1, "resolved config must be a single point");
            // FOLLOW: the built RSI's (and its gain/loss smoothers') period IS the config period.
            assert_eq!(rsi.period(), c.period.resolved());
            for i in 1..=30 { rsi.feed(100.0 + i as f64); }
            assert!(rsi.is_ready());
        }
        // the period axis expanded to exactly the 5 swept values.
        let periods: std::collections::BTreeSet<usize> =
            resolved.iter().map(|c| c.period.resolved()).collect();
        assert_eq!(periods, [5, 10, 15, 20, 25].into_iter().collect());
    }

    #[test]
    fn slot_follow_vs_own_period() {
        use crate::contract::{Config, Indicator, Param, SlotPeriod};
        use crate::engine::contract_engine::SmootherId;

        // FOLLOW: smoother inherits the host period (no independent axis).
        let follow = RsiConfig {
            period: Param::Solo(10),
            source: Param::Solo(OhlcvField::Close),
            smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        };
        assert_eq!(follow.cube_size(), 1);
        let rsi = <Rsi as Indicator>::create(follow);
        assert_eq!(rsi.period(), 10, "Follow → host period 10");

        // OWN: smoother runs at its own period, independent of the host's `period`. The own sweep
        // is the OUTER Param::Many of Own orders: Own(7), Own(21) → 2 cube points.
        let own = RsiConfig {
            period: Param::Solo(10),
            source: Param::Solo(OhlcvField::Close),
            smoother: Param::many(vec![
                SmootherChoice::own(SmootherId::Rma, 7),
                SmootherChoice::own(SmootherId::Rma, 21),
            ]),
        };
        assert_eq!(own.cube_size(), 2, "own-many is the outer Param::Many of Own orders");
        // (host period 10 is unchanged across the sweep; the smoother's OWN period varies.)
        let owns: Vec<SlotPeriod> = own.iter().map(|c| c.smoother.resolved().period).collect();
        assert_eq!(owns, vec![SlotPeriod::Own(7), SlotPeriod::Own(21)]);

        // A single resolved point is EITHER Follow OR Own — never both (it is an enum). A sweep MAY
        // list both as alternatives:
        let mixed = RsiConfig {
            period: Param::Solo(10),
            source: Param::Solo(OhlcvField::Close),
            smoother: Param::many(vec![
                SmootherChoice::follow(SmootherId::Rma),
                SmootherChoice::own(SmootherId::Rma, 20),
            ]),
        };
        assert_eq!(mixed.cube_size(), 2, "follow-vs-own explored as two cube points");
    }

}
