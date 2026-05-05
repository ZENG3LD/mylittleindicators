//! Morning Star and Evening Star Pattern Detector
use serde::{Deserialize, Serialize};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MorningStarResult {
    pub detected: bool,
    pub is_bullish: bool,
    pub strength: f64,
}

#[derive(Debug, Clone)]
pub struct MorningStar {
    max_star_ratio: f64,
    bars: Vec<(f64, f64, f64, f64)>,  // Buffer for last 3 bars (open, high, low, close)
    last_value: f64,                   // Cached last detection value for on-fly mode
}

impl Default for MorningStar {
    fn default() -> Self {
        Self {
            max_star_ratio: 0.3,
            bars: Vec::with_capacity(3),
            last_value: 0.0,
        }
    }
}

impl MorningStar {
    pub fn new(max_star_ratio: f64) -> Self {
        Self {
            max_star_ratio,
            bars: Vec::with_capacity(3),
            last_value: 0.0,
        }
    }
    
    pub fn detect(&self, f_o: f64, f_h: f64, f_l: f64, f_c: f64,
                  s_o: f64, s_h: f64, s_l: f64, s_c: f64,
                  t_o: f64, t_h: f64, t_l: f64, t_c: f64) -> MorningStarResult {
        let avg_range = ((f_h-f_l)+(s_h-s_l)+(t_h-t_l))/3.0;
        if avg_range == 0.0 { return MorningStarResult{detected:false,is_bullish:false,strength:0.0}; }
        let star_ratio = (s_c-s_o).abs()/avg_range;
        if star_ratio > self.max_star_ratio { return MorningStarResult{detected:false,is_bullish:false,strength:0.0}; }
        
        if f_c<f_o && t_c>t_o && s_h<f_c.min(f_o) && t_c>(f_o+f_c)/2.0 {
            return MorningStarResult{detected:true,is_bullish:true,strength:0.8};
        }
        if f_c>f_o && t_c<t_o && s_l>f_c.max(f_o) && t_c<(f_o+f_c)/2.0 {
            return MorningStarResult{detected:true,is_bullish:false,strength:0.8};
        }
        MorningStarResult{detected:false,is_bullish:false,strength:0.0}
    }

    /// Updates the indicator with a new bar and returns detection value
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        // Add current bar to buffer
        self.bars.push((open, high, low, close));

        // Keep only last 3 bars
        if self.bars.len() > 3 {
            self.bars.remove(0);
        }

        // Detect pattern if we have 3 bars
        if self.bars.len() == 3 {
            let (f_o, f_h, f_l, f_c) = self.bars[0];
            let (s_o, s_h, s_l, s_c) = self.bars[1];
            let (t_o, t_h, t_l, t_c) = self.bars[2];

            let result = self.detect(f_o, f_h, f_l, f_c, s_o, s_h, s_l, s_c, t_o, t_h, t_l, t_c);
            self.last_value = if result.detected {
                result.strength
            } else {
                0.0
            };
        } else {
            self.last_value = 0.0;
        }

        self.last_value
    }

    /// Returns the cached last detection value
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_value)
    }

    /// Checks if the indicator is ready to produce values (needs at least 3 bars)
    pub fn is_ready(&self) -> bool {
        self.bars.len() >= 3
    }

    /// Resets the indicator state
    pub fn reset(&mut self) {
        self.bars.clear();
        self.last_value = 0.0;
    }
}
// Type aliases removed - EveningStar is now a standalone wrapper struct
// pub type EveningStar = MorningStar;
// pub type EveningStarResult = MorningStarResult;
