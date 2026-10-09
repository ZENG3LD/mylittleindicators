//! Threshold primitive — detects when a value crosses or enters/exits a
//! threshold or range boundary.
//!
//! Maps to `OperatorClass::ThresholdCompare`.

use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{SignalKind, ThresholdSub};

/// Which threshold geometry to check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(mli_contract_macros::ParamScalar)]
pub enum ThresholdKind {
    /// Fire when value rises above `upper`.
    Above,
    /// Fire when value falls below `lower`.
    Below,
    /// Fire when value enters the range `[lower, upper]`.
    InRange,
    /// Fire when value exits the range `[lower, upper]`.
    OutOfRange,
}

/// Detects threshold crossings and zone transitions.
#[derive(Debug, Clone)]
pub struct Threshold {
    kind: ThresholdKind,
    upper: f64,
    lower: f64,
    last_state: Option<bool>,
    last_signal: i8,
}

impl Threshold {
    /// Create with explicit upper and lower bounds.
    ///
    /// For `Above` / `Below`, only the relevant bound is used.
    pub fn new(kind: ThresholdKind, upper: f64, lower: f64) -> Self {
        Self {
            kind,
            upper,
            lower,
            last_state: None,
            last_signal: 0,
        }
    }

    /// Feed ONE resolved scalar (the configured source, default close). Returns the
    /// threshold-crossing edge signal (+1 / -1 on the transition bar, 0 otherwise).
    pub fn feed(&mut self, value: f64) {
        self.last_signal = match self.detect_from_values(value) {
            Some((_, Direction::Up)) => 1,
            Some((_, Direction::Down)) => -1,
            _ => 0,
        };
    }

    /// The last threshold edge signal.
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }

    /// Ready once it has seen its first value.
    pub fn is_ready(&self) -> bool {
        self.last_state.is_some()
    }

    /// Create for single-level comparison (above/below). `level` is stored as both bounds.
    pub fn single(kind: ThresholdKind, level: f64) -> Self {
        Self::new(kind, level, level)
    }

    /// Detect threshold event from a pre-computed value (slice-based hot loop).
    ///
    /// Fires on the transition bar only (not on every bar while condition holds).
    /// Direction::Up = moved above threshold / entered zone from below.
    /// Direction::Down = moved below threshold / exited zone downward.
    pub fn detect_from_values(&mut self, value: f64) -> Option<(SignalKind, Direction)> {
        let current_state = match self.kind {
            ThresholdKind::Above => value > self.upper,
            ThresholdKind::Below => value < self.lower,
            ThresholdKind::InRange => value >= self.lower && value <= self.upper,
            ThresholdKind::OutOfRange => value < self.lower || value > self.upper,
        };

        let prev = self.last_state;
        self.last_state = Some(current_state);

        match prev {
            None => None,
            Some(was) if !was && current_state => {
                let sub = match self.kind {
                    ThresholdKind::Above | ThresholdKind::InRange => ThresholdSub::Enter,
                    ThresholdKind::Below | ThresholdKind::OutOfRange => ThresholdSub::Exit,
                };
                let dir = if value >= self.lower {
                    Direction::Up
                } else {
                    Direction::Down
                };
                Some((SignalKind::Threshold(sub), dir))
            }
            Some(was) if was && !current_state => {
                let sub = match self.kind {
                    ThresholdKind::Above | ThresholdKind::InRange => ThresholdSub::Exit,
                    ThresholdKind::Below | ThresholdKind::OutOfRange => ThresholdSub::Enter,
                };
                let dir = if value < self.lower {
                    Direction::Down
                } else {
                    Direction::Up
                };
                Some((SignalKind::Threshold(sub), dir))
            }
            _ => None,
        }
    }

    /// Reset detector state.
    pub fn reset(&mut self) {
        self.last_state = None;
        self.last_signal = 0;
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`Threshold`] — geometry kind + the upper/lower bounds. A scalar
/// edge detector over a configurable source (default close); the events variant
/// (distinct from the self-contained `ThresholdGate` / `IndicatorId::Thresh`).
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ThresholdConfig {
    pub kind: Param<ThresholdKind>,
    pub upper: Param<f64>,
    pub lower: Param<f64>,
}

impl Indicator for Threshold {
    const ID: IndicatorId = IndicatorId::ThreshEdge;
    /// No family — an atomic threshold-crossing DETECTOR, consumed by name.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// O(1) per bar; one boolean of prior-zone state.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::ordinal(IndicatorOutputId::ThreshEdge)];
    type Config = ThresholdConfig;
    type Runtime = Threshold;

    fn create(cfg: ThresholdConfig) -> Threshold {
        Threshold::new(cfg.kind.resolved(), cfg.upper.resolved(), cfg.lower.resolved())
    }
}

impl crate::contract::Config for ThresholdConfig {
    fn defaults() -> Self {
        ThresholdConfig {
            kind: Param::Solo(ThresholdKind::Above),
            upper: Param::Solo(70.0),
            lower: Param::Solo(30.0),
        }
    }
    fn machine_defaults() -> Self {
        use crate::contract::{Param, sweep_f64};
        let mut s = Self::machine_defaults_auto();
        // kind: Class Q — all ThresholdKind variants.
        s.kind = Param::many(vec![
            ThresholdKind::Above,
            ThresholdKind::Below,
            ThresholdKind::InRange,
            ThresholdKind::OutOfRange,
        ]);
        // upper/lower: defaults are 70/30 → oscillator 0-100 space.
        // Class F sub-case: upper in OB zone sweep_f64(50,95,1); lower in OS zone sweep_f64(5,50,1).
        s.upper = Param::many(sweep_f64(50.0, 95.0, 1.0));
        s.lower = Param::many(sweep_f64(5.0, 50.0, 1.0));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for Threshold {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::ThreshEdge, "Threshold Edge", Color::hex(0xFF5722))
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
    fn factory_feeds_resolved_threshold() {
        use crate::contract::Param;
        let mut cfg = <<Threshold as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        cfg.kind = Param::Solo(ThresholdKind::Above);
        cfg.upper = Param::Solo(100.0);
        let mut f = IndicatorOrder::ThreshEdge(cfg).build_solo().unwrap();
        let bar = |c: f64| MarketSample::Bar { open: c, high: c, low: c, close: c, volume: 1.0 };
        f.feed(0, bar(90.0)); // init below upper
        f.feed(0, bar(110.0));
        assert_eq!(f.primary(), 1.0, "rising above upper = +1 edge");
    }
}
