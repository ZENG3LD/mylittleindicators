//! ZoneExit primitive — fires when a value leaves a defined zone `[lower, upper]`.
//!
//! Maps to `OperatorClass::ZoneExit`.

use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{SignalKind, ThresholdSub};

/// Detects exit from a price/indicator zone.
#[derive(Debug, Clone)]
pub struct ZoneExit {
    lower: f64,
    upper: f64,
    was_inside: bool,
}

impl ZoneExit {
    pub fn new(lower: f64, upper: f64) -> Self {
        Self {
            lower,
            upper,
            was_inside: false,
        }
    }

    /// Detect zone exit from a pre-computed value (slice-based hot loop).
    ///
    /// Returns `Some((SignalKind::Threshold(ThresholdSub::Exit), Direction::Up))`
    /// when value exits above `upper`, `Direction::Down` when exiting below `lower`.
    pub fn detect_from_values(&mut self, value: f64) -> Option<(SignalKind, Direction)> {
        let inside = value >= self.lower && value <= self.upper;
        let exited = self.was_inside && !inside;
        self.was_inside = inside;
        if exited {
            let dir = if value > self.upper {
                Direction::Up
            } else {
                Direction::Down
            };
            Some((SignalKind::Threshold(ThresholdSub::Exit), dir))
        } else {
            None
        }
    }

    /// Reset state.
    pub fn reset(&mut self) {
        self.was_inside = false;
    }
}
