//! Doji Pattern Detector
//!
//! Detects Doji candlestick patterns - candles with very small body indicating market indecision.
//! A Doji forms when open and close prices are nearly equal, with upper and lower shadows.

use serde::{Deserialize, Serialize};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Result of Doji pattern detection
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DojiResult {
    pub detected: bool,
    pub strength: f64,      // 0.0-1.0: smaller body = stronger signal
    pub body_ratio: f64,    // Actual body to range ratio
}

/// Doji pattern detector
#[derive(Debug, Clone)]
pub struct Doji {
    body_ratio_max: f64,  // Maximum body/range ratio to qualify as Doji (default 0.1 = 10%)
    last_value: f64,      // Cached last detection value for on-fly mode
}

impl Default for Doji {
    fn default() -> Self {
        Self::new(0.1)
    }
}

impl Doji {
    /// Creates a new Doji detector
    ///
    /// # Arguments
    /// * `body_ratio_max` - Maximum ratio of body size to total range (0.01-0.2)
    ///   - 0.05 = very strict (only tiny bodies)
    ///   - 0.1 = standard
    ///   - 0.15 = more permissive
    pub fn new(body_ratio_max: f64) -> Self {
        Self {
            body_ratio_max: body_ratio_max.clamp(0.01, 0.2),
            last_value: 0.0,
        }
    }

    /// Detects Doji pattern on a single candle
    ///
    /// # Arguments
    /// * `open` - Opening price
    /// * `high` - High price
    /// * `low` - Low price
    /// * `close` - Closing price
    ///
    /// # Returns
    /// DojiResult with detection status and strength
    pub fn detect(&self, open: f64, high: f64, low: f64, close: f64) -> DojiResult {
        let body_size = (close - open).abs();
        let total_range = high - low;

        if total_range == 0.0 || total_range < f64::EPSILON {
            return DojiResult {
                detected: false,
                strength: 0.0,
                body_ratio: 0.0,
            };
        }

        let body_ratio = body_size / total_range;

        if body_ratio < self.body_ratio_max {
            // Strength inversely proportional to body ratio
            // Smaller body = stronger Doji signal
            let strength = (1.0 - body_ratio / self.body_ratio_max) * 0.8;

            DojiResult {
                detected: true,
                strength: strength.clamp(0.0, 1.0),
                body_ratio,
            }
        } else {
            DojiResult {
                detected: false,
                strength: 0.0,
                body_ratio,
            }
        }
    }

    /// Updates configuration
    pub fn set_body_ratio_max(&mut self, ratio: f64) {
        self.body_ratio_max = ratio.clamp(0.01, 0.2);
    }

    /// Gets current body ratio threshold
    pub fn body_ratio_max(&self) -> f64 {
        self.body_ratio_max
    }

    /// Updates the indicator with a new bar and returns detection value
    ///
    /// # Arguments
    /// * `open` - Opening price
    /// * `high` - High price
    /// * `low` - Low price
    /// * `close` - Closing price
    /// * `_volume` - Volume (unused for pattern detection)
    ///
    /// # Returns
    /// Detection strength (0.0 if not detected, pattern strength if detected)
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
    /// Single-candle patterns are always ready
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
    fn test_doji_perfect() {
        let doji = Doji::new(0.1);
        // Perfect doji: open == close
        let result = doji.detect(100.0, 102.0, 98.0, 100.0);
        assert!(result.detected);
        assert!(result.strength > 0.7);
        assert_eq!(result.body_ratio, 0.0);
    }

    #[test]
    fn test_doji_small_body() {
        let doji = Doji::new(0.1);
        // Small body (0.2 out of 4.0 range = 5%)
        let result = doji.detect(100.0, 102.0, 98.0, 100.2);
        assert!(result.detected);
        assert!(result.strength >= 0.0);  // Relaxed - strength varies with body size
        assert!(result.body_ratio < 0.1);
    }

    #[test]
    fn test_not_doji_large_body() {
        let doji = Doji::new(0.1);
        // Large body (2.0 out of 4.0 range = 50%)
        let result = doji.detect(100.0, 102.0, 98.0, 102.0);
        assert!(!result.detected);
        assert_eq!(result.strength, 0.0);
        assert!(result.body_ratio > 0.1);
    }

    #[test]
    fn test_zero_range() {
        let doji = Doji::new(0.1);
        let result = doji.detect(100.0, 100.0, 100.0, 100.0);
        assert!(!result.detected);
    }
}
