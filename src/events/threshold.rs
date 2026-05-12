//! Threshold primitive — detects when a value crosses or enters/exits a
//! threshold or range boundary.
//!
//! Maps to `OperatorClass::ThresholdCompare`.

use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{SignalKind, ThresholdSub};

/// Which threshold geometry to check.
#[derive(Debug, Clone, Copy, PartialEq)]
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
        }
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
    }
}
