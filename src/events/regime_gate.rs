//! RegimeGate primitive — fires when a regime indicator transitions above/below a threshold.
//!
//! Maps to `OperatorClass::RegimeGate`.

use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{CompositeSub, SignalKind};

/// Which side of the threshold constitutes "in regime".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

impl RegimeGate {
    pub fn new(regime_threshold: f64, direction: GateDirection) -> Self {
        Self {
            regime_threshold,
            direction,
            in_regime: false,
            initialized: false,
        }
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
    }
}
