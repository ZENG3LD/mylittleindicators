//! Piercing Pattern and Dark Cloud Cover Detector
//!
//! Detects two related reversal patterns:
//! - Piercing Pattern (bullish): bearish candle followed by bullish candle that closes above midpoint
//! - Dark Cloud Cover (bearish): bullish candle followed by bearish candle that closes below midpoint

use serde::{Deserialize, Serialize};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PiercingPatternResult {
    pub detected: bool,
    pub is_bullish: bool,
    pub strength: f64,
    pub penetration: f64,  // How far into first candle body (0.5 = at midpoint, 1.0 = full)
}

#[derive(Debug, Clone)]
pub struct PiercingPattern {
    min_penetration: f64,  // Minimum penetration into first body (default 0.5)
    prev_bar: Option<(f64, f64, f64, f64)>,  // Previous bar (open, high, low, close)
    last_value: f64,       // Cached last detection value for on-fly mode
}

impl Default for PiercingPattern {
    fn default() -> Self {
        Self::new(0.5)
    }
}

impl PiercingPattern {
    pub fn new(min_penetration: f64) -> Self {
        Self {
            min_penetration: min_penetration.clamp(0.3, 0.7),
            prev_bar: None,
            last_value: 0.0,
        }
    }

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
    ) -> PiercingPatternResult {
        let prev_body_top = prev_open.max(prev_close);
        let prev_body_bottom = prev_open.min(prev_close);
        let prev_body_mid = (prev_body_top + prev_body_bottom) / 2.0;
        let prev_body_size = (prev_close - prev_open).abs();

        if prev_body_size == 0.0 {
            return PiercingPatternResult {
                detected: false,
                is_bullish: false,
                strength: 0.0,
                penetration: 0.0,
            };
        }

        // Piercing Pattern: bearish first, bullish second, closes above midpoint
        if prev_close < prev_open && curr_close > curr_open
            && curr_open < prev_close && curr_close > prev_body_mid && curr_close < prev_open {
                let penetration = (curr_close - prev_body_bottom) / prev_body_size;
                if penetration >= self.min_penetration {
                    return PiercingPatternResult {
                        detected: true,
                        is_bullish: true,
                        strength: ((penetration - self.min_penetration) / 0.5 * 0.6 + 0.4).min(1.0),
                        penetration,
                    };
                }
            }

        // Dark Cloud Cover: bullish first, bearish second, closes below midpoint
        if prev_close > prev_open && curr_close < curr_open
            && curr_open > prev_close && curr_close < prev_body_mid && curr_close > prev_open {
                let penetration = (prev_body_top - curr_close) / prev_body_size;
                if penetration >= self.min_penetration {
                    return PiercingPatternResult {
                        detected: true,
                        is_bullish: false,
                        strength: ((penetration - self.min_penetration) / 0.5 * 0.6 + 0.4).min(1.0),
                        penetration,
                    };
                }
            }

        PiercingPatternResult {
            detected: false,
            is_bullish: false,
            strength: 0.0,
            penetration: 0.0,
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

// Type aliases removed - DarkCloudCover is now a standalone wrapper struct
// pub type DarkCloudCover = PiercingPattern;
// pub type DarkCloudCoverResult = PiercingPatternResult;
