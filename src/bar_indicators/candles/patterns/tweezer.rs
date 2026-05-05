//! Tweezer Pattern Detector
use serde::{Deserialize, Serialize};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TweezerResult {
    pub detected: bool,
    pub is_bullish: bool,
    pub strength: f64,
}

#[derive(Debug, Clone)]
pub struct Tweezer {
    max_diff_ratio: f64,
    prev_bar: Option<(f64, f64, f64, f64)>,  // Previous bar (open, high, low, close)
    last_value: f64,                          // Cached last detection value for on-fly mode
}

impl Default for Tweezer {
    fn default() -> Self {
        Self {
            max_diff_ratio: 0.02,
            prev_bar: None,
            last_value: 0.0,
        }
    }
}

impl Tweezer {
    pub fn new(max_diff_ratio: f64) -> Self {
        Self {
            max_diff_ratio,
            prev_bar: None,
            last_value: 0.0,
        }
    }
    
    pub fn detect(&self, po:f64,ph:f64,pl:f64,pc:f64, co:f64,ch:f64,cl:f64,cc:f64) -> TweezerResult {
        let avg_range = ((ph-pl)+(ch-cl))/2.0;
        if avg_range == 0.0 { return TweezerResult{detected:false,is_bullish:false,strength:0.0}; }
        
        if (ph-ch).abs()/avg_range < self.max_diff_ratio && pc>po && cc<co {
            return TweezerResult{detected:true,is_bullish:false,strength:0.75};
        }
        if (pl-cl).abs()/avg_range < self.max_diff_ratio && pc<po && cc>co {
            return TweezerResult{detected:true,is_bullish:true,strength:0.75};
        }
        TweezerResult{detected:false,is_bullish:false,strength:0.0}
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
