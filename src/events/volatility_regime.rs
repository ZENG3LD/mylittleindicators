//! VolatilityRegimeDetector primitive — classifies volatility into Low/Normal/High
//! regimes and fires on regime transitions.
//!
//! Accepts ATR, StdDev, or any scalar volatility measure.
//! Maps to `OperatorClass::VolatilityRegime`.

use crate::core::signal::direction::Direction;
use crate::core::signal::kind::{SignalKind, VolatilitySub};

/// Volatility regime level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolatilityLevel {
    /// Below `low_threshold`.
    Low,
    /// Between `low_threshold` and `high_threshold`.
    Normal,
    /// Above `high_threshold`.
    High,
}

/// Detects transitions between volatility regimes.
#[derive(Debug, Clone)]
pub struct VolatilityRegimeDetector {
    low_threshold: f64,
    high_threshold: f64,
    current: Option<VolatilityLevel>,
}

impl VolatilityRegimeDetector {
    /// `low_threshold`: boundary between Low and Normal.
    /// `high_threshold`: boundary between Normal and High.
    pub fn new(low_threshold: f64, high_threshold: f64) -> Self {
        Self {
            low_threshold,
            high_threshold,
            current: None,
        }
    }

    fn classify(&self, value: f64) -> VolatilityLevel {
        if value < self.low_threshold {
            VolatilityLevel::Low
        } else if value > self.high_threshold {
            VolatilityLevel::High
        } else {
            VolatilityLevel::Normal
        }
    }

    /// Detect volatility regime from a pre-computed ATR/StdDev value (slice-based hot loop).
    ///
    /// Fires only on regime transitions:
    /// - Transition into High → `Volatility(Extreme)`, `Direction::Up`
    /// - Transition out of High → `Volatility(Shift)`, `Direction::Down`
    /// - Transition into Low → `Volatility(Squeeze)`, `Direction::Down`
    /// - Transition out of Low → `Volatility(Shift)`, `Direction::Up`
    pub fn detect_from_values(&mut self, atr_or_stddev: f64) -> Option<(SignalKind, Direction)> {
        let new_level = self.classify(atr_or_stddev);
        let prev = self.current;
        self.current = Some(new_level);

        let prev_level = match prev {
            None => return None,
            Some(l) => l,
        };

        if prev_level == new_level {
            return None;
        }

        let result = match (prev_level, new_level) {
            (_, VolatilityLevel::High) => {
                Some((SignalKind::Volatility(VolatilitySub::Extreme), Direction::Up))
            }
            (VolatilityLevel::High, _) => {
                Some((SignalKind::Volatility(VolatilitySub::Shift), Direction::Down))
            }
            (_, VolatilityLevel::Low) => {
                Some((SignalKind::Volatility(VolatilitySub::Squeeze), Direction::Down))
            }
            (VolatilityLevel::Low, _) => {
                Some((SignalKind::Volatility(VolatilitySub::Shift), Direction::Up))
            }
            _ => None,
        };
        result
    }

    /// Current regime level (None until first value fed).
    pub fn current_level(&self) -> Option<VolatilityLevel> {
        self.current
    }

    /// Reset state.
    pub fn reset(&mut self) {
        self.current = None;
    }
}
