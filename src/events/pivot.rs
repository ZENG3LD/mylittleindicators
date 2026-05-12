//! Pivot primitive — N-bar high/low pivot detector.
//!
//! Confirms a pivot-high at position `n` if its value is the highest
//! in a `[left + 1 + right]` window centered on that position.
//! Similarly for pivot-low (minimum).
//!
//! Maps to `OperatorClass::Pivot`.

use std::collections::VecDeque;

use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{SignalKind, StructureSub};

/// N-bar pivot detector (left + right confirmation bars).
#[derive(Debug, Clone)]
pub struct Pivot {
    left: usize,
    right: usize,
    buffer: VecDeque<f64>,
}

impl Pivot {
    /// `left`: bars before the candidate pivot.
    /// `right`: bars after (confirmation lag).
    pub fn new(left: usize, right: usize) -> Self {
        let left = left.max(1);
        let right = right.max(1);
        Self {
            left,
            right,
            buffer: VecDeque::with_capacity(left + right + 1),
        }
    }

    /// Feed one value and return a signal if a pivot is confirmed.
    ///
    /// Returns `Some((SignalKind::Structure(StructureSub::OrderBlock), Direction::Up))`
    /// for pivot-high, `Direction::Down` for pivot-low.
    ///
    /// Note: result lags by `right` bars — the signal fires on the bar that
    /// provides the right-side confirmation, not on the pivot bar itself.
    pub fn detect_from_values(&mut self, value: f64) -> Option<(SignalKind, Direction)> {
        self.buffer.push_back(value);
        let needed = self.left + self.right + 1;
        if self.buffer.len() < needed {
            return None;
        }
        // Keep buffer at exactly `needed` length.
        if self.buffer.len() > needed {
            self.buffer.pop_front();
        }

        let pivot_idx = self.left; // 0-indexed center of window
        let pivot_val = self.buffer[pivot_idx];

        let is_high = (0..needed).all(|i| i == pivot_idx || self.buffer[i] <= pivot_val);
        let is_low = (0..needed).all(|i| i == pivot_idx || self.buffer[i] >= pivot_val);

        if is_high && !is_low {
            Some((SignalKind::Structure(StructureSub::OrderBlock), Direction::Up))
        } else if is_low && !is_high {
            Some((SignalKind::Structure(StructureSub::OrderBlock), Direction::Down))
        } else {
            None
        }
    }

    /// Reset buffer.
    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}
