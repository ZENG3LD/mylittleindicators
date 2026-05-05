//! Evening Star Pattern Detector
//!
//! Detects the bearish Evening Star pattern using AdvancedPatternRecognition.

use crate::bar_indicators::candles::pattern_recognition::{AdvancedPatternRecognition, PatternType};
use crate::bar_indicators::indicator_value::IndicatorValue;


#[derive(Clone)]
pub struct EveningStar {
    recognizer: AdvancedPatternRecognition,
    value: f64,
}

impl Default for EveningStar {
    fn default() -> Self {
        Self::new()
    }
}

impl EveningStar {
    pub fn new() -> Self {
        Self {
            recognizer: AdvancedPatternRecognition::new(),
            value: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.recognizer = AdvancedPatternRecognition::new();
        self.value = 0.0;
    }

    pub fn is_ready(&self) -> bool {
        true
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let result = self.recognizer.update_bar(o, h, l, c, v);
        self.value = if result.pattern_type == PatternType::EveningStar {
            result.confidence
        } else {
            0.0
        };
        self.value
    }
}
