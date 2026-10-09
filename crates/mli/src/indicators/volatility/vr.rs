// High-performance Volatility Ratio (VR)
// (c) 2024

use super::atr::Atr;
use crate::engine::contract_engine::SmootherId;

#[derive(Debug, Clone)]
pub struct Vr {
    atr_fast: Atr,
    atr_slow: Atr,
    value: f64,
}

impl Vr {
    /// Default ctor — both ATRs smoothed with RMA (Wilder).
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self::from_smoothers(fast_period, slow_period, SmootherId::Rma)
    }

    /// Build from a narrow `SmootherId` applied to both ATRs.
    pub fn from_smoothers(fast_period: usize, slow_period: usize, smoother: SmootherId) -> Self {
        Self {
            atr_fast: Atr::from_smoother(fast_period, smoother),
            atr_slow: Atr::from_smoother(slow_period, smoother),
            value: 0.0,
        }
    }

    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high  = lanes[0];
        let low   = lanes[1];
        let close = lanes[2];
        let fast = self.atr_fast.feed(&[high, low, close]);
        let slow = self.atr_slow.feed(&[high, low, close]);
        if fast > 0.0 {
            self.value = slow / fast;
        } else {
            self.value = 0.0;
        }
        self.value
    }

    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.atr_fast.is_ready() && self.atr_slow.is_ready()
    }
    pub fn reset(&mut self) {
        self.atr_fast.reset();
        self.atr_slow.reset();
        self.value = 0.0;
    }
}

impl Default for Vr {
    fn default() -> Self {
        Self::new(10, 20)
    }
}

// -- contract ------------------------------------------------------------------

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Config for [`Vr`]: two independent period fields + two smoother slots (fast ATR + slow ATR).
///
/// Each slot defaults to `follow(Rma)`: the fast smoother follows `fast_period`, the slow
/// smoother follows `slow_period`.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct VrConfig {
    /// Fast ATR period.
    pub fast_period: Param<usize>,
    /// Slow ATR period.
    pub slow_period: Param<usize>,
    /// Fast ATR smoother — default `follow(Rma)` at `fast_period`.
    #[slot]
    pub fast_smoother: Param<SmootherChoice>,
    /// Slow ATR smoother — default `follow(Rma)` at `slow_period`.
    #[slot]
    pub slow_smoother: Param<SmootherChoice>,
}

impl Indicator for Vr {
    const ID: IndicatorId = IndicatorId::VoVr;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High, OhlcvField::Low, OhlcvField::Close,
    ]));
    /// O(1): two recursive ATR smoothers; no window buffer on the outer struct itself.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr]),
            Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr]),
        ],
    };
    const SLOTS: &'static [Slot] = VrConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::ratio(IndicatorOutputId::VoVr)];
    type Config = VrConfig;
    type Runtime = Vr;

    fn create(cfg: VrConfig) -> Vr {
        let fp = cfg.fast_period.resolved();
        let sp = cfg.slow_period.resolved();
        let fch = cfg.fast_smoother.resolved();
        let sch = cfg.slow_smoother.resolved();
        Vr {
            atr_fast: Atr::from_smoother(fch.period.resolve(fp), fch.kind),
            atr_slow: Atr::from_smoother(sch.period.resolve(sp), sch.kind),
            value: 0.0,
        }
    }

    fn slot_members(cfg: &VrConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for VrConfig {
    fn valid_params(&self) -> Result<(), String> {
        let fast = self.fast_period.resolved();
        let slow = self.slow_period.resolved();
        if fast >= slow {
            return Err(format!("fast_period({fast}) >= slow_period({slow})"));
        }
        Ok(())
    }
    fn defaults() -> Self {
        VrConfig {
            fast_period:   Param::Solo(10),
            slow_period:   Param::Solo(20),
            fast_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
            slow_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn machine_defaults() -> Self {
        // fast_period, slow_period: Class A usize — auto range(2,4048,1) each, but BOTH
        // resolve to the same min (2) independently, which fails this config's OWN
        // `valid_params` (fast < slow) at the min corner (2026-07-03 fix). Split into two
        // disjoint ranges so `resolved()` (the first/min value of each) stays ordered.
        // #[slot] fast_smoother, slow_smoother: leave Solo (deferred sweep wave)
        let mut s = Self::machine_defaults_auto();
        s.fast_period = Param::range(1, 100, 1);
        s.slow_period = Param::range(101, 10000, 1);
        s
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
}


impl Render for Vr {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::VoVr, "Vol Ratio", Color::hex(0x009688))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vr_creation() {
        let vr = Vr::new(7, 14);
        assert!(!vr.is_ready());
        assert_eq!(vr.value(), 0.0);
    }

    #[test]
    fn test_vr_warmup() {
        let mut vr = Vr::new(7, 14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vr.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(vr.is_ready());
    }

    #[test]
    fn test_vr_positive() {
        let mut vr = Vr::new(7, 14);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vr.feed(&[price + 1.0, price - 1.0, price]);
            assert!(value >= 0.0, "VR should be non-negative");
        }
    }

    #[test]
    fn test_vr_reset() {
        let mut vr = Vr::new(7, 14);
        for i in 0..20 {
            vr.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        vr.reset();
        assert!(!vr.is_ready());
        assert_eq!(vr.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_vovr() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::VoVr(<<Vr as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..25 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, // not used by H/L/C source
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.read(IndicatorOutputId::VoVr) >= 0.0);
    }
}
