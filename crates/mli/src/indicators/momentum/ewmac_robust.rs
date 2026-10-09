// Robust EWMAC: MAD-normalized fast-slow MA crossover, squashed to [-1, 1] by tanh.

use crate::engine::contract_engine::{SmootherSlot, SmootherId};
use crate::engine::ohlcv_field::OhlcvField;
use crate::indicators::utils::math::percentile::median;

/// Robust EWMAC = `tanh((FastMA - SlowMA) / MAD)`, where MAD is the median absolute
/// deviation of price over a robust window (scaled by 1.4826). Two independent smoother
/// slots; the MAD normalization makes the signal scale-free and bounded in [-1, 1].
#[derive(Clone, Debug)]
pub struct EwmacRobust {
    fast_period: usize,
    slow_period: usize,
    fast: SmootherSlot,
    slow: SmootherSlot,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl EwmacRobust {
    /// Default ctor — both MAs are EMA over close.
    pub fn new(fast_period: usize, slow_period: usize, robust_window: usize) -> Self {
        Self::from_smoothers(SmootherId::Ema, fast_period, SmootherId::Ema, slow_period, robust_window)
    }

    /// Build from narrow `SmootherId`s for the fast + slow MAs (over close).
    /// Legacy bridge; the contract path goes through `EwmacRobustConfig`.
    pub fn from_smoothers(fast: SmootherId, fast_period: usize, slow: SmootherId, slow_period: usize, robust_window: usize) -> Self {
        let window = robust_window.max(15);
        Self {
            fast_period,
            slow_period,
            fast: SmootherSlot::new(fast, fast_period.max(1)),
            slow: SmootherSlot::new(slow, slow_period.max(1)),
            window,
            buf: vec![0.0; window],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        // Reset the existing slots in place — never rebuild them (that would drop a
        // configured smoother's shape, e.g. ALMA offset/sigma).
        self.fast.reset();
        self.slow.reset();
        self.idx = 0;
        self.filled = false;
        self.buf.fill(0.0);
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.fast.is_ready() && self.slow.is_ready()
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    /// Median of the robust window (O(window) quickselect).
    fn median_value(&self) -> f64 {
        let mut v = self.buf.clone();
        median(&mut v)
    }

    /// Median absolute deviation about `med`, scaled to a robust std estimate.
    fn mad(&self, med: f64) -> f64 {
        let mut d: Vec<f64> = self.buf.iter().map(|x| (x - med).abs()).collect();
        let m = median(&mut d);
        (m * 1.4826).max(1e-9)
    }

    /// Feed ONE resolved scalar (the configured source field).
    pub fn feed(&mut self, v: f64) -> f64 {
        self.buf[self.idx] = v;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        let f = self.fast.feed(v);
        let s = self.slow.feed(v);
        let raw = f - s;
        if self.filled {
            let med = self.median_value();
            let scale = self.mad(med);
            self.value = (raw / scale).tanh();
        } else {
            self.value = 0.0;
        }
        self.value
    }

    pub fn fast_period(&self) -> usize {
        self.fast_period
    }

    pub fn slow_period(&self) -> usize {
        self.slow_period
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed contract config for [`EwmacRobust`] — MAD-normalized fast/slow crossover.
/// Two independent smoother slots each following their lane's period field; `robust_window`
/// is the median/MAD window; `source` is the price field.
///
/// Dual-mode: every field is a `Param`. The two `#[slot]` smoothers are `Param<SmootherChoice>`
/// that follow their respective period fields (fast=32, slow=128 by default).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct EwmacRobustConfig {
    pub source: Param<OhlcvField>,
    pub robust_window: Param<usize>,
    pub fast_period: Param<usize>,
    pub slow_period: Param<usize>,
    #[slot]
    pub fast: Param<SmootherChoice>,
    #[slot]
    pub slow: Param<SmootherChoice>,
}

impl Indicator for EwmacRobust {
    const ID: IndicatorId = IndicatorId::EwmacRobust;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(window): a per-bar median + MAD rescan of the robust window (`Vec`); the two
    /// smoother buffers land recursively through `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const SLOTS: &'static [Slot] = EwmacRobustConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::EwmacRobust)];
    type Config = EwmacRobustConfig;
    type Runtime = EwmacRobust;

    fn create(cfg: EwmacRobustConfig) -> EwmacRobust {
        let fast_period = cfg.fast_period.resolved().max(1);
        let slow_period = cfg.slow_period.resolved().max(1);
        let window = cfg.robust_window.resolved().max(15);
        EwmacRobust {
            fast_period,
            slow_period,
            fast: cfg.fast.resolved().build(fast_period),
            slow: cfg.slow.resolved().build(slow_period),
            window,
            buf: vec![0.0; window],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    fn slot_members(cfg: &EwmacRobustConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }

    /// Field-source core: the factory holds `cfg.source` and feeds the resolved scalar.
    fn source_fields(cfg: &EwmacRobustConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for EwmacRobustConfig {
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast_period.resolved();
        let slow = self.slow_period.resolved();
        if fast >= slow {
            return Err(format!("fast_period({fast}) >= slow_period({slow})"));
        }
        Ok(())
    }
    fn defaults() -> Self {
        EwmacRobustConfig {
            source: Param::Solo(OhlcvField::Close),
            robust_window: Param::Solo(252),
            fast_period: Param::Solo(32),
            slow_period: Param::Solo(128),
            fast: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
            slow: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // robust_window/fast_period/slow_period: Class A → auto range(2,4048,1) EACH, but
        // fast_period and slow_period then both resolve to the same min (2), failing this
        // config's OWN `valid_params` (fast < slow) at the min corner (2026-07-03 fix). Split
        // into disjoint ranges so `resolved()` stays ordered; robust_window (an independent
        // MAD window) keeps the full auto range.
        // source: Class O → auto all-8; fast/slow: #[slot] SmootherChoice → left Solo (deferred wave).
        let mut s = Self::machine_defaults_auto();
        s.fast_period = Param::range(1, 100, 1);
        s.slow_period = Param::range(101, 10000, 1);
        s
    }
}


impl Render for EwmacRobust {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::EwmacRobust, "EWMAC Robust", Color::hex(0x9C27B0))
            .bounds(-1.0, 1.0)
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ewmac_robust_creation() {
        let ewmac = EwmacRobust::new(8, 32, 30);
        assert!(!ewmac.is_ready());
        assert_eq!(ewmac.value, 0.0);
        assert_eq!(ewmac.fast_period(), 8);
        assert_eq!(ewmac.slow_period(), 32);
    }

    #[test]
    fn test_ewmac_robust_uptrend() {
        let mut ewmac = EwmacRobust::new(8, 32, 30);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            ewmac.feed(price);
        }
        assert!(ewmac.is_ready());
        assert!(ewmac.value > 0.0, "EWMAC Robust should be > 0 in uptrend, got {}", ewmac.value);
    }

    #[test]
    fn test_ewmac_robust_range() {
        let mut ewmac = EwmacRobust::new(8, 32, 30);
        for i in 1..=60 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = ewmac.feed(price);
            if ewmac.is_ready() {
                assert!(value >= -1.0 && value <= 1.0, "EWMAC Robust should be in [-1, 1], got {}", value);
            }
        }
    }

    #[test]
    fn test_ewmac_robust_reset() {
        let mut ewmac = EwmacRobust::new(8, 32, 30);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            ewmac.feed(price);
        }
        assert!(ewmac.is_ready());
        ewmac.reset();
        assert!(!ewmac.is_ready());
        assert_eq!(ewmac.value, 0.0);
    }

    #[test]
    fn test_ewmac_robust_contract_create() {
        let cfg = <<EwmacRobust as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.fast_period.resolved(), 32);
        assert_eq!(cfg.slow_period.resolved(), 128);
        let mut ewmac = <EwmacRobust as Indicator>::create(cfg);
        for i in 1..=300 {
            let price = 100.0 + i as f64;
            ewmac.feed(price);
        }
        assert!(ewmac.is_ready());
    }
}
