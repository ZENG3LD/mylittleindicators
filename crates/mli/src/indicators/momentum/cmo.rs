//! Chande Momentum Oscillator (CMO) indicator.

use crate::engine::contract_engine::{SmootherId, SmootherSlot};
use crate::engine::ohlcv_field::OhlcvField;

/// Chande Momentum Oscillator (CMO) - measures momentum as ratio of gains vs losses.
///
/// CMO = 100 × (Sum of Gains - Sum of Losses) / (Sum of Gains + Sum of Losses)
///
/// Developed by Tushar Chande. Unlike RSI which uses ratio of gains to losses,
/// CMO uses the difference, making it oscillate between -100 and +100.
///
/// Interpretation:
/// - CMO > 50: Strong upward momentum, overbought
/// - CMO < -50: Strong downward momentum, oversold
/// - Zero crossovers: Trend change signals
/// - CMO near 0: Neutral momentum
///
/// # Parameters
/// - `period`: Lookback period for momentum calculation
/// - `ma_type`: Type of moving average for smoothing (default RMA)
/// - `mode`: Calculation mode (Wilder or Classic)
///
/// # Implementation
///
/// Supports two modes: Wilder (uses MAs) and Classic (uses buffers).
/// O(1) per update in Wilder mode, O(period) in Classic mode.
#[derive(Debug, Clone)]
pub struct Cmo {
    period: usize,
    gain_ma: SmootherSlot,
    loss_ma: SmootherSlot,
    prev: f64,
    value: f64,
    filled: bool,
    index: usize,
}

impl Cmo {
    /// Creates a new CMO (gains/losses smoothed by a MovingAverage member, RMA default).
    pub fn new(period: usize) -> Self {
        Self::from_smoother(period, SmootherId::Rma)
    }

    /// Feed ONE pre-extracted scalar — the pure core computation. The factory resolves the
    /// configured price field and feeds the scalar; the core knows no transport.
    pub fn feed(&mut self, value: f64) -> f64 {
        if self.index == 0 && self.prev == 0.0 {
            self.prev = value;
            self.index = 1;
            return self.value;
        }
        let diff = value - self.prev;
        let (gain, loss) = if diff > 0.0 {
            (diff, 0.0)
        } else {
            (0.0, -diff)
        };
        self.prev = value;
        self.index += 1;
        let avg_gain = self.gain_ma.feed(gain);
        let avg_loss = self.loss_ma.feed(loss);
        if self.index >= self.period {
            self.filled = true;
        }
        let denom = avg_gain + avg_loss;
        self.value = if self.filled && denom.abs() >= 1e-12 {
            100.0 * (avg_gain - avg_loss) / denom
        } else {
            0.0
        };
        self.value
    }
    /// Returns the current CMO value.
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the current CMO value as `f64` (the scalar-slot interface).
    #[inline]
    pub fn value_f64(&self) -> f64 {
        self.value
    }

    /// Returns the period of this CMO.
    #[inline]
    pub fn period(&self) -> usize {
        self.period
    }

    /// Returns `true` if the CMO has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Resets the CMO to its initial state.
    pub fn reset(&mut self) {
        self.gain_ma.reset();
        self.loss_ma.reset();
        self.index = 0;
        self.filled = false;
        self.prev = 0.0;
        self.value = 0.0;
    }
}

impl crate::contract::Oscillator for Cmo {
    type Params = crate::indicators::average::moving_average::PeriodConfig;
    fn from_params(p: Self::Params) -> Self {
        Cmo::new(p.period)
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

impl Cmo {
    /// Build a CMO from a smoother CHOICE (kind + follow/own period) at the host `period`.
    /// Both gain/loss smoothers use the same choice; `Follow` resolves to `period`.
    pub fn from_choice(choice: SmootherChoice, period: usize) -> Self {
        Self {
            period,
            gain_ma: choice.build(period),
            loss_ma: choice.build(period),
            prev: 0.0,
            value: 0.0,
            filled: false,
            index: 0,
        }
    }

    /// Build a Wilder-mode CMO whose two gain/loss smoothers are the given
    /// [`SmootherId`] member. This is what the `Slot` declares — the runtime
    /// honors the chosen smoother.
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        Self {
            period,
            gain_ma: SmootherSlot::new(smoother, period),
            loss_ma: SmootherSlot::new(smoother, period),
            prev: 0.0,
            value: 0.0,
            filled: false,
            index: 0,
        }
    }
}

/// Typed config for [`Cmo`]: period + source + the gain/loss smoother CHOICE.
///
/// Dual-mode: every field is a `Param` — `Solo` = one value, `Many` = a swept set. The
/// `#[derive(ConfigAxes)]` reads them for `cube_size`/`iter`. The `#[slot]` smoother is a
/// `Param<SmootherChoice>`: which kind + follow/own period.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct CmoConfig {
    pub period: Param<usize>,
    pub source: Param<OhlcvField>,
    #[slot]
    pub smoother: Param<SmootherChoice>,
}

impl Indicator for Cmo {
    const ID: IndicatorId = IndicatorId::Cmo;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// CMO's OWN state is O(1): the running gain/loss diff. The two gain/loss smoothers
    /// share ONE config knob — one `#[slot]` field, one cost entry.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = CmoConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Cmo)];

    type Config = CmoConfig;
    type Runtime = Cmo;

    fn create(cfg: CmoConfig) -> Cmo {
        Cmo::from_choice(cfg.smoother.resolved(), cfg.period.resolved())
    }

    /// Single configurable price field — the factory resolves it and feeds the scalar.
    fn source_fields(cfg: &CmoConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &CmoConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for CmoConfig {
    fn defaults() -> Self {
        CmoConfig {
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
        // period: Class A → auto; source: Class O → auto all-8.
        // smoother: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for Cmo {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Cmo, "CMO", Color::hex(0x9C27B0))
            .bounds(-100.0, 100.0)
            .reference_line(ReferenceLine::new(50.0, Color::hex(0xFF5722)))
            .reference_line(ReferenceLine::new(-50.0, Color::hex(0x4CAF50)))
            .reference_line(ReferenceLine::new(0.0, Color::hex(0x9E9E9E)))
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
    fn test_cmo_basic_calculation() {
        let mut cmo = Cmo::new(14);

        // Feed uptrend data
        for i in 1..=30 {
            cmo.feed(100.0 + i as f64);
        }

        assert!(cmo.is_ready());
        // Pure uptrend = only gains, CMO should be near +100
        assert!(cmo.value() > 50.0, "CMO in strong uptrend should be > 50");
    }

    #[test]
    fn test_cmo_downtrend() {
        let mut cmo = Cmo::new(14);

        // Feed downtrend data
        for i in 1..=30 {
            cmo.feed(200.0 - i as f64);
        }

        assert!(cmo.is_ready());
        // Pure downtrend = only losses, CMO should be near -100
        assert!(cmo.value() < -50.0, "CMO in strong downtrend should be < -50");
    }

    #[test]
    fn test_cmo_range() {
        let mut cmo = Cmo::new(14);

        // Feed oscillating data
        for i in 1..=30 {
            let price = if i % 2 == 0 { 105.0 } else { 95.0 };
            cmo.feed(price);
        }

        assert!(cmo.is_ready());
        // CMO should be between -100 and +100
        assert!(cmo.value() >= -100.0 && cmo.value() <= 100.0);
    }

    #[test]
    fn test_cmo_constant_price() {
        let mut cmo = Cmo::new(14);

        // First bar
        cmo.feed(100.0);

        // Constant price = no change, CMO should be 0
        for _ in 1..=30 {
            cmo.feed(100.0);
        }

        assert!(cmo.is_ready());
        assert!(cmo.value().abs() < 1.0, "CMO with constant price should be near 0");
    }

    #[test]
    fn test_cmo_reset() {
        let mut cmo = Cmo::new(14);

        for i in 1..=30 {
            cmo.feed(100.0 + i as f64);
        }
        assert!(cmo.is_ready());

        cmo.reset();
        assert!(!cmo.is_ready());
        assert!(cmo.value().abs() < 1e-10);
    }

    #[test]
    fn test_cmo_with_ema() {
        let mut cmo = Cmo::from_smoother(14, SmootherId::Ema);

        for i in 1..=30 {
            cmo.feed(100.0 + i as f64);
        }

        assert!(cmo.is_ready());
        assert!(cmo.value() > 0.0);
    }

    /// The factory resolves the configured field and feeds the scalar; CMO runs end-to-end.
    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Cmo(<<Cmo as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=30 {
            f.feed(0, MarketSample::Bar {
                open: 0.0, high: 999.0, low: 0.0, close: 100.0 + i as f64, volume: 0.0,
            });
        }
        assert!(f.primary() > 50.0, "factory CMO on CLOSE uptrend should be > 50, got {}", f.primary());
    }
}


















