//! RegimeGate primitive — fires when a regime indicator transitions above/below a threshold.
//!
//! Maps to `OperatorClass::RegimeGate`.

use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{CompositeSub, SignalKind};

/// Which side of the threshold constitutes "in regime".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(mli_contract_macros::ParamScalar)]
pub enum GateDirection {
    /// Regime is active when `regime_value > threshold`.
    Above,
    /// Regime is active when `regime_value < threshold`.
    Below,
}

/// Detects entry/exit into a market regime defined by a threshold.
#[derive(Debug, Clone)]
pub struct RegimeGate {
    regime_threshold: f64,
    direction: GateDirection,
    in_regime: bool,
    initialized: bool,
    last_signal: i8,
}

impl RegimeGate {
    pub fn new(regime_threshold: f64, direction: GateDirection) -> Self {
        Self {
            regime_threshold,
            direction,
            in_regime: false,
            initialized: false,
            last_signal: 0,
        }
    }

    /// Feed ONE resolved scalar (the configured source, default close). Returns the
    /// regime transition signal (+1 entry / -1 exit / 0 none).
    pub fn feed(&mut self, value: f64) {
        self.last_signal = match self.detect_from_values(value) {
            Some((_, Direction::Up)) => 1,
            Some((_, Direction::Down)) => -1,
            _ => 0,
        };
    }

    /// The last regime transition signal.
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }

    /// Ready once it has seen its first value (regime state initialized).
    pub fn is_ready(&self) -> bool {
        self.initialized
    }

    /// Detect regime gate transitions from a pre-computed regime value (slice-based hot loop).
    ///
    /// Returns `Some((SignalKind::Composite(CompositeSub::Confirmed), Direction::Up))`
    /// on regime entry, `Direction::Down` on regime exit.
    pub fn detect_from_values(&mut self, regime_value: f64) -> Option<(SignalKind, Direction)> {
        let now_in = match self.direction {
            GateDirection::Above => regime_value > self.regime_threshold,
            GateDirection::Below => regime_value < self.regime_threshold,
        };

        if !self.initialized {
            self.in_regime = now_in;
            self.initialized = true;
            return None;
        }

        let prev = self.in_regime;
        self.in_regime = now_in;

        if !prev && now_in {
            Some((SignalKind::Composite(CompositeSub::Confirmed), Direction::Up))
        } else if prev && !now_in {
            Some((SignalKind::Composite(CompositeSub::Confirmed), Direction::Down))
        } else {
            None
        }
    }

    /// Whether the detector is currently inside a regime.
    pub fn in_regime(&self) -> bool {
        self.in_regime
    }

    /// Reset state.
    pub fn reset(&mut self) {
        self.in_regime = false;
        self.initialized = false;
        self.last_signal = 0;
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, UpdateComplexity, sweep_f64};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`RegimeGate`] — the regime threshold and which side is "in regime".
/// A scalar gate over a configurable source (default close); compose it with an upstream
/// oscillator by choosing that source — the inner-fed form is a future output-slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RegimeGateConfig {
    pub regime_threshold: Param<f64>,
    pub direction: Param<GateDirection>,
}

impl Indicator for RegimeGate {
    const ID: IndicatorId = IndicatorId::RegimeGate;
    /// No family — an atomic regime-transition DETECTOR, consumed by name.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// O(1) per bar; two booleans of state.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::RegimeGate)];
    type Config = RegimeGateConfig;
    type Runtime = RegimeGate;

    fn create(cfg: RegimeGateConfig) -> RegimeGate {
        RegimeGate::new(cfg.regime_threshold.resolved(), cfg.direction.resolved())
    }
}

impl crate::contract::Config for RegimeGateConfig {
    fn defaults() -> Self {
        RegimeGateConfig {
            regime_threshold: Param::Solo(0.5),
            direction: Param::Solo(GateDirection::Above),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // regime_threshold: Class F sigma/threshold — sweep_f64(0.1, 5.0, 0.1).
        s.regime_threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        // direction: Class Q plain enum — all GateDirection variants.
        s.direction = Param::many(vec![GateDirection::Above, GateDirection::Below]);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for RegimeGate {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::RegimeGate, "Regime Gate", Color::hex(0x795548))
            .bounds(-1.0, 1.0)
            .zero_baseline()
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_regime_gate() {
        let cfg = RegimeGateConfig {
            regime_threshold: Param::Solo(100.0),
            direction: Param::Solo(GateDirection::Above),
        };
        let mut f = IndicatorOrder::RegimeGate(cfg).build_solo().unwrap();
        let bar = |c: f64| MarketSample::Bar { open: c, high: c, low: c, close: c, volume: 1.0 };
        f.feed(0, bar(90.0)); // init below
        f.feed(0, bar(110.0));
        assert_eq!(f.primary(), 1.0, "crossing above threshold = entry +1");
        f.feed(0, bar(90.0));
        assert_eq!(f.primary(), -1.0, "crossing back below = exit -1");
    }
}
