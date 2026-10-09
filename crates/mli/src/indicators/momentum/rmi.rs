use crate::engine::contract_engine::SmootherSlot;
use std::collections::VecDeque;

/// Relative Momentum Index (RMI)
/// RMI_t = 100 * EMA_n( max(0, C_t - C_{t-m}) ) / ( EMA_n(max(0, C_t - C_{t-m})) + EMA_n(max(0, C_{t-m} - C_t)) )
///
/// PURE core — owns no source; the factory feeds it the resolved scalar via [`Rmi::feed`].
#[derive(Debug, Clone)]
pub struct Rmi {
    momentum_lookback: usize,
    ema_period: usize,
    up_ma: SmootherSlot,
    down_ma: SmootherSlot,
    closes: VecDeque<f64>,
    value: f64,
    ready: bool,
}

impl Rmi {
    /// Default ctor — up/down momentum smoothed with RMA (Wilder).
    pub fn new(momentum_lookback: usize, ema_period: usize) -> Self {
        Self::from_smoother(momentum_lookback, ema_period, SmootherId::Rma)
    }

    /// Build from a narrow `SmootherId` for the up/down smoothers + lookback + period.
    /// Legacy bridge; the contract path goes through `RmiConfig`.
    pub fn from_smoother(momentum_lookback: usize, ema_period: usize, smoother: SmootherId) -> Self {
        let m = momentum_lookback.max(1);
        let n = ema_period.max(1);
        Self {
            momentum_lookback: m,
            ema_period: n,
            up_ma: SmootherSlot::new(smoother, n),
            down_ma: SmootherSlot::new(smoother, n),
            closes: VecDeque::with_capacity(m + 1),
            value: 0.0,
            ready: false,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, value: f64) -> f64 {
        self.closes.push_back(value);
        if self.closes.len() > self.momentum_lookback + 1 {
            self.closes.pop_front();
        }

        if self.closes.len() <= self.momentum_lookback {
            self.value = 0.0;
            self.ready = false;
            return self.value;
        }

        let base = self.closes[0];
        let diff = value - base;
        let up = if diff > 0.0 { diff } else { 0.0 };
        let down = if diff < 0.0 { -diff } else { 0.0 };

        let up_avg = self.up_ma.feed(up);
        let down_avg = self.down_ma.feed(down);
        let denom = up_avg + down_avg;
        self.value = if denom > 0.0 {
            100.0 * (up_avg / denom)
        } else {
            50.0
        };
        self.ready = self.up_ma.is_ready() && self.down_ma.is_ready();
        self.value
    }

    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    pub fn reset(&mut self) {
        self.closes.clear();
        self.up_ma.reset();
        self.down_ma.reset();
        self.value = 0.0;
        self.ready = false;
    }

    pub fn momentum_lookback(&self) -> usize {
        self.momentum_lookback
    }

    pub fn ema_period(&self) -> usize {
        self.ema_period
    }
}

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherId;
use crate::engine::contract_engine::SmootherChoice;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderSpec};

impl Rmi {
    /// Build an RMI from a smoother CHOICE (kind + follow/own period) at the host `ema_period`.
    /// Both up/down smoothers use the same choice; `Follow` resolves to `ema_period`.
    pub fn from_choice(choice: SmootherChoice, momentum_lookback: usize, ema_period: usize) -> Self {
        let m = momentum_lookback.max(1);
        let n = ema_period.max(1);
        Self {
            momentum_lookback: m,
            ema_period: n,
            up_ma: choice.build(n),
            down_ma: choice.build(n),
            closes: VecDeque::with_capacity(m + 1),
            value: 0.0,
            ready: false,
        }
    }
}

/// Typed contract config for [`Rmi`] — RSI computed on m-period momentum (bounded 0-100).
///
/// Dual-mode: every field is a `Param`. The `#[slot]` smoother is a `Param<SmootherChoice>`
/// (fed to both up/down legs).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct RmiConfig {
    pub momentum_lookback: Param<usize>,
    pub ema_period: Param<usize>,
    pub source: Param<OhlcvField>,
    #[slot]
    pub ma: Param<SmootherChoice>,
}

impl Indicator for Rmi {
    const ID: IndicatorId = IndicatorId::Rmi;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(1): a momentum gap fed to the two smoothers over a small deque; the smoother
    /// buffers land recursively through `SLOTS`.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    const SLOTS: &'static [Slot] = RmiConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Rmi)];
    type Config = RmiConfig;
    type Runtime = Rmi;

    fn create(cfg: RmiConfig) -> Rmi {
        let m = cfg.momentum_lookback.resolved().max(1);
        let n = cfg.ema_period.resolved().max(1);
        let choice = cfg.ma.resolved();
        Rmi {
            momentum_lookback: m,
            ema_period: n,
            up_ma: choice.build(n),
            down_ma: choice.build(n),
            closes: VecDeque::with_capacity(m + 1),
            value: 0.0,
            ready: false,
        }
    }

    /// Field-source core: the factory variant holds `cfg.source` and feeds the resolved scalar.
    fn source_fields(cfg: &RmiConfig) -> arrayvec::ArrayVec<crate::engine::ohlcv_field::OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &RmiConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for RmiConfig {
    fn defaults() -> Self {
        RmiConfig {
            momentum_lookback: Param::Solo(5),
            ema_period: Param::Solo(14),
            source: Param::Solo(OhlcvField::Close),
            ma: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // momentum_lookback/ema_period: Class A → auto range(2,4048,1); source: Class O → auto all-8.
        // ma: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for Rmi {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Rmi, "RMI", Color::hex(0x9C27B0))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(70.0, Color::hex(0x9E9E9E)))
            .reference_line(ReferenceLine::new(30.0, Color::hex(0x9E9E9E)))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rmi_creation() {
        let rmi = Rmi::new(5, 14);
        assert!(!rmi.is_ready());
        assert_eq!(rmi.value(), 0.0);
        assert_eq!(rmi.momentum_lookback(), 5);
        assert_eq!(rmi.ema_period(), 14);
    }

    #[test]
    fn test_rmi_with_smoother() {
        let rmi = Rmi::from_smoother(5, 14, SmootherId::Ema);
        assert!(!rmi.is_ready());
    }

    #[test]
    fn test_rmi_uptrend() {
        let mut rmi = Rmi::new(5, 10);
        for i in 1..=50 {
            rmi.feed(100.0 + i as f64 * 2.0);
        }
        assert!(rmi.is_ready());
        // In uptrend, RMI should be > 50
        assert!(rmi.value() > 50.0, "RMI should be > 50 in uptrend, got {}", rmi.value());
    }

    #[test]
    fn test_rmi_downtrend() {
        let mut rmi = Rmi::new(5, 10);
        for i in 1..=50 {
            rmi.feed(200.0 - i as f64 * 2.0);
        }
        assert!(rmi.is_ready());
        // In downtrend, RMI should be < 50
        assert!(rmi.value() < 50.0, "RMI should be < 50 in downtrend, got {}", rmi.value());
    }

    #[test]
    fn test_rmi_range_bounds() {
        let mut rmi = Rmi::new(5, 10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = rmi.feed(price);
            if rmi.is_ready() {
                assert!(value >= 0.0 && value <= 100.0, "RMI should be in [0, 100], got {}", value);
            }
        }
    }

    #[test]
    fn test_rmi_reset() {
        let mut rmi = Rmi::new(5, 10);
        for i in 1..=50 {
            rmi.feed(100.0 + i as f64);
        }
        assert!(rmi.is_ready());
        rmi.reset();
        assert!(!rmi.is_ready());
        assert_eq!(rmi.value(), 0.0);
    }

    #[test]
    fn test_rmi_finite_values() {
        let mut rmi = Rmi::new(5, 10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            let value = rmi.feed(price);
            assert!(value.is_finite(), "RMI should always be finite");
        }
    }

    #[test]
    fn test_rmi_contract_create() {
        let cfg = <<Rmi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.ema_period.resolved(), 14);
        let mut rmi = <Rmi as Indicator>::create(cfg);
        for i in 1..=50 {
            rmi.feed(100.0 + i as f64 * 2.0);
        }
        assert!(rmi.is_ready());
    }
}
