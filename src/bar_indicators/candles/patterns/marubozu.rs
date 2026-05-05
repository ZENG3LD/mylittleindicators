//! Marubozu Pattern Detector
//!
//! Detects Marubozu candlestick pattern - strong trend continuation signal with:
//! - Large body (>95% of total range)
//! - Little to no shadows
//! - Bullish (white) or Bearish (black) versions

use serde::{Deserialize, Serialize};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Result of Marubozu pattern detection
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MarubozuResult {
    pub detected: bool,
    pub is_bullish: bool,    // true if white/bullish, false if black/bearish
    pub strength: f64,        // 0.0-1.0: pattern quality
    pub body_ratio: f64,      // Body size / total range ratio
}

/// Marubozu pattern detector
#[derive(Debug, Clone)]
pub struct Marubozu {
    body_ratio_min: f64,  // Minimum body/range ratio (default 0.95 = 95%)
    last_value: f64,      // Cached last detection value for on-fly mode
}

impl Default for Marubozu {
    fn default() -> Self {
        Self::new(0.95)
    }
}

impl Marubozu {
    /// Creates a new Marubozu detector
    ///
    /// # Arguments
    /// * `body_ratio_min` - Minimum ratio of body size to total range (0.90-0.99, default 0.95)
    pub fn new(body_ratio_min: f64) -> Self {
        Self {
            body_ratio_min: body_ratio_min.clamp(0.90, 0.99),
            last_value: 0.0,
        }
    }

    /// Detects Marubozu pattern on a single candle
    ///
    /// # Arguments
    /// * `open` - Opening price
    /// * `high` - High price
    /// * `low` - Low price
    /// * `close` - Closing price
    ///
    /// # Returns
    /// MarubozuResult with detection status, direction, and strength
    pub fn detect(&self, open: f64, high: f64, low: f64, close: f64) -> MarubozuResult {
        let body_size = (close - open).abs();
        let total_range = high - low;

        if total_range == 0.0 || total_range < f64::EPSILON {
            return MarubozuResult {
                detected: false,
                is_bullish: close > open,
                strength: 0.0,
                body_ratio: 0.0,
            };
        }

        let body_ratio = body_size / total_range;
        let is_bullish = close > open;

        if body_ratio >= self.body_ratio_min {
            // Strength based on how close to perfect Marubozu (100% body)
            let strength = ((body_ratio - self.body_ratio_min) / (1.0 - self.body_ratio_min)) * 0.8 + 0.2;

            MarubozuResult {
                detected: true,
                is_bullish,
                strength: strength.clamp(0.0, 1.0),
                body_ratio,
            }
        } else {
            MarubozuResult {
                detected: false,
                is_bullish,
                strength: 0.0,
                body_ratio,
            }
        }
    }

    /// Updates configuration
    pub fn set_body_ratio_min(&mut self, ratio: f64) {
        self.body_ratio_min = ratio.clamp(0.90, 0.99);
    }

    /// Gets current body ratio threshold
    pub fn body_ratio_min(&self) -> f64 {
        self.body_ratio_min
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
    fn test_marubozu_perfect_bullish() {
        let marubozu = Marubozu::default();
        // Perfect bullish marubozu: open=low, close=high
        let result = marubozu.detect(100.0, 105.0, 100.0, 105.0);
        assert!(result.detected);
        assert!(result.is_bullish);
        assert_eq!(result.body_ratio, 1.0);
        assert!(result.strength > 0.8);
    }

    #[test]
    fn test_marubozu_perfect_bearish() {
        let marubozu = Marubozu::default();
        // Perfect bearish marubozu: open=high, close=low
        let result = marubozu.detect(105.0, 105.0, 100.0, 100.0);
        assert!(result.detected);
        assert!(!result.is_bullish);
        assert_eq!(result.body_ratio, 1.0);
        assert!(result.strength > 0.8);
    }

    #[test]
    fn test_marubozu_near_perfect() {
        let marubozu = Marubozu::default();
        // Near perfect: 4.8 body out of 5.0 range (96%)
        let result = marubozu.detect(100.1, 105.0, 100.0, 104.9);
        assert!(result.detected);
        assert!(result.is_bullish);
        assert!(result.body_ratio >= 0.95);
    }

    #[test]
    fn test_not_marubozu_small_body() {
        let marubozu = Marubozu::default();
        // Small body: 3.0 out of 5.0 range (60%)
        let result = marubozu.detect(101.0, 105.0, 100.0, 104.0);
        assert!(!result.detected);
        assert!(result.body_ratio < 0.95);
    }
}
