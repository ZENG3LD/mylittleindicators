//! Three White Soldiers and Three Black Crows Detector
use serde::{Deserialize, Serialize};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThreeWhiteSoldiersResult {
    pub detected: bool,
    pub is_bullish: bool,
    pub strength: f64,
}

#[derive(Debug, Clone)]
pub struct ThreeWhiteSoldiers {
    min_body_ratio: f64,
    bars: Vec<(f64, f64, f64, f64)>,  // Buffer for last 3 bars (open, high, low, close)
    last_value: f64,                   // Cached last detection value for on-fly mode
}

impl Default for ThreeWhiteSoldiers {
    fn default() -> Self {
        Self {
            min_body_ratio: 0.6,
            bars: Vec::with_capacity(3),
            last_value: 0.0,
        }
    }
}

impl ThreeWhiteSoldiers {
    pub fn new(min_body_ratio: f64) -> Self {
        Self {
            min_body_ratio,
            bars: Vec::with_capacity(3),
            last_value: 0.0,
        }
    }
    
    pub fn detect(&self, c1o:f64,c1h:f64,c1l:f64,c1c:f64, c2o:f64,c2h:f64,c2l:f64,c2c:f64, c3o:f64,c3h:f64,c3l:f64,c3c:f64) -> ThreeWhiteSoldiersResult {
        let candles = [(c1o,c1h,c1l,c1c),(c2o,c2h,c2l,c2c),(c3o,c3h,c3l,c3c)];
        
        let mut is_white = true;
        for i in 0..3 {
            let (o,h,l,c) = candles[i];
            if c <= o || (c-o).abs()/(h-l) < self.min_body_ratio { is_white = false; break; }
            if i > 0 && (o <= candles[i-1].3 || c <= candles[i-1].3*1.05) { is_white = false; break; }
        }
        if is_white { return ThreeWhiteSoldiersResult{detected:true,is_bullish:true,strength:0.85}; }
        
        let mut is_black = true;
        for i in 0..3 {
            let (o,h,l,c) = candles[i];
            if c >= o || (c-o).abs()/(h-l) < self.min_body_ratio { is_black = false; break; }
            if i > 0 && (o >= candles[i-1].3 || c >= candles[i-1].3/1.05) { is_black = false; break; }
        }
        if is_black { return ThreeWhiteSoldiersResult{detected:true,is_bullish:false,strength:0.85}; }

        ThreeWhiteSoldiersResult{detected:false,is_bullish:false,strength:0.0}
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
            let (c1o, c1h, c1l, c1c) = self.bars[0];
            let (c2o, c2h, c2l, c2c) = self.bars[1];
            let (c3o, c3h, c3l, c3c) = self.bars[2];

            let result = self.detect(c1o, c1h, c1l, c1c, c2o, c2h, c2l, c2c, c3o, c3h, c3l, c3c);
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
// Type aliases removed - ThreeBlackCrows is now a standalone wrapper struct
// pub type ThreeBlackCrows = ThreeWhiteSoldiers;
// pub type ThreeBlackCrowsResult = ThreeWhiteSoldiersResult;
