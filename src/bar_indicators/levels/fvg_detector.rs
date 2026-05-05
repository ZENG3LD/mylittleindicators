// Fair Value Gap (FVG) detector over 3-bar pattern

use crate::bar_indicators::indicator_value::IndicatorValue;
use std::collections::VecDeque;

#[derive(Clone)]
pub struct FvgDetector {
    pub bull_fvg: bool,
    pub bear_fvg: bool,
    buffer: VecDeque<(f64, f64, f64, f64)>,  // (open, high, low, close)
}

impl Default for FvgDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl FvgDetector {
    pub fn new() -> Self {
        Self {
            bull_fvg: false,
            bear_fvg: false,
            buffer: VecDeque::with_capacity(3),
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.bull_fvg = false;
        self.bear_fvg = false;
        self.buffer.clear();
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.buffer.len() >= 3
    }

    /// Update with OHLCV bar
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, _volume: f64) {
        // Maintain 3-bar ring buffer
        if self.buffer.len() >= 3 {
            self.buffer.pop_front();
        }
        self.buffer.push_back((open, high, low, close));

        // Need minimum 3 bars for triplet
        if self.buffer.len() >= 3 {
            let (o0, h0, l0, c0) = self.buffer[0];
            let (o1, h1, l1, c1) = self.buffer[1];
            let (o2, h2, l2, c2) = self.buffer[2];

            self.update_triplet(
                o0, h0, l0, c0,
                o1, h1, l1, c1,
                o2, h2, l2, c2,
            );
        }
    }

    // pattern: bull FVG if low[1] > high[0] and low[1] > high[2]
    // bear FVG if high[1] < low[0] and high[1] < low[2]
    pub fn update_triplet(
        &mut self,
        _o0: f64,
        h0: f64,
        l0: f64,
        _c0: f64,
        _o1: f64,
        h1: f64,
        l1: f64,
        _c1: f64,
        _o2: f64,
        h2: f64,
        l2: f64,
        _c2: f64,
    ) -> (bool, bool) {
        let bull = l1 > h0 && l1 > h2;
        let bear = h1 < l0 && h1 < l2;
        self.bull_fvg = bull;
        self.bear_fvg = bear;
        (bull, bear)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::DoubleFlag(self.bull_fvg, self.bear_fvg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fvg_detector_creation() {
        let fvg = FvgDetector::new();
        assert!(!fvg.is_ready()); // Not ready until 3 bars
        assert!(!fvg.bull_fvg);
        assert!(!fvg.bear_fvg);
    }

    #[test]
    fn test_fvg_detector_bull_fvg() {
        let mut fvg = FvgDetector::new();
        // Bull FVG: low[1] > high[0] and low[1] > high[2]
        // Bar 0: high=100, Bar 1: low=105, Bar 2: high=102
        let (bull, bear) = fvg.update_triplet(
            99.0, 100.0, 98.0, 99.5, // Bar 0
            104.0, 108.0, 105.0, 107.0, // Bar 1 (gap up)
            101.0, 102.0, 100.0, 101.5, // Bar 2
        );
        assert!(bull, "Should detect bullish FVG");
        assert!(!bear);
    }

    #[test]
    fn test_fvg_detector_bear_fvg() {
        let mut fvg = FvgDetector::new();
        // Bear FVG: high[1] < low[0] and high[1] < low[2]
        // Bar 0: low=100, Bar 1: high=95, Bar 2: low=98
        let (bull, bear) = fvg.update_triplet(
            101.0, 102.0, 100.0, 101.5, // Bar 0
            92.0, 95.0, 90.0, 93.0, // Bar 1 (gap down)
            99.0, 100.0, 98.0, 99.5, // Bar 2
        );
        assert!(!bull);
        assert!(bear, "Should detect bearish FVG");
    }

    #[test]
    fn test_fvg_detector_no_fvg() {
        let mut fvg = FvgDetector::new();
        // Normal overlapping bars - no FVG
        let (bull, bear) = fvg.update_triplet(
            100.0, 102.0, 99.0, 101.0, // Bar 0
            101.0, 103.0, 100.0, 102.0, // Bar 1
            102.0, 104.0, 101.0, 103.0, // Bar 2
        );
        assert!(!bull);
        assert!(!bear);
    }

    #[test]
    fn test_fvg_detector_reset() {
        let mut fvg = FvgDetector::new();
        fvg.bull_fvg = true;
        fvg.bear_fvg = true;
        fvg.reset();
        assert!(!fvg.bull_fvg);
        assert!(!fvg.bear_fvg);
    }
}
