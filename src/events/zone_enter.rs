//! ZoneEnter primitive — fires when a value moves into a defined zone `[lower, upper]`.
//!
//! Maps to `OperatorClass::ZoneEnter`.

use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{SignalKind, ThresholdSub};

/// Detects entry into a price/indicator zone.
#[derive(Debug, Clone)]
pub struct ZoneEnter {
    lower: f64,
    upper: f64,
    was_inside: bool,
}

impl ZoneEnter {
    pub fn new(lower: f64, upper: f64) -> Self {
        Self {
            lower,
            upper,
            was_inside: false,
        }
    }

    /// Detect zone entry from a pre-computed value (slice-based hot loop).
    ///
    /// Returns `Some((SignalKind::Threshold(ThresholdSub::Enter), Direction::Up))`
    /// on the bar the value first enters `[lower, upper]` from below or above.
    pub fn detect_from_values(&mut self, value: f64) -> Option<(SignalKind, Direction)> {
        let inside = value >= self.lower && value <= self.upper;
        let fired = !self.was_inside && inside;
        self.was_inside = inside;
        if fired {
            Some((SignalKind::Threshold(ThresholdSub::Enter), Direction::Up))
        } else {
            None
        }
    }

    /// Reset state.
    pub fn reset(&mut self) {
        self.was_inside = false;
    }
}
