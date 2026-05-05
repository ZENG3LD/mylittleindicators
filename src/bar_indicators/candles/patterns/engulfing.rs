//! Engulfing Pattern Detector
//!
//! Detects Engulfing candlestick pattern - strong reversal signal where:
//! - Second candle's body completely engulfs the first candle's body
//! - Candles have opposite colors (bullish after bearish, or vice versa)
//! - Bullish Engulfing: bearish candle followed by larger bullish candle
//! - Bearish Engulfing: bullish candle followed by larger bearish candle

use serde::{Deserialize, Serialize};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Result of Engulfing pattern detection
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct EngulfingResult {
    pub detected: bool,
    pub is_bullish: bool,     // true if bullish engulfing, false if bearish
    pub strength: f64,         // 0.0-1.0: pattern quality
    pub size_ratio: f64,       // Second body / first body ratio
}

/// Engulfing pattern detector
#[derive(Debug, Clone)]
pub struct Engulfing {
    min_size_ratio: f64,  // Minimum size ratio (default 1.2 = 20% larger)
    prev_bar: Option<(f64, f64, f64, f64)>,  // Previous bar (open, high, low, close)
    last_value: f64,      // Cached last detection value for on-fly mode
}

impl Default for Engulfing {
    fn default() -> Self {
        Self::new(1.2)
    }
}

impl Engulfing {
    /// Creates a new Engulfing detector
    ///
    /// # Arguments
    /// * `min_size_ratio` - Minimum ratio of second body to first body (1.0-3.0, default 1.2)
    pub fn new(min_size_ratio: f64) -> Self {
        Self {
            min_size_ratio: min_size_ratio.clamp(1.0, 3.0),
            prev_bar: None,
            last_value: 0.0,
        }
    }

    /// Detects Engulfing pattern on two consecutive candles
    ///
    /// # Arguments
    /// * `prev_open` - Previous candle opening price
    /// * `prev_high` - Previous candle high price
    /// * `prev_low` - Previous candle low price
    /// * `prev_close` - Previous candle closing price
    /// * `curr_open` - Current candle opening price
    /// * `curr_high` - Current candle high price
    /// * `curr_low` - Current candle low price
    /// * `curr_close` - Current candle closing price
    ///
    /// # Returns
    /// EngulfingResult with detection status, direction, and strength
    pub fn detect(
        &self,
        prev_open: f64,
        _prev_high: f64,
        _prev_low: f64,
        prev_close: f64,
        curr_open: f64,
        _curr_high: f64,
        _curr_low: f64,
        curr_close: f64,
    ) -> EngulfingResult {
        let prev_body = (prev_close - prev_open).abs();
        let curr_body = (curr_close - curr_open).abs();

        if prev_body == 0.0 || curr_body == 0.0 {
            return EngulfingResult {
                detected: false,
                is_bullish: curr_close > curr_open,
                strength: 0.0,
                size_ratio: 0.0,
            };
        }

        let size_ratio = curr_body / prev_body;

        // Check size requirement
        if size_ratio < self.min_size_ratio {
            return EngulfingResult {
                detected: false,
                is_bullish: curr_close > curr_open,
                strength: 0.0,
                size_ratio,
            };
        }

        let prev_body_top = prev_open.max(prev_close);
        let prev_body_bottom = prev_open.min(prev_close);
        let curr_body_top = curr_open.max(curr_close);
        let curr_body_bottom = curr_open.min(curr_close);

        // Check if current body engulfs previous body
        let is_engulfed = curr_body_bottom < prev_body_bottom && curr_body_top > prev_body_top;

        if !is_engulfed {
            return EngulfingResult {
                detected: false,
                is_bullish: curr_close > curr_open,
                strength: 0.0,
                size_ratio,
            };
        }

        // Check opposite colors
        let prev_is_bullish = prev_close > prev_open;
        let curr_is_bullish = curr_close > curr_open;

        if prev_is_bullish == curr_is_bullish {
            return EngulfingResult {
                detected: false,
                is_bullish: curr_is_bullish,
                strength: 0.0,
                size_ratio,
            };
        }

        // Calculate strength based on size ratio
        let strength = ((size_ratio - self.min_size_ratio) / 1.8)
            .min(0.7)
            + 0.3;

        EngulfingResult {
            detected: true,
            is_bullish: curr_is_bullish,
            strength: strength.clamp(0.0, 1.0),
            size_ratio,
        }
    }

    /// Updates configuration
    pub fn set_min_size_ratio(&mut self, ratio: f64) {
        self.min_size_ratio = ratio.clamp(1.0, 3.0);
    }

    /// Updates the indicator with a new bar and returns detection value
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        if let Some((prev_open, prev_high, prev_low, prev_close)) = self.prev_bar {
            let result = self.detect(
                prev_open, prev_high, prev_low, prev_close,
                open, high, low, close,
            );
            self.last_value = if result.detected {
                result.strength
            } else {
                0.0
            };
        } else {
            self.last_value = 0.0;
        }

        // Store current bar as previous for next iteration
        self.prev_bar = Some((open, high, low, close));
        self.last_value
    }

    /// Returns the cached last detection value
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_value)
    }

    /// Checks if the indicator is ready to produce values (needs at least 2 bars)
    pub fn is_ready(&self) -> bool {
        self.prev_bar.is_some()
    }

    /// Resets the indicator state
    pub fn reset(&mut self) {
        self.prev_bar = None;
        self.last_value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bullish_engulfing() {
        let engulfing = Engulfing::default();
        // Bearish candle: 100-99 (body 1)
        // Bullish candle: 98-102 (body 4, engulfs previous)
        let result = engulfing.detect(100.0, 100.5, 98.5, 99.0, 98.0, 102.5, 97.5, 102.0);
        assert!(result.detected);
        assert!(result.is_bullish);
        assert!(result.size_ratio > 1.2);
    }

    #[test]
    fn test_bearish_engulfing() {
        let engulfing = Engulfing::default();
        // Bullish candle: 99-100 (body 1)
        // Bearish candle: 102-98 (body 4, engulfs previous)
        let result = engulfing.detect(99.0, 100.5, 98.5, 100.0, 102.0, 102.5, 97.5, 98.0);
        assert!(result.detected);
        assert!(!result.is_bullish);
        assert!(result.size_ratio > 1.2);
    }

    #[test]
    fn test_not_engulfing_same_direction() {
        let engulfing = Engulfing::default();
        // Both bullish - should not detect
        let result = engulfing.detect(99.0, 100.0, 98.0, 100.0, 100.0, 104.0, 99.0, 104.0);
        assert!(!result.detected);
    }

    #[test]
    fn test_not_engulfing_too_small() {
        let engulfing = Engulfing::default();
        // Second candle not large enough
        let result = engulfing.detect(99.0, 100.0, 98.0, 100.0, 100.0, 101.0, 99.5, 99.5);
        assert!(!result.detected);
    }
}
