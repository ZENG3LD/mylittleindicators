//! VolumeEventDetector primitive — detects volume spikes relative to a rolling average.
//!
//! Maps to `OperatorClass::VolumeEvent`.

use std::collections::VecDeque;

use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{SignalKind, VolumeSub};

/// Detects volume spikes as multiples of a rolling mean.
#[derive(Debug, Clone)]
pub struct VolumeEventDetector {
    /// Spike fires when `volume > multiplier * rolling_mean`.
    multiplier: f64,
    /// Rolling window of past volume values.
    history: VecDeque<f64>,
    /// Window period for the rolling mean.
    period: usize,
}

impl VolumeEventDetector {
    /// `period`: number of bars for rolling average.
    /// `multiplier`: volume must be `multiplier × average` to fire (e.g. `2.0`).
    pub fn new(period: usize, multiplier: f64) -> Self {
        let period = period.max(1);
        Self {
            multiplier,
            history: VecDeque::with_capacity(period),
            period,
        }
    }

    /// Detect volume event from a pre-computed volume value (slice-based hot loop).
    ///
    /// Returns `Some((SignalKind::Volume(VolumeSub::Spike), Direction::Up))`
    /// when `volume > multiplier × rolling_mean(period)`.
    /// Returns `None` during warmup (fewer than `period` bars seen).
    pub fn detect_from_values(&mut self, volume: f64) -> Option<(SignalKind, Direction)> {
        if self.history.len() >= self.period {
            self.history.pop_front();
        }
        self.history.push_back(volume);

        if self.history.len() < self.period {
            return None;
        }

        let mean: f64 = self.history.iter().sum::<f64>() / self.history.len() as f64;
        if mean <= 0.0 {
            return None;
        }

        if volume > self.multiplier * mean {
            Some((SignalKind::Volume(VolumeSub::Spike), Direction::Up))
        } else {
            None
        }
    }

    /// Reset history.
    pub fn reset(&mut self) {
        self.history.clear();
    }
}
