// RAVI (Range Action Verification Index): |FastMA - SlowMA| / SlowMA * 100

use crate::engine::contract_engine::{SmootherSlot, SmootherId};

/// RAVI = `|FastMA - SlowMA| / SlowMA * 100` — a trend/range filter (high = trending,
/// low = ranging). Two independent smoother slots, each owning its period + shape. PURE
/// core — owns no source; the factory feeds it the resolved scalar via [`Ravi::feed`].
#[derive(Debug, Clone)]
pub struct Ravi {
    fast: SmootherSlot,
    slow: SmootherSlot,
    value: f64,
}

impl Ravi {
    /// Default ctor — both MAs are EMA.
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self::from_smoothers(SmootherId::Ema, fast_period, SmootherId::Ema, slow_period)
    }

    /// Alias kept for the legacy call site.
    pub fn new_default(fast_period: usize, slow_period: usize) -> Self {
        Self::new(fast_period, slow_period)
    }

    /// Build from narrow `SmootherId`s for the fast + slow MAs.
    /// Legacy bridge; the contract path goes through `RaviConfig`.
    pub fn from_smoothers(fast: SmootherId, fast_period: usize, slow: SmootherId, slow_period: usize) -> Self {
        Self {
            fast: SmootherSlot::new(fast, fast_period.max(1)),
            slow: SmootherSlot::new(slow, slow_period.max(2)),
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.fast.reset();
        self.slow.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.fast.is_ready() && self.slow.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, value: f64) -> f64 {
        let f = self.fast.feed(value);
        let s = self.slow.feed(value);
        self.value = if s.abs() > 1e-12 {
            ((f - s).abs() / s) * 100.0
        } else {
            0.0
        };
        self.value
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed contract config for [`Ravi`] — `|FastMA - SlowMA| / SlowMA * 100`. Two
/// independent smoother slots each following their lane's period field; `source` is the price field.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct RaviConfig {
    pub source: Param<OhlcvField>,
    pub fast_period: Param<usize>,
    pub slow_period: Param<usize>,
    #[slot]
    pub fast: Param<SmootherChoice>,
    #[slot]
    pub slow: Param<SmootherChoice>,
}

impl Indicator for Ravi {
    const ID: IndicatorId = IndicatorId::Ravi;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1): a ratio of two smoothers; both buffers land recursively through `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = RaviConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Ravi)];
    type Config = RaviConfig;
    type Runtime = Ravi;

    fn create(cfg: RaviConfig) -> Ravi {
        let fast_p = cfg.fast_period.resolved().max(1);
        let slow_p = cfg.slow_period.resolved().max(2);
        Ravi {
            fast: cfg.fast.resolved().build(fast_p),
            slow: cfg.slow.resolved().build(slow_p),
            value: 0.0,
        }
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the resolved scalar.
    fn source_fields(cfg: &RaviConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &RaviConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for RaviConfig {
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast_period.resolved();
        let slow = self.slow_period.resolved();
        if fast >= slow {
            return Err(format!("fast_period({fast}) >= slow_period({slow})"));
        }
        Ok(())
    }
    fn defaults() -> Self {
        RaviConfig {
            source: Param::Solo(OhlcvField::Close),
            fast_period: Param::Solo(7),
            slow_period: Param::Solo(65),
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
        // source: Class O — auto all-8; fast_period/slow_period: Class A — auto range(2,4048,1)
        // EACH, but both then resolve to the same min (2), failing this config's OWN
        // valid_params (fast < slow) at the min corner (2026-07-03 fix). Split into disjoint
        // ranges so resolved() stays ordered.
        // fast/slow: #[slot] SmootherChoice — left Solo (deferred wave). [FLAG: slots]
        let mut s = Self::machine_defaults_auto();
        s.fast_period = Param::range(1, 100, 1);
        s.slow_period = Param::range(101, 10000, 1);
        s
    }
}


impl Render for Ravi {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Ravi, "RAVI", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ravi_creation() {
        let ravi = Ravi::new(7, 65);
        assert!(!ravi.is_ready());
        assert_eq!(ravi.value(), 0.0);
    }

    #[test]
    fn test_ravi_warmup() {
        let mut ravi = Ravi::new(7, 65);
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ravi.feed(price);
        }
        assert!(ravi.is_ready());
    }

    #[test]
    fn test_ravi_values_non_negative() {
        let mut ravi = Ravi::new(7, 65);
        for i in 0..80 {
            let value = ravi.feed(100.0 + i as f64);
            assert!(value >= 0.0, "RAVI should be non-negative");
        }
    }

    #[test]
    fn test_ravi_reset() {
        let mut ravi = Ravi::new(7, 65);
        for _ in 0..80 {
            ravi.feed(101.0);
        }
        ravi.reset();
        assert!(!ravi.is_ready());
        assert_eq!(ravi.value(), 0.0);
    }

    #[test]
    fn test_ravi_contract_create() {
        let cfg = <<Ravi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.fast_period.resolved(), 7);
        assert_eq!(cfg.slow_period.resolved(), 65);
        let mut ravi = <Ravi as Indicator>::create(cfg);
        for i in 0..80 {
            ravi.feed(100.0 + i as f64);
        }
        assert!(ravi.is_ready());
    }
}
