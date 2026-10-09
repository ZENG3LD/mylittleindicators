//! Moving Average Convergence Divergence (MACD) indicator.

use crate::engine::contract_engine::{IndicatorOutputId, SmootherSlot, SmootherId};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Slot, SourceAxis, SourceLane, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, HistogramStyle, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Moving Average Convergence Divergence (MACD) - trend-following momentum indicator.
///
/// MACD Line = Fast MA - Slow MA
/// Signal Line = MA(MACD Line)
/// Histogram = MACD Line - Signal Line
///
/// Traditional settings: 12/26/9 (fast/slow/signal periods).
///
/// # Signals
/// - MACD crossing above signal: Bullish
/// - MACD crossing below signal: Bearish
/// - Histogram expanding: Trend strengthening
/// - Histogram contracting: Trend weakening
///
/// # Implementation
///
/// PURE core: three smoother slots, O(1) update. It owns NO source fields and NO
/// `update_bar`. MACD is a genuine TWO-lane consumer — the factory resolves the two
/// configured price fields (`MacdConfig::fast_source`, `MacdConfig::slow_source`) and feeds
/// the scalar pair via [`Macd::feed`] (lane 0 = fast, lane 1 = slow). The signal line
/// smooths the internal MACD line, not an input source.
#[derive(Clone)]
pub struct Macd {
    fast_ma: SmootherSlot,
    slow_ma: SmootherSlot,
    signal_ma: SmootherSlot,
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    value: f64,
    signal: f64,
    ready: bool,
}

impl Macd {
    /// Creates a MACD with default signal period (9), all-EMA smoothers.
    ///
    /// # Arguments
    /// * `fast_period` - Fast MA period (typically 12)
    /// * `slow_period` - Slow MA period (typically 26)
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self::from_smoother(SmootherId::Ema, fast_period, slow_period, 9)
    }

    /// Creates a MACD with custom signal period, all-EMA smoothers.
    pub fn new_with_signal(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self::from_smoother(SmootherId::Ema, fast_period, slow_period, signal_period)
    }

    /// Build a MACD whose three smoother slots all use one narrow `SmootherId`.
    /// Legacy bridge for un-contracted composites; the contract path goes through
    /// `MacdConfig` (three independent smoother choices).
    pub fn from_smoother(ma: SmootherId, fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self {
            fast_ma: SmootherSlot::new(ma, fast_period),
            slow_ma: SmootherSlot::new(ma, slow_period),
            signal_ma: SmootherSlot::new(ma, signal_period),
            fast_period,
            slow_period,
            signal_period,
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    /// Feed the resolved input lanes — `lanes[0]` = fast price, `lanes[1]` = slow price.
    /// The factory resolves the two configured sources and hands the scalar pair here; the
    /// core knows nothing about OHLCV fields. Returns the MACD line value.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let fast_price = lanes[0];
        let slow_price = lanes[1];

        self.fast_ma.feed(fast_price);
        self.slow_ma.feed(slow_price);

        let fast = self.fast_ma.value();
        let slow = self.slow_ma.value();
        self.value = fast - slow;

        if self.fast_ma.is_ready() && self.slow_ma.is_ready() {
            self.signal_ma.feed(self.value);
            self.signal = self.signal_ma.value();
        }

        self.ready = self.fast_ma.is_ready() && self.slow_ma.is_ready() && self.signal_ma.is_ready();
        self.value
    }


    /// Returns the MACD line value.
    #[inline]
    pub fn value_macd(&self) -> f64 {
        self.value
    }

    /// Returns the signal line value.
    #[inline]
    pub fn value_signal(&self) -> f64 {
        self.signal
    }

    /// Returns the histogram value (MACD - Signal).
    #[inline]
    pub fn value_histogram(&self) -> f64 {
        self.value - self.signal
    }

    /// Brace-named getter: `line` output (MACD line = fast MA - slow MA).
    #[inline]
    pub fn line(&self) -> f64 {
        self.value
    }

    /// Brace-named getter: `signal` output (signal line).
    #[inline]
    pub fn signal(&self) -> f64 {
        self.signal
    }

    /// Brace-named getter: `histogram` output (MACD line - signal line).
    #[inline]
    pub fn histogram(&self) -> f64 {
        self.value - self.signal
    }

    /// Returns `true` if the MACD has received enough bars to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Resets the MACD to its initial state.
    pub fn reset(&mut self) {
        self.fast_ma.reset();
        self.slow_ma.reset();
        self.signal_ma.reset();
        self.value = 0.0;
        self.signal = 0.0;
        self.ready = false;
    }
}

impl std::fmt::Debug for Macd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Macd")
            .field("fast_period", &self.fast_period)
            .field("slow_period", &self.slow_period)
            .field("signal_period", &self.signal_period)
            .field("value", &self.value)
            .field("signal", &self.signal)
            .field("histogram", &(self.value - self.signal))
            .field("ready", &self.ready)
            .finish()
    }
}

use crate::engine::contract_engine::SmootherChoice;

/// Typed configuration for [`Macd`]: fast/slow/signal periods (host's own scalars), the
/// three per-line smoother choices (each carries its own period + shape), and the two
/// per-component price sources (`fast_source` for the fast MA, `slow_source` for the
/// slow MA — two INDEPENDENT input lanes, a real config axis).
///
/// Dual-mode: every field is a `Param`. The `#[slot]` smoothers are `Param<SmootherChoice>`
/// that FOLLOW their lane's period field (`fast`/`slow`/`signal`); the slot chooses only the kind.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct MacdConfig {
    pub fast: Param<usize>,
    pub slow: Param<usize>,
    pub signal: Param<usize>,
    #[slot]
    pub fast_ma: Param<SmootherChoice>,
    #[slot]
    pub slow_ma: Param<SmootherChoice>,
    #[slot]
    pub signal_ma: Param<SmootherChoice>,
    pub fast_source: Param<OhlcvField>,
    pub slow_source: Param<OhlcvField>,
}

impl Indicator for Macd {
    const ID: IndicatorId = IndicatorId::Macd;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Two INDEPENDENTLY-configurable lanes: the fast MA's source field + the slow MA's
    /// source field. Never collapsed — a real config axis (defaults Close/Close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Lanes(&[
        SourceLane::Config { default: OhlcvField::Close },
        SourceLane::Config { default: OhlcvField::Close },
    ]));
    /// Three outputs: line, signal, histogram.
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::MacdLine),
        Output::centered(IndicatorOutputId::MacdSignal),
        Output::centered(IndicatorOutputId::MacdHistogram),
    ];
    /// Own base ~O(1) (the line/signal/histogram scalars). The three smoothers are
    /// MovingAverage SLOTS — their cost lands recursively, one per slot.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = MacdConfig::SLOTS;
    type Config = MacdConfig;
    type Runtime = Macd;

    fn create(cfg: MacdConfig) -> Macd {
        let fast_period = cfg.fast.resolved();
        let slow_period = cfg.slow.resolved();
        let signal_period = cfg.signal.resolved();
        // Each smoother follows its lane's period field; the slot chooses only the kind.
        Macd {
            fast_ma: cfg.fast_ma.resolved().build(fast_period),
            slow_ma: cfg.slow_ma.resolved().build(slow_period),
            signal_ma: cfg.signal_ma.resolved().build(signal_period),
            fast_period,
            slow_period,
            signal_period,
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    /// MACD is a genuine TWO-lane consumer: lane 0 = `fast_source`, lane 1 = `slow_source`
    /// (the runtime resolution of `const SOURCE`). The factory extracts both from the sample
    /// and feeds the scalar pair — the two sources are an independent config axis.
    fn source_fields(cfg: &MacdConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.fast_source.resolved(), cfg.slow_source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &MacdConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for MacdConfig {
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast.resolved();
        let slow = self.slow.resolved();
        if fast >= slow {
            return Err(format!("fast({fast}) >= slow({slow})"));
        }
        Ok(())
    }
    /// Classic MACD: 12/26/9, all-EMA Own, both lanes over close.
    fn defaults() -> Self {
        MacdConfig {
            fast: Param::Solo(12),
            slow: Param::Solo(26),
            signal: Param::Solo(9),
            fast_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            slow_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            signal_ma: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            fast_source: Param::Solo(OhlcvField::Close),
            slow_source: Param::Solo(OhlcvField::Close),
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
        // fast_source/slow_source: Class O → auto all-8.
        // fast_ma/slow_ma/signal_ma: #[slot] SmootherChoice → left Solo (deferred wave).
        let mut s = Self::machine_defaults_auto();
        s.fast = Param::range(1, 100, 1);
        s.slow = Param::range(101, 10000, 1);
        s
    }
}


impl Render for Macd {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::MacdLine, "MACD", Color::hex(0x2196F3), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::MacdSignal, "Signal", Color::hex(0xFF9800), 1.0))
            .output(RenderOutput::histogram(IndicatorOutputId::MacdHistogram, "Histogram", Color::hex(0x4CAF50)))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests — pure multi-lane `feed(&[fast, slow])` core
    // =========================================================================

    /// Feed both lanes the same scalar (the Close/Close default case).
    fn feed_close(macd: &mut Macd, close: f64) {
        macd.feed(&[close, close]);
    }

    #[test]
    fn test_macd_basic_calculation() {
        let mut macd = Macd::new(12, 26);
        for i in 1..=40 {
            feed_close(&mut macd, 100.0 + i as f64);
        }
        assert!(macd.is_ready());
        assert!(macd.value_macd() > 0.0, "MACD in uptrend should be positive");
    }

    #[test]
    fn test_macd_downtrend() {
        let mut macd = Macd::new(12, 26);
        for i in 1..=40 {
            feed_close(&mut macd, 200.0 - i as f64);
        }
        assert!(macd.is_ready());
        assert!(macd.value_macd() < 0.0, "MACD in downtrend should be negative");
    }

    #[test]
    fn test_macd_value_types() {
        let mut macd = Macd::new(12, 26);
        for i in 1..=40 {
            feed_close(&mut macd, 100.0 + i as f64);
        }
        let (line, signal, histogram) = (macd.line(), macd.signal(), macd.histogram());
        assert!((line - macd.value_macd()).abs() < 1e-10);
        assert!((signal - macd.value_signal()).abs() < 1e-10);
        assert!((histogram - macd.value_histogram()).abs() < 1e-10);
    }

    #[test]
    fn test_macd_histogram() {
        let mut macd = Macd::new(12, 26);
        for i in 1..=40 {
            feed_close(&mut macd, 100.0 + i as f64);
        }
        assert!(macd.is_ready());
        let hist = macd.value_histogram();
        let expected = macd.value_macd() - macd.value_signal();
        assert!((hist - expected).abs() < 1e-10);
    }

    #[test]
    fn test_macd_custom_signal_period() {
        let mut macd = Macd::new_with_signal(12, 26, 5);
        for i in 1..=40 {
            feed_close(&mut macd, 100.0 + i as f64);
        }
        assert!(macd.is_ready());
    }

    #[test]
    fn test_macd_reset() {
        let mut macd = Macd::new(12, 26);
        for i in 1..=40 {
            feed_close(&mut macd, 100.0 + i as f64);
        }
        assert!(macd.is_ready());

        macd.reset();
        assert!(!macd.is_ready());
        assert!((macd.value_macd()).abs() < 1e-10);
        assert!((macd.value_signal()).abs() < 1e-10);
    }

    /// The factory resolves TWO independent lanes: fast over HIGH, slow over LOW. With high
    /// consistently above low, the fast MA (on high) leads the slow MA (on low) → MACD line
    /// solidly positive. If the two lanes did not resolve to DIFFERENT fields (both close),
    /// the line would hover near zero — so this proves both sources are wired.
    #[test]
    fn factory_resolves_two_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut c = <<Macd as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        c.fast_source = Param::Solo(OhlcvField::High);
        c.slow_source = Param::Solo(OhlcvField::Low);
        let mut f = IndicatorOrder::Macd(c).build_solo().unwrap();
        for i in 1..=60 {
            f.feed(0, MarketSample::Bar {
                open: 0.0,
                high: 110.0 + i as f64,
                low: 90.0 + i as f64,
                close: 100.0,
                volume: 0.0,
            });
        }
        assert!(f.primary() > 0.0, "MACD fast=High slow=Low must be > 0, got {}", f.primary());
    }
}
