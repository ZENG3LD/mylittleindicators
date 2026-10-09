//! Percentage Volume Oscillator (PVO) indicator.

use crate::engine::contract_engine::SmootherSlot;

/// Percentage Volume Oscillator (PVO) — volume-based momentum indicator.
///
/// PVO Line   = 100 × (Fast MA(volume) - Slow MA(volume)) / Slow MA(volume)
/// Signal Line = MA(PVO Line)
/// Histogram   = PVO Line - Signal Line
///
/// Similar to PPO but applied to volume. Helps identify volume trends and
/// divergences with price action.
///
/// PURE core — three independent smoother slots (fast, slow, signal).
/// The factory feeds the resolved volume scalar via [`Pvo::feed`].
#[derive(Debug, Clone)]
pub struct Pvo {
    fast_ma: SmootherSlot,
    slow_ma: SmootherSlot,
    signal_ma: SmootherSlot,
    value: f64,
    signal: f64,
    ready: bool,
}

impl Pvo {
    /// Creates a new PVO with default EMA smoothers.
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self::from_smoothers(
            SmootherId::Ema, fast_period,
            SmootherId::Ema, slow_period,
            SmootherId::Ema, signal_period,
        )
    }

    /// Build from narrow `SmootherId`s for each of the three components.
    /// Legacy bridge; the contract path goes through `PvoConfig`.
    pub fn from_smoothers(
        fast: SmootherId, fast_period: usize,
        slow: SmootherId, slow_period: usize,
        signal: SmootherId, signal_period: usize,
    ) -> Self {
        Self {
            fast_ma: SmootherSlot::new(fast, fast_period.max(1)),
            slow_ma: SmootherSlot::new(slow, slow_period.max(1)),
            signal_ma: SmootherSlot::new(signal, signal_period.max(1)),
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    /// Feed one pre-extracted volume scalar — the pure core computation.
    pub fn feed(&mut self, v: f64) -> f64 {
        let fast = self.fast_ma.feed(v);
        let slow = self.slow_ma.feed(v);
        let denom = if slow.abs() < 1e-12 { 1e-12 } else { slow };
        self.value = 100.0 * (fast - slow) / denom;
        if self.fast_ma.is_ready() && self.slow_ma.is_ready() {
            self.signal = self.signal_ma.feed(self.value);
        }
        self.ready =
            self.fast_ma.is_ready() && self.slow_ma.is_ready() && self.signal_ma.is_ready();
        self.value
    }


    /// Returns the PVO line value.
    #[inline]
    pub fn value_pvo(&self) -> f64 {
        self.value
    }

    /// Returns the signal line value.
    #[inline]
    pub fn value_signal(&self) -> f64 {
        self.signal
    }

    /// Returns the histogram value (PVO - Signal).
    #[inline]
    pub fn value_histogram(&self) -> f64 {
        self.value - self.signal
    }

    /// Named output: brace `line`.
    #[inline]
    pub fn line(&self) -> f64 { self.value }
    /// Named output: brace `signal`.
    #[inline]
    pub fn signal(&self) -> f64 { self.signal }
    /// Named output: brace `histogram`.
    #[inline]
    pub fn histogram(&self) -> f64 { self.value - self.signal }

    /// Returns `true` if the PVO has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Resets the PVO to its initial state.
    pub fn reset(&mut self) {
        self.fast_ma.reset();
        self.slow_ma.reset();
        self.signal_ma.reset();
        self.value = 0.0;
        self.signal = 0.0;
        self.ready = false;
    }
}

impl Default for Pvo {
    fn default() -> Self {
        Self::new(12, 26, 9)
    }
}

// ─── Contract ────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::{IndicatorOutputId, SmootherId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, HistogramStyle, Indicator, Output, Param, Render, RenderOutput, RenderSpec,
    Slot, SourceAxis, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Typed contract config for [`Pvo`] — three independent smoother slots plus their periods.
///
/// PVO reads only volume (a fixed field — not user-configurable), so there is no
/// `source` field. Each slot is `Param<SmootherSlotOrder>` carrying its own member + period.
/// Defaults = EMA at the respective host period (fast=12, slow=26, signal=9).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct PvoConfig {
    pub fast_period: Param<usize>,
    pub slow_period: Param<usize>,
    pub signal_period: Param<usize>,
    #[slot]
    pub fast: Param<SmootherSlotOrder>,
    #[slot]
    pub slow: Param<SmootherSlotOrder>,
    #[slot]
    pub signal: Param<SmootherSlotOrder>,
}

impl Indicator for Pvo {
    const ID: IndicatorId = IndicatorId::Pvo;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed volume field — PVO always operates on volume; not user-configurable.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Volume });
    /// O(1): three smoother scalars + one percentage ratio; smoother buffers cost via SLOTS.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = PvoConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::PvoLine),
        Output::centered(IndicatorOutputId::PvoSignal),
        Output::centered(IndicatorOutputId::PvoHistogram),
    ];
    type Config = PvoConfig;
    type Runtime = Pvo;

    fn create(cfg: PvoConfig) -> Pvo {
        // Each slot order carries its own period — no host-period resolve needed.
        Pvo {
            fast_ma: cfg.fast.resolved().into_slot(),
            slow_ma: cfg.slow.resolved().into_slot(),
            signal_ma: cfg.signal.resolved().into_slot(),
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    /// Volume is the fixed source — override to lock it regardless of the Field default.
    fn source_fields(_cfg: &PvoConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Volume].into_iter().collect()
    }

    fn slot_members(cfg: &PvoConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for PvoConfig {
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast_period.resolved();
        let slow = self.slow_period.resolved();
        if fast >= slow {
            return Err(format!("fast_period({fast}) >= slow_period({slow})"));
        }
        Ok(())
    }
    fn defaults() -> Self {
        PvoConfig {
            fast_period: Param::Solo(12),
            slow_period: Param::Solo(26),
            signal_period: Param::Solo(9),
            fast: Param::Solo(SmootherSlotOrder::Ema(PeriodConfig { period: 12 })),
            slow: Param::Solo(SmootherSlotOrder::Ema(PeriodConfig { period: 26 })),
            signal: Param::Solo(SmootherSlotOrder::Ema(PeriodConfig { period: 9 })),
        }
    }
    fn machine_defaults() -> Self {
        // fast_period/slow_period/signal_period: Class A — auto gives range(2,4048,1) EACH,
        // but fast_period and slow_period then both resolve to the same min (2), failing this
        // config's OWN `valid_params` (fast < slow) at the min corner (2026-07-03 fix). Split
        // into disjoint ranges so `resolved()` stays ordered; signal_period (an independent
        // lane) keeps the full auto range.
        let mut s = Self::machine_defaults_auto();
        s.fast_period = Param::range(1, 100, 1);
        s.slow_period = Param::range(101, 10000, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Pvo {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::PvoLine, "PVO", Color::hex(0x9C27B0), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::PvoSignal, "Signal", Color::hex(0xFF9800), 1.0))
            .output(RenderOutput::histogram(IndicatorOutputId::PvoHistogram, "Histogram", Color::hex(0x4CAF50)))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;

    #[test]
    fn test_pvo_creation() {
        let pvo = Pvo::new(12, 26, 9);
        assert!(!pvo.is_ready());
        assert_eq!(pvo.value_pvo(), 0.0);
        assert_eq!(pvo.value_signal(), 0.0);
    }

    #[test]
    fn test_pvo_warmup() {
        let mut pvo = Pvo::new(12, 26, 9);
        for i in 0..50 {
            pvo.feed(1000.0 + i as f64 * 10.0);
        }
        assert!(pvo.is_ready());
    }

    #[test]
    fn test_pvo_values_finite() {
        let mut pvo = Pvo::new(12, 26, 9);
        for i in 0..50 {
            let volume = 1000.0 + (i as f64 * 0.2).sin() * 500.0;
            let value = pvo.feed(volume);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_pvo_histogram() {
        let mut pvo = Pvo::new(12, 26, 9);
        for i in 0..50 {
            pvo.feed(1000.0 + i as f64 * 100.0);
        }
        let histogram = pvo.value_histogram();
        assert!(histogram.is_finite());
        assert_eq!(histogram, pvo.value_pvo() - pvo.value_signal());
    }

    #[test]
    fn test_pvo_reset() {
        let mut pvo = Pvo::new(12, 26, 9);
        for i in 0..50 {
            pvo.feed(1000.0 + i as f64 * 10.0);
        }
        pvo.reset();
        assert!(!pvo.is_ready());
        assert_eq!(pvo.value_pvo(), 0.0);
        assert_eq!(pvo.value_signal(), 0.0);
    }

    #[test]
    fn test_pvo_with_sma() {
        let mut pvo = Pvo::from_smoothers(
            SmootherId::Sma, 12,
            SmootherId::Sma, 26,
            SmootherId::Sma, 9,
        );
        for i in 0..50 {
            pvo.feed(1000.0 + i as f64 * 100.0);
        }
        assert!(pvo.is_ready());
    }

    #[test]
    fn test_pvo_value_returns_macd_type() {
        let mut pvo = Pvo::new(12, 26, 9);
        for i in 0..50 {
            pvo.feed(1000.0 + i as f64 * 100.0);
        }
        let (line, signal, histogram) = (pvo.line(), pvo.signal(), pvo.histogram());
        assert!((line - pvo.value_pvo()).abs() < 1e-10);
        assert!((signal - pvo.value_signal()).abs() < 1e-10);
        assert!((histogram - pvo.value_histogram()).abs() < 1e-10);
    }

    /// Factory smoke: build via `IndicatorOrder`, feed volume bars, assert the PVO line is
    /// finite. The open/high/low/close fields are set to a wild sentinel (9999.0) — because
    /// const SOURCE declares Volume, the factory must ignore them entirely.
    #[test]
    fn factory_feeds_resolved_pvo() {
        use crate::contract::MarketSample;
        let cfg = <<Pvo as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Pvo(cfg).build_solo().unwrap();
        for i in 0..50 {
            let vol = 1000.0 + i as f64 * 50.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: 9999.0,
                volume: vol,
            });
        }
        assert!(f.read(IndicatorOutputId::PvoLine).is_finite());
    }
}
