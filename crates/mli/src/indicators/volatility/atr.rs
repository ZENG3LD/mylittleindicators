// High-performance Average True Range (ATR)
// (c) 2024

use crate::indicators::utils::true_range::true_range;

#[derive(Debug, Clone)]
pub struct Atr {
    smoother: SmootherSlot,
    period: usize,
    prev_close: Option<f64>,
    value: f64,
}

impl Atr {
    /// Build ATR from a pre-built `SmootherSlot` + ATR's own period — every node is
    /// self-contained (clean config tree): ATR owns `period`, the smoother owns its own.
    /// Used by `SmootherChoice::build` (create) and the convenience constructors.
    fn from_slot(smoother: SmootherSlot, period: usize) -> Self {
        Self { smoother, period, prev_close: None, value: 0.0 }
    }

    /// Build ATR from a narrow `SmootherId` + period — the legacy contract path still
    /// used by the composites that embed ATR directly (they have not yet migrated
    /// to `SmootherChoice`). Bridges the old 2-arg call site without a breaking change.
    pub fn from_smoother(period: usize, smoother: SmootherId) -> Self {
        Self::from_slot(SmootherSlot::new(smoother, period), period)
    }

    /// Create ATR with traditional Wilder's smoothing (RMA) at `period`.
    pub fn new_wilder(period: usize) -> Self {
        Self::from_slot(SmootherSlot::new(SmootherId::Rma, period), period)
    }

    /// Create ATR with Simple Moving Average at `period`.
    pub fn new_sma(period: usize) -> Self {
        Self::from_slot(SmootherSlot::new(SmootherId::Sma, period), period)
    }

    /// Create ATR with Exponential Moving Average at `period`.
    pub fn new_ema(period: usize) -> Self {
        Self::from_slot(SmootherSlot::new(SmootherId::Ema, period), period)
    }

    /// Feed the resolved `[high, low, close]` lanes (in `SOURCE` order). Computes
    /// classic Wilder True Range over the tracked prev_close; the factory extracts the
    /// fields from the bar, the core knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let tr = if let Some(prev_close) = self.prev_close {
            true_range(high, low, prev_close)
        } else {
            high - low
        };
        self.value = self.smoother.feed(tr);
        self.prev_close = Some(close);
        self.value
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    /// ATR's own configured period (self-contained node). In the normal lockstep this
    /// equals the smoother's `ma.period()`; that equality is the config resolver's job
    /// UPSTREAM of the tree, not enforced here.
    pub fn period(&self) -> usize {
        self.period
    }

    pub fn is_ready(&self) -> bool {
        self.smoother.is_ready()
    }

    pub fn reset(&mut self) {
        self.smoother.reset();
        self.prev_close = None;
        self.value = 0.0;
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::{SmootherId, SmootherSlot, SmootherChoice};
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed dual-mode contract config for [`Atr`] — self-contained: ATR owns its `period`
/// and the `smoother` (`SmootherChoice` = kind + follow/own period) runs at that period
/// by default (`Follow`). In the normal lockstep the two periods are equal; that
/// equality is the config resolver's job UPSTREAM of the tree, NOT enforced here.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct AtrConfig {
    pub period: Param<usize>,
    /// ATR smoother — default `follow(Rma)` (Wilder's smoothing) at `period`.
    #[slot]
    pub smoother: Param<SmootherChoice>,
}

impl Indicator for Atr {
    const ID: IndicatorId = IndicatorId::Atr;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Bound to the raw range slices -- True Range is h/l/prev-close.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// ATR's own base is O(1): the True Range scalar over `prev_close`. The
    /// smoothing buffer is NOT here -- it belongs to the SLOT member, whose cost
    /// lands recursively through `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = AtrConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Atr)];
    type Config = AtrConfig;
    type Runtime = Atr;

    fn create(cfg: AtrConfig) -> Atr {
        let period = cfg.period.resolved();
        let choice = cfg.smoother.resolved();
        Atr::from_slot(choice.build(period), period)
    }

    fn slot_members(cfg: &AtrConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for AtrConfig {
    fn defaults() -> Self {
        AtrConfig {
            period: Param::Solo(14),
            smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn machine_defaults() -> Self {
        // period: Class A usize — auto range(2,4048,1)
        // #[slot] smoother: leave Solo (deferred sweep wave)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Atr {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Atr, "ATR", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::Param;

    #[test]
    fn test_atr_creation() {
        let atr = Atr::new_wilder(14);
        assert!(!atr.is_ready());
        assert_eq!(atr.value(), 0.0);
    }

    #[test]
    fn test_atr_warmup() {
        let mut atr = Atr::new_wilder(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            atr.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(atr.is_ready());
    }

    #[test]
    fn test_atr_positive() {
        let mut atr = Atr::new_wilder(14);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            let value = atr.feed(&[price + 2.0, price - 2.0, price]);
            assert!(value >= 0.0, "ATR should be non-negative");
        }
    }

    #[test]
    fn test_atr_with_ema() {
        let mut atr = Atr::new_ema(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            atr.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(atr.is_ready());
        assert!(atr.value() > 0.0);
    }

    #[test]
    fn test_atr_reset() {
        let mut atr = Atr::new_wilder(14);
        for i in 0..20 {
            atr.feed(&[101.0, 99.0, 100.0 + i as f64]);
        }
        atr.reset();
        assert!(!atr.is_ready());
        assert_eq!(atr.value(), 0.0);
    }

    #[test]
    fn test_atr_dual_mode_config() {
        let cfg = AtrConfig {
            period: Param::Solo(14),
            smoother: Param::Solo(SmootherChoice::own(SmootherId::Sma, 14)),
        };
        assert_eq!(cfg.period.resolved(), 14);
        let mut atr = <Atr as Indicator>::create(cfg);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            atr.feed(&[price + 2.0, price - 2.0, price]);
        }
        assert!(atr.is_ready());
    }
}
