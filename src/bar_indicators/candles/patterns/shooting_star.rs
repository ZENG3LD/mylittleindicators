//! Shooting Star Pattern Detector
//!
//! Detects Shooting Star candlestick pattern - bearish reversal signal with:
//! - Small body at the bottom of the range
//! - Long upper shadow (at least 2x body size)
//! - Little to no lower shadow

use serde::{Deserialize, Serialize};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Result of Shooting Star pattern detection
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ShootingStarResult {
    pub detected: bool,
    pub strength: f64,           // 0.0-1.0: pattern quality
    pub upper_shadow_ratio: f64, // Upper shadow / body ratio
    pub body_position: f64,      // Body position in range (0=bottom, 1=top)
}

/// Shooting Star pattern detector
#[derive(Debug, Clone)]
pub struct ShootingStar {
    shadow_ratio_min: f64,         // Minimum upper shadow / body ratio (default 2.0)
    opposite_shadow_max: f64,      // Maximum lower shadow / body ratio (default 0.5)
    min_body_to_range: f64,        // Minimum body size relative to range (default 0.1)
    last_value: f64,               // Cached last detection value for on-fly mode
}

impl Default for ShootingStar {
    fn default() -> Self {
        Self::new(2.0, 0.5)
    }
}

impl ShootingStar {
    /// Creates a new Shooting Star detector
    ///
    /// # Arguments
    /// * `shadow_ratio_min` - Minimum ratio of upper shadow to body (1.5-5.0, default 2.0)
    /// * `opposite_shadow_max` - Maximum ratio of lower shadow to body (0.0-1.0, default 0.5)
    pub fn new(shadow_ratio_min: f64, opposite_shadow_max: f64) -> Self {
        Self {
            shadow_ratio_min: shadow_ratio_min.clamp(1.5, 5.0),
            opposite_shadow_max: opposite_shadow_max.clamp(0.0, 1.0),
            min_body_to_range: 0.1,
            last_value: 0.0,
        }
    }

    /// Detects Shooting Star pattern on a single candle
    ///
    /// # Arguments
    /// * `open` - Opening price
    /// * `high` - High price
    /// * `low` - Low price
    /// * `close` - Closing price
    ///
    /// # Returns
    /// ShootingStarResult with detection status and metrics
    pub fn detect(&self, open: f64, high: f64, low: f64, close: f64) -> ShootingStarResult {
        let body_size = (close - open).abs();
        let total_range = high - low;

        if total_range == 0.0 || total_range < f64::EPSILON {
            return ShootingStarResult {
                detected: false,
                strength: 0.0,
                upper_shadow_ratio: 0.0,
                body_position: 0.0,
            };
        }

        let body_top = open.max(close);
        let body_bottom = open.min(close);
        let upper_shadow = high - body_top;
        let lower_shadow = body_bottom - low;

        // Check minimum body size
        let body_to_range = body_size / total_range;
        if body_to_range < self.min_body_to_range {
            return ShootingStarResult {
                detected: false,
                strength: 0.0,
                upper_shadow_ratio: if body_size > 0.0 { upper_shadow / body_size } else { 0.0 },
                body_position: if total_range > 0.0 { (body_bottom - low) / total_range } else { 0.0 },
            };
        }

        let upper_shadow_ratio = upper_shadow / body_size;
        let lower_shadow_ratio = lower_shadow / body_size;
        let body_position = (body_bottom - low) / total_range;

        // Check Shooting Star criteria
        let is_shooting_star = upper_shadow_ratio >= self.shadow_ratio_min
            && lower_shadow_ratio <= self.opposite_shadow_max;

        if is_shooting_star {
            // Calculate strength based on shadow ratio and body position
            let shadow_strength = ((upper_shadow_ratio - self.shadow_ratio_min) / 3.0)
                .min(1.0);
            let position_strength = 1.0 - body_position; // Lower position = stronger
            let strength = (shadow_strength * 0.5 + position_strength * 0.5) * 0.85;

            ShootingStarResult {
                detected: true,
                strength: strength.clamp(0.0, 1.0),
                upper_shadow_ratio,
                body_position,
            }
        } else {
            ShootingStarResult {
                detected: false,
                strength: 0.0,
                upper_shadow_ratio,
                body_position,
            }
        }
    }

    /// Updates the indicator with a new bar and returns detection value
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        let result = self.detect(open, high, low, close);
        self.last_value = if result.detected {
            result.strength
        } else {
            0.0
        };
        self.last_value
    }

    /// Returns the cached last detection value
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_value)
    }

    /// Checks if the indicator is ready to produce values
    pub fn is_ready(&self) -> bool {
        true
    }

    /// Resets the indicator state
    pub fn reset(&mut self) {
        self.last_value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shooting_star_perfect() {
        let detector = ShootingStar::default();
        // Perfect shooting star: small body at bottom, long upper shadow
        // Body: 95-96 (size 1), Range: 95-101 (size 6), Upper shadow: 5
        let result = detector.detect(96.0, 101.0, 95.0, 95.0);
        assert!(result.detected);
        assert!(result.upper_shadow_ratio >= 2.0);
        assert!(result.body_position < 0.4);
    }

    #[test]
    fn test_not_shooting_star_short_shadow() {
        let detector = ShootingStar::default();
        // Not enough upper shadow
        let result = detector.detect(99.0, 101.0, 95.0, 100.0);
        assert!(!result.detected);
    }
}
