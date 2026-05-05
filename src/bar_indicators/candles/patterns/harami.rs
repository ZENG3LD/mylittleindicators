//! Harami Pattern Detector
//!
//! Detects Harami candlestick pattern - reversal signal where:
//! - Second candle's body is completely inside the first candle's body
//! - First candle has a large body
//! - Second candle has a smaller body with opposite color
//! - Bullish Harami: large bearish candle followed by small bullish candle inside
//! - Bearish Harami: large bullish candle followed by small bearish candle inside

use serde::{Deserialize, Serialize};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Result of Harami pattern detection
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct HaramiResult {
    pub detected: bool,
    pub is_bullish: bool,  // true if bullish harami, false if bearish
    pub strength: f64,      // 0.0-1.0: pattern quality
    pub containment: f64,   // How centered the second body is (0.5 = perfectly centered)
}

/// Harami pattern detector
#[derive(Debug, Clone)]
pub struct Harami {
    min_first_body_ratio: f64,  // Minimum first candle body/range ratio (default 0.6)
    prev_bar: Option<(f64, f64, f64, f64)>,  // Previous bar (open, high, low, close)
    last_value: f64,      // Cached last detection value for on-fly mode
}

impl Default for Harami {
    fn default() -> Self {
        Self::new(0.6)
    }
}

impl Harami {
    /// Creates a new Harami detector
    ///
    /// # Arguments
    /// * `min_first_body_ratio` - Minimum first candle body ratio (0.5-0.8, default 0.6)
    pub fn new(min_first_body_ratio: f64) -> Self {
        Self {
            min_first_body_ratio: min_first_body_ratio.clamp(0.5, 0.8),
            prev_bar: None,
            last_value: 0.0,
        }
    }

    /// Detects Harami pattern on two consecutive candles
    pub fn detect(
        &self,
        prev_open: f64,
        prev_high: f64,
        prev_low: f64,
        prev_close: f64,
        curr_open: f64,
        _curr_high: f64,
        _curr_low: f64,
        curr_close: f64,
    ) -> HaramiResult {
        let prev_body = (prev_close - prev_open).abs();
        let prev_range = prev_high - prev_low;
        let curr_body = (curr_close - curr_open).abs();

        if prev_range == 0.0 || prev_body == 0.0 || curr_body == 0.0 {
            return HaramiResult {
                detected: false,
                is_bullish: prev_close < prev_open,
                strength: 0.0,
                containment: 0.0,
            };
        }

        // Check if first candle has substantial body
        let prev_body_ratio = prev_body / prev_range;
        if prev_body_ratio < self.min_first_body_ratio {
            return HaramiResult {
                detected: false,
                is_bullish: prev_close < prev_open,
                strength: 0.0,
                containment: 0.0,
            };
        }

        let prev_body_top = prev_open.max(prev_close);
        let prev_body_bottom = prev_open.min(prev_close);
        let curr_body_top = curr_open.max(curr_close);
        let curr_body_bottom = curr_open.min(curr_close);

        // Check if current body is inside previous body
        let is_inside = curr_body_top <= prev_body_top && curr_body_bottom >= prev_body_bottom;

        if !is_inside {
            return HaramiResult {
                detected: false,
                is_bullish: prev_close < prev_open,
                strength: 0.0,
                containment: 0.0,
            };
        }

        // Calculate how centered the second candle is
        let prev_body_mid = (prev_body_top + prev_body_bottom) / 2.0;
        let curr_body_mid = (curr_body_top + curr_body_bottom) / 2.0;
        let containment = 1.0 - ((prev_body_mid - curr_body_mid).abs() / (prev_body / 2.0)).min(1.0);

        let prev_is_bullish = prev_close > prev_open;
        let curr_is_bullish = curr_close > curr_open;
        let is_bullish = !prev_is_bullish && curr_is_bullish; // Bullish harami = bearish first, bullish second

        // Strength based on body size difference and centering
        let size_diff = 1.0 - (curr_body / prev_body);
        let strength = (size_diff * 0.5 + containment * 0.5) * 0.75;

        HaramiResult {
            detected: true,
            is_bullish,
            strength: strength.clamp(0.0, 1.0),
            containment,
        }
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
    fn test_bullish_harami() {
        let harami = Harami::default();
        // Large bearish: 105-100 (body 5)
        // Small bullish inside: 101-103 (body 2)
        let result = harami.detect(105.0, 105.5, 99.5, 100.0, 101.0, 103.5, 100.5, 103.0);
        assert!(result.detected);
        assert!(result.is_bullish);
    }

    #[test]
    fn test_bearish_harami() {
        let harami = Harami::default();
        // Large bullish: 100-105 (body 5)
        // Small bearish inside: 103-101 (body 2)
        let result = harami.detect(100.0, 105.5, 99.5, 105.0, 103.0, 103.5, 100.5, 101.0);
        assert!(result.detected);
        assert!(!result.is_bullish);
    }
}
