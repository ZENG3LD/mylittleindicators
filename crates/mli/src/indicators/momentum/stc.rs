use crate::engine::contract_engine::{SmootherSlot, SmootherId};
use crate::indicators::momentum::macd::Macd;

/// Schaff Trend Cycle (STC): Stochastic of MACD smoothed twice.
///
/// Algorithm:
/// 1. Compute MACD(fast, slow, signal_period)
/// 2. Feed MACD histogram difference (macd_line - signal_line) into k_ma and d_ma
/// 3. Apply tanh normalization to map to 0-100
///
/// Output: Double(%K, %D)
#[derive(Debug, Clone)]
pub struct Stc {
    macd: Macd,
    k_ma: SmootherSlot, // smoothing for %K
    d_ma: SmootherSlot, // smoothing for %D
    k: f64,
    d: f64,
    ready: bool,
}

impl Stc {
    pub fn new(fast: usize, slow: usize, k_period: usize, d_period: usize) -> Self {
        Self::from_smoothers(
            fast, slow, 9,
            SmootherId::Ema, k_period,
            SmootherId::Ema, d_period,
        )
    }

    /// Create STC with configurable MACD signal period and smoothing MA type.
    pub fn from_smoothers(
        fast: usize,
        slow: usize,
        signal_period: usize,
        k_id: SmootherId,
        k_period: usize,
        d_id: SmootherId,
        d_period: usize,
    ) -> Self {
        Self {
            macd: Macd::new_with_signal(fast, slow, signal_period.max(1)),
            k_ma: SmootherSlot::new(k_id, k_period.max(1)),
            d_ma: SmootherSlot::new(d_id, d_period.max(1)),
            k: 0.0,
            d: 0.0,
            ready: false,
        }
    }

    /// Feed ONE pre-extracted scalar (close). Returns (%K, %D).
    pub fn feed(&mut self, c: f64) -> (f64, f64) {
        // MACD is a pure two-lane core; STC is close-based, so feed both lanes the close.
        let macd_val = self.macd.feed(&[c, c]);
        let signal = self.macd.value_signal();
        let raw = macd_val - signal;
        let k_sm = self.k_ma.feed(raw);
        let d_sm = self.d_ma.feed(k_sm);
        self.k = 50.0 + 50.0 * (k_sm.tanh());
        self.d = 50.0 + 50.0 * (d_sm.tanh());
        self.ready = self.k_ma.is_ready() && self.d_ma.is_ready() && self.macd.is_ready();
        (self.k, self.d)
    }


    /// Brace-named getter: `k` output (%K Schaff Trend Cycle).
    #[inline]
    pub fn k(&self) -> f64 {
        self.k
    }

    /// Brace-named getter: `d` output (%D Schaff Trend Cycle).
    #[inline]
    pub fn d(&self) -> f64 {
        self.d
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }
    pub fn reset(&mut self) {
        self.macd.reset();
        self.k_ma.reset();
        self.d_ma.reset();
        self.k = 0.0;
        self.d = 0.0;
        self.ready = false;
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed contract config for [`Stc`].
///
/// Absorbs the MACD knobs (fast/slow/signal) as own `Param<usize>` fields.
/// The k and d smoothers are `Param<SmootherChoice>` that follow their respective period fields.
/// Dual-mode: every field is a `Param`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct StcConfig {
    pub fast: Param<usize>,
    pub slow: Param<usize>,
    pub signal: Param<usize>,
    pub k_period: Param<usize>,
    pub d_period: Param<usize>,
    #[slot]
    pub k_smoother: Param<SmootherChoice>,
    #[slot]
    pub d_smoother: Param<SmootherChoice>,
}

impl Indicator for Stc {
    const ID: IndicatorId = IndicatorId::Stc;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Single configurable price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1) own update; MACD cost is charged via the Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Macd, &[
            IndicatorOutputId::MacdLine,
            IndicatorOutputId::MacdSignal,
            IndicatorOutputId::MacdHistogram,
        ])],
    };
    const SLOTS: &'static [Slot] = StcConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::StcK),
        Output::percent(IndicatorOutputId::StcD),
    ];
    type Config = StcConfig;
    type Runtime = Stc;

    fn create(cfg: StcConfig) -> Stc {
        let fast = cfg.fast.resolved();
        let slow = cfg.slow.resolved();
        let signal = cfg.signal.resolved();
        let k_period = cfg.k_period.resolved().max(1);
        let d_period = cfg.d_period.resolved().max(1);
        Stc {
            macd: Macd::new_with_signal(fast, slow, signal.max(1)),
            k_ma: cfg.k_smoother.resolved().build(k_period),
            d_ma: cfg.d_smoother.resolved().build(d_period),
            k: 0.0,
            d: 0.0,
            ready: false,
        }
    }

    fn source_fields(cfg: &StcConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }

    fn slot_members(cfg: &StcConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for StcConfig {
    fn defaults() -> Self {
        StcConfig {
            fast: Param::Solo(12),
            slow: Param::Solo(26),
            signal: Param::Solo(9),
            k_period: Param::Solo(10),
            d_period: Param::Solo(3),
            k_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            d_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast.resolved();
        let slow = self.slow.resolved();
        if fast >= slow {
            return Err(format!("fast({fast}) >= slow({slow})"));
        }
        Ok(())
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // fast/slow/signal/k_period/d_period: Class A → auto range(2,4048,1) EACH, but fast and
        // slow then both resolve to the same min (2), failing this config's OWN `valid_params`
        // (fast < slow) at the min corner (2026-07-03 fix). Split into disjoint ranges so
        // `resolved()` stays ordered; signal/k_period/d_period (independent lanes) keep the
        // full auto range.
        // k_smoother/d_smoother: #[slot] SmootherChoice → left Solo (deferred wave).
        let mut s = Self::machine_defaults_auto();
        s.fast = Param::range(1, 100, 1);
        s.slow = Param::range(101, 10000, 1);
        s
    }
}


impl Render for Stc {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::StcK, "STC", Color::hex(0x9C27B0))
            .line_output(IndicatorOutputId::StcD, "Signal", Color::hex(0xFF9800))
            .bounds(0.0, 100.0)
            .precision(2)
            .build()
    }
}

impl Default for Stc {
    fn default() -> Self {
        Self::from_smoothers(12, 26, 9, SmootherId::Ema, 10, SmootherId::Ema, 3)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stc_creation() {
        let stc = Stc::new(12, 26, 10, 3);
        assert!(!stc.is_ready());
        assert_eq!(stc.k(), 0.0);
        assert_eq!(stc.d(), 0.0);
    }

    #[test]
    fn test_stc_uptrend() {
        let mut stc = Stc::new(12, 26, 10, 3);
        for i in 1..=50 {
            let p = 100.0 + i as f64 * 2.0;
            stc.feed(p);
        }
        assert!(stc.is_ready());
        assert!(stc.k().is_finite() && stc.d().is_finite());
    }

    #[test]
    fn test_stc_range() {
        let mut stc = Stc::new(12, 26, 10, 3);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let (k, d) = stc.feed(price);
            assert!(k >= 0.0 && k <= 100.0, "STC K should be in [0, 100], got {}", k);
            assert!(d >= 0.0 && d <= 100.0, "STC D should be in [0, 100], got {}", d);
        }
    }

    #[test]
    fn test_stc_reset() {
        let mut stc = Stc::new(12, 26, 10, 3);
        for i in 1..=50 {
            stc.feed(100.0 + i as f64);
        }
        assert!(stc.is_ready());
        stc.reset();
        assert!(!stc.is_ready());
        assert_eq!(stc.k(), 0.0);
        assert_eq!(stc.d(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_stc() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<Stc as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Stc(cfg).build_solo().unwrap();
        for i in 1..=60 {
            let p = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: p,
                volume: 9999.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
