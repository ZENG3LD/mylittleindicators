//! Percentage Price Oscillator (PPO) indicator.

use crate::engine::contract_engine::{SmootherSlot, SmootherId};
use crate::engine::ohlcv_field::OhlcvField;

/// Percentage Price Oscillator (PPO) - momentum indicator showing MA difference as percentage.
///
/// PPO Line = 100 × (Fast MA - Slow MA) / Slow MA
/// Signal Line = MA(PPO Line)
/// Histogram = PPO Line - Signal Line
///
/// Similar to MACD but expressed as a percentage, making it comparable across
/// different securities regardless of price level.
///
/// PURE core — owns no source; the factory feeds it the resolved scalar via [`Ppo::feed`].
#[derive(Debug, Clone)]
pub struct Ppo {
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    fast_ma: SmootherSlot,
    slow_ma: SmootherSlot,
    signal_ma: SmootherSlot,
    value: f64,
    signal: f64,
    ready: bool,
}

impl Ppo {
    /// Default ctor — all three MAs are EMA.
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self::from_smoother(SmootherId::Ema, fast_period, slow_period, signal_period)
    }

    /// Build all three MAs from one narrow `SmootherId`.
    /// Legacy bridge; the contract path goes through `PpoConfig`.
    pub fn from_smoother(ma: SmootherId, fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        let fast = fast_period.max(1);
        let slow = slow_period.max(1);
        let signal = signal_period.max(1);
        Self {
            fast_period: fast,
            slow_period: slow,
            signal_period: signal,
            fast_ma: SmootherSlot::new(ma, fast),
            slow_ma: SmootherSlot::new(ma, slow),
            signal_ma: SmootherSlot::new(ma, signal),
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, price: f64) -> f64 {
        self.fast_ma.feed(price);
        self.slow_ma.feed(price);
        let fast = self.fast_ma.value();
        let slow = self.slow_ma.value();
        let denom = if slow.abs() < 1e-12 { 1e-12 } else { slow };
        self.value = 100.0 * (fast - slow) / denom;
        if self.fast_ma.is_ready() && self.slow_ma.is_ready() {
            self.signal_ma.feed(self.value);
            self.signal = self.signal_ma.value();
        }
        self.ready =
            self.fast_ma.is_ready() && self.slow_ma.is_ready() && self.signal_ma.is_ready();
        self.value
    }


    /// Returns the PPO line value.
    #[inline]
    pub fn value_ppo(&self) -> f64 {
        self.value
    }

    /// Returns the signal line value.
    #[inline]
    pub fn value_signal(&self) -> f64 {
        self.signal
    }

    /// Returns the histogram value (PPO - Signal).
    #[inline]
    pub fn value_histogram(&self) -> f64 {
        self.value - self.signal
    }

    /// Brace-named getter: `line` output (PPO line = 100 * (fast - slow) / slow).
    #[inline]
    pub fn line(&self) -> f64 {
        self.value
    }

    /// Brace-named getter: `signal` output (signal line).
    #[inline]
    pub fn signal(&self) -> f64 {
        self.signal
    }

    /// Brace-named getter: `histogram` output (PPO line - signal line).
    #[inline]
    pub fn histogram(&self) -> f64 {
        self.value - self.signal
    }

    /// The fast line's own period.
    #[inline]
    pub fn fast_period(&self) -> usize {
        self.fast_period
    }

    /// The slow line's own period.
    #[inline]
    pub fn slow_period(&self) -> usize {
        self.slow_period
    }

    /// The signal line's own period.
    #[inline]
    pub fn signal_period(&self) -> usize {
        self.signal_period
    }

    /// Returns `true` if the PPO has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Resets the PPO to its initial state.
    pub fn reset(&mut self) {
        self.fast_ma.reset();
        self.slow_ma.reset();
        self.signal_ma.reset();
        self.value = 0.0;
        self.signal = 0.0;
        self.ready = false;
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Cost, Family, Param, Slot, Indicator, Output, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, HistogramStyle, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed configuration for [`Ppo`]: fast/slow/signal periods (host's own scalars), the
/// three per-line smoother choices (each carries its own period + shape), and the price
/// source. PPO is MACD-as-percentage — same shape, `Macd` output.
///
/// Dual-mode: every field is a `Param`. The `#[slot]` smoothers are `Param<SmootherChoice>`
/// with Own periods.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct PpoConfig {
    pub fast: Param<usize>,
    pub slow: Param<usize>,
    pub signal: Param<usize>,
    #[slot]
    pub fast_ma: Param<SmootherChoice>,
    #[slot]
    pub slow_ma: Param<SmootherChoice>,
    #[slot]
    pub signal_ma: Param<SmootherChoice>,
    pub source: Param<OhlcvField>,
}

impl Indicator for Ppo {
    const ID: IndicatorId = IndicatorId::Ppo;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Three outputs: line, signal, histogram.
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::PpoLine),
        Output::centered(IndicatorOutputId::PpoSignal),
        Output::centered(IndicatorOutputId::PpoHistogram),
    ];
    /// Own base ~O(1) (line/signal/histogram scalars). The three smoothers are
    /// MovingAverage SLOTS — their cost lands recursively, one per slot.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = PpoConfig::SLOTS;
    type Config = PpoConfig;
    type Runtime = Ppo;

    fn create(cfg: PpoConfig) -> Ppo {
        let fast_period = cfg.fast.resolved();
        let slow_period = cfg.slow.resolved();
        let signal_period = cfg.signal.resolved();
        let fast_choice = cfg.fast_ma.resolved();
        let slow_choice = cfg.slow_ma.resolved();
        let signal_choice = cfg.signal_ma.resolved();
        // Each smoother follows its lane's period field; the slot chooses only the kind.
        Ppo {
            fast_period,
            slow_period,
            signal_period,
            fast_ma: fast_choice.build(fast_period),
            slow_ma: slow_choice.build(slow_period),
            signal_ma: signal_choice.build(signal_period),
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    /// Single-source core: the factory variant holds the resolved `Source` and feeds the scalar.
    fn source_fields(cfg: &PpoConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &PpoConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for PpoConfig {
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast.resolved();
        let slow = self.slow.resolved();
        if fast >= slow {
            return Err(format!("fast({fast}) >= slow({slow})"));
        }
        Ok(())
    }
    /// Classic PPO: 12/26/9, all-EMA; the smoothers follow the period fields, over close.
    fn defaults() -> Self {
        PpoConfig {
            fast: Param::Solo(12),
            slow: Param::Solo(26),
            signal: Param::Solo(9),
            fast_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            slow_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            signal_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // fast/slow/signal: Class A → auto range(2,4048,1) EACH, but fast and slow then both
        // resolve to the same min (2), failing this config's OWN `valid_params` (fast < slow)
        // at the min corner (2026-07-03 fix). Split into disjoint ranges so `resolved()` stays
        // ordered; signal (an independent lane) keeps the full auto range.
        // source: Class O → auto all-8.
        // fast_ma/slow_ma/signal_ma: #[slot] SmootherChoice → left Solo (deferred wave).
        let mut s = Self::machine_defaults_auto();
        s.fast = Param::range(1, 100, 1);
        s.slow = Param::range(101, 10000, 1);
        s
    }
}


impl Render for Ppo {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::PpoLine, "PPO", Color::hex(0x2196F3), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::PpoSignal, "Signal", Color::hex(0xFF9800), 1.0))
            .output(RenderOutput::histogram(IndicatorOutputId::PpoHistogram, "Histogram", Color::hex(0x9E9E9E)))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ppo_basic_calculation() {
        let mut ppo = Ppo::new(12, 26, 9);
        for i in 1..=50 {
            ppo.feed(100.0 + i as f64);
        }
        assert!(ppo.is_ready());
        assert!(ppo.value_ppo() > 0.0, "PPO in uptrend should be positive");
    }

    #[test]
    fn test_ppo_downtrend() {
        let mut ppo = Ppo::new(12, 26, 9);
        for i in 1..=50 {
            ppo.feed(200.0 - i as f64);
        }
        assert!(ppo.is_ready());
        assert!(ppo.value_ppo() < 0.0, "PPO in downtrend should be negative");
    }

    #[test]
    fn test_ppo_constant_price() {
        let mut ppo = Ppo::new(5, 10, 3);
        for _ in 1..=30 {
            ppo.feed(100.0);
        }
        assert!(ppo.is_ready());
        assert!(ppo.value_ppo().abs() < 0.1, "PPO with constant price should be near 0");
    }

    #[test]
    fn test_ppo_histogram() {
        let mut ppo = Ppo::new(12, 26, 9);
        for i in 1..=50 {
            ppo.feed(100.0 + i as f64);
        }
        assert!(ppo.is_ready());
        let expected_histogram = ppo.value_ppo() - ppo.value_signal();
        assert!((ppo.value_histogram() - expected_histogram).abs() < 1e-10);
    }

    #[test]
    fn test_ppo_reset() {
        let mut ppo = Ppo::new(12, 26, 9);
        for i in 1..=50 {
            ppo.feed(100.0 + i as f64);
        }
        assert!(ppo.is_ready());
        ppo.reset();
        assert!(!ppo.is_ready());
        assert!(ppo.value_ppo().abs() < 1e-10);
    }

    #[test]
    fn test_ppo_with_sma_smoother() {
        let mut ppo = Ppo::from_smoother(SmootherId::Sma, 5, 10, 3);
        for i in 1..=30 {
            ppo.feed(100.0 + i as f64);
        }
        assert!(ppo.is_ready());
        assert!(ppo.value_ppo() > 0.0);
    }

    /// The `ContractFactory` variant holds the resolved source and the `Source` feed runs
    /// PPO end-to-end over the resolved scalar (not a raw OHLCV bar).
    #[test]
    fn factory_feeds_resolved_source() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Ppo(<<Ppo as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        let bar = |close: f64| MarketSample::Bar {
            open: 0.0, high: 999.0, low: 0.0, close, volume: 0.0,
        };
        for i in 1..=60 {
            f.feed(0, bar(100.0 + i as f64));
        }
        assert!(f.primary() > 0.0, "factory PPO on CLOSE uptrend should be > 0, got {}", f.primary());
    }
}
