//! Fair Value Gap (FVG) event detector.
//!
//! Detects 3-bar imbalance pattern:
//! - Bullish FVG: `low[middle] > high[older]` AND `low[middle] > high[newer]`
//! - Bearish FVG: `high[middle] < low[older]` AND `high[middle] < low[newer]`
//!
//! Output: `Option<(SignalKind::Structure(StructureSub::FVG), Direction)>`
//! - `Direction::Up` for bullish FVG, `Direction::Down` for bearish.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::signal::kind::StructureSub;
use crate::core::signal::{Direction, SignalKind};
use std::collections::VecDeque;

/// Fair Value Gap event detector.
///
/// Buffers last 3 bars and emits a typed signal when a FVG pattern is detected.
/// The detector fires on the bar that *completes* the triplet (bar index 2 = newest).
#[derive(Clone, Debug)]
pub struct FvgEventDetector {
    /// Ring buffer of `(high, low)` for the 3-bar window.
    bars: VecDeque<(f64, f64)>,
    last_signal: i8,
}

impl FvgEventDetector {
    pub fn new() -> Self {
        Self {
            bars: VecDeque::with_capacity(3),
            last_signal: 0,
        }
    }

    /// Feed a new bar and return the typed signal if a FVG pattern is detected.
    pub fn detect_from_values(&mut self, high: f64, low: f64) -> Option<(SignalKind, Direction)> {
        self.bars.push_back((high, low));
        if self.bars.len() > 3 {
            self.bars.pop_front();
        }
        if self.bars.len() < 3 {
            self.last_signal = 0;
            return None;
        }

        let (h0, l0) = self.bars[0]; // older
        let (h1, l1) = self.bars[1]; // middle
        let (h2, l2) = self.bars[2]; // newer

        // Bull FVG: low[middle] > high[older] && low[middle] > high[newer]
        let bull = l1 > h0 && l1 > h2;
        // Bear FVG: high[middle] < low[older] && high[middle] < low[newer]
        let bear = h1 < l0 && h1 < l2;

        if bull {
            self.last_signal = 1;
            Some((SignalKind::Structure(StructureSub::FVG), Direction::Up))
        } else if bear {
            self.last_signal = -1;
            Some((SignalKind::Structure(StructureSub::FVG), Direction::Down))
        } else {
            self.last_signal = 0;
            None
        }
    }

    /// Detect from an explicit OHLC triplet.
    ///
    /// Used by scoring indicators (FVGDUR, FVGALT, FVGREV) that manage their own
    /// 3-bar buffer and call this method directly. Returns `(bull, bear)` flags for
    /// backward compatibility with those callers.
    #[allow(clippy::too_many_arguments)]
    pub fn update_triplet(
        &mut self,
        _o0: f64, h0: f64, l0: f64, _c0: f64,
        _o1: f64, h1: f64, l1: f64, _c1: f64,
        _o2: f64, h2: f64, l2: f64, _c2: f64,
    ) -> (bool, bool) {
        let bull = l1 > h0 && l1 > h2;
        let bear = h1 < l0 && h1 < l2;
        self.last_signal = if bull { 1 } else if bear { -1 } else { 0 };
        (bull, bear)
    }

    /// Update with a full OHLCV bar; returns legacy `IndicatorValue::Signal`.
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, _c: f64, _v: f64) -> IndicatorValue {
        match self.detect_from_values(h, l) {
            Some((_, Direction::Up)) => IndicatorValue::Signal(1),
            Some((_, Direction::Down)) => IndicatorValue::Signal(-1),
            _ => IndicatorValue::Signal(0),
        }
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    pub fn is_ready(&self) -> bool {
        self.bars.len() == 3
    }

    pub fn reset(&mut self) {
        self.bars.clear();
        self.last_signal = 0;
    }
}

impl Default for FvgEventDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bull_fvg_detected() {
        let mut det = FvgEventDetector::new();
        // Bar 0: high=100, low=98
        // Bar 1: high=108, low=105  — low[1]=105 > high[0]=100 ✓
        // Bar 2: high=102, low=100  — low[1]=105 > high[2]=102 ✓
        det.detect_from_values(100.0, 98.0);
        det.detect_from_values(108.0, 105.0);
        let result = det.detect_from_values(102.0, 100.0);
        assert_eq!(
            result,
            Some((SignalKind::Structure(StructureSub::FVG), Direction::Up)),
            "should detect bullish FVG"
        );
    }

    #[test]
    fn bear_fvg_detected() {
        let mut det = FvgEventDetector::new();
        // Bar 0: high=102, low=100
        // Bar 1: high=95,  low=90   — high[1]=95 < low[0]=100 ✓
        // Bar 2: high=100, low=98   — high[1]=95 < low[2]=98  ✓
        det.detect_from_values(102.0, 100.0);
        det.detect_from_values(95.0, 90.0);
        let result = det.detect_from_values(100.0, 98.0);
        assert_eq!(
            result,
            Some((SignalKind::Structure(StructureSub::FVG), Direction::Down)),
            "should detect bearish FVG"
        );
    }

    #[test]
    fn no_gap_returns_none() {
        let mut det = FvgEventDetector::new();
        // Overlapping bars — no gap
        det.detect_from_values(102.0, 99.0);
        det.detect_from_values(103.0, 100.0);
        let result = det.detect_from_values(104.0, 101.0);
        assert_eq!(result, None, "overlapping bars should produce no FVG");
    }

    #[test]
    fn not_ready_until_three_bars() {
        let mut det = FvgEventDetector::new();
        assert!(!det.is_ready());
        det.detect_from_values(100.0, 99.0);
        assert!(!det.is_ready());
        det.detect_from_values(101.0, 100.0);
        assert!(!det.is_ready());
        det.detect_from_values(102.0, 101.0);
        assert!(det.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut det = FvgEventDetector::new();
        det.detect_from_values(100.0, 98.0);
        det.detect_from_values(108.0, 105.0);
        det.detect_from_values(102.0, 100.0);
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.value(), IndicatorValue::Signal(0));
    }
}
