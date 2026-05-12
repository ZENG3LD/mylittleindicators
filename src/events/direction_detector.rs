//! DirectionDetector primitive — fires on each bar that the value changes direction
//! relative to the previous bar.
//!
//! Maps to `OperatorClass::Direction`.

use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{SignalKind, TrendSub};

/// Detects up/down direction changes of a scalar value.
#[derive(Debug, Clone)]
pub struct DirectionDetector {
    prev: Option<f64>,
}

impl DirectionDetector {
    pub fn new() -> Self {
        Self { prev: None }
    }

    /// Detect direction from a pre-computed value (slice-based hot loop).
    ///
    /// Returns `Some((SignalKind::Trend(TrendSub::PriceCross), Direction::Up))`
    /// when `value > prev`, `Direction::Down` when `value < prev`, `None` when equal
    /// or on the first call.
    pub fn detect_from_values(&mut self, value: f64) -> Option<(SignalKind, Direction)> {
        let result = match self.prev {
            None => None,
            Some(p) if value > p => {
                Some((SignalKind::Trend(TrendSub::PriceCross), Direction::Up))
            }
            Some(p) if value < p => {
                Some((SignalKind::Trend(TrendSub::PriceCross), Direction::Down))
            }
            _ => None,
        };
        self.prev = Some(value);
        result
    }

    /// Reset state.
    pub fn reset(&mut self) {
        self.prev = None;
    }
}

impl Default for DirectionDetector {
    fn default() -> Self {
        Self::new()
    }
}
