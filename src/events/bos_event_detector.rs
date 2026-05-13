//! Break of Structure (BOS) event detector.
//!
//! Detects when the current bar breaks the rolling extremum over a lookback window:
//! - Up break:   `current high > max(highs[window-1 preceding bars])`
//! - Down break: `current low  < min(lows[window-1 preceding bars])`
//!
//! NOTE: CHoCH (Change of Character) is NOT implemented — this is a simple
//! rolling-extremum breakout detector, not a swing-based structure tracker.
//!
//! The algorithm matches the original `BosChochDetector` exactly:
//! - Circular buffer of `lookback` slots
//! - Fills current slot BEFORE computing prev extremes (using `idx` before increment)
//! - `filled` flag is set once the write pointer wraps around once
//! - Prev extremes = max/min over the `lookback-1` slots BEFORE the current slot
//! - Initial `highs` fill = `0.0` (matches original); detection is stable after warmup

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::signal::kind::StructureSub;
use crate::core::signal::{Direction, SignalKind};

/// Break of Structure event detector.
///
/// Uses a circular buffer of `lookback` bars. After warmup (`is_ready() == true`),
/// emits `BOS Up` when current high beats the window maximum and `BOS Down` when
/// current low undercuts the window minimum.
#[derive(Clone, Debug)]
pub struct BosEventDetector {
    lookback: usize,
    highs: Vec<f64>,
    lows: Vec<f64>,
    idx: usize,
    filled: bool,
    last_signal: i8,
}

impl BosEventDetector {
    pub fn new(lookback: usize) -> Self {
        let lookback = lookback.max(2);
        Self {
            lookback,
            highs: vec![0.0; lookback],
            lows: vec![0.0; lookback],
            idx: 0,
            filled: false,
            last_signal: 0,
        }
    }

    /// Feed a new bar and return the typed signal if a BOS pattern is detected.
    ///
    /// Mirrors the original `BosChochDetector::update_bar` logic precisely:
    /// write current bar into slot, advance pointer, then compare current vs
    /// the remaining `lookback-1` previous slots.
    pub fn detect_from_values(&mut self, high: f64, low: f64) -> Option<(SignalKind, Direction)> {
        self.highs[self.idx] = high;
        self.lows[self.idx] = low;
        self.idx = (self.idx + 1) % self.lookback;
        if self.idx == 0 {
            self.filled = true;
        }
        if !self.filled {
            self.last_signal = 0;
            return None;
        }

        // Compare current bar against the lookback-1 preceding bars.
        // Current bar lives at slot `(self.idx + lookback - 1) % lookback`.
        let len = self.lookback;
        let mut prev_max = f64::MIN;
        let mut prev_min = f64::MAX;
        for k in 1..len {
            let i = (self.idx + len - 1 - k) % len;
            if self.highs[i] > prev_max {
                prev_max = self.highs[i];
            }
            if self.lows[i] < prev_min {
                prev_min = self.lows[i];
            }
        }

        let curr_i = (self.idx + len - 1) % len;
        let h = self.highs[curr_i];
        let l = self.lows[curr_i];

        if h > prev_max {
            self.last_signal = 1;
            Some((SignalKind::Structure(StructureSub::BOS), Direction::Up))
        } else if l < prev_min {
            self.last_signal = -1;
            Some((SignalKind::Structure(StructureSub::BOS), Direction::Down))
        } else {
            self.last_signal = 0;
            None
        }
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
        self.filled
    }

    pub fn reset(&mut self) {
        self.highs.fill(0.0);
        self.lows.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.last_signal = 0;
    }
}

impl Default for BosEventDetector {
    fn default() -> Self {
        Self::new(20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn warmup(det: &mut BosEventDetector, n: usize, high: f64, low: f64) {
        for _ in 0..n {
            det.detect_from_values(high, low);
        }
    }

    #[test]
    fn not_ready_during_warmup() {
        let mut det = BosEventDetector::new(5);
        assert!(!det.is_ready());
        warmup(&mut det, 4, 101.0, 99.0);
        assert!(!det.is_ready());
        det.detect_from_values(101.0, 99.0);
        assert!(det.is_ready());
    }

    #[test]
    fn bos_up_detected_after_warmup() {
        let mut det = BosEventDetector::new(5);
        // Warmup with 5 stable bars (high=101, low=99)
        warmup(&mut det, 5, 101.0, 99.0);
        assert!(det.is_ready());
        // A bar whose high clearly exceeds the window maximum (101)
        let result = det.detect_from_values(115.0, 110.0);
        assert_eq!(
            result,
            Some((SignalKind::Structure(StructureSub::BOS), Direction::Up)),
            "high breakout should yield BOS Up"
        );
    }

    #[test]
    fn bos_down_detected_after_warmup() {
        let mut det = BosEventDetector::new(5);
        warmup(&mut det, 5, 101.0, 99.0);
        // A bar whose low clearly undercuts the window minimum (99)
        let result = det.detect_from_values(96.0, 85.0);
        assert_eq!(
            result,
            Some((SignalKind::Structure(StructureSub::BOS), Direction::Down)),
            "low breakout should yield BOS Down"
        );
    }

    #[test]
    fn normal_bar_returns_none() {
        let mut det = BosEventDetector::new(5);
        warmup(&mut det, 5, 101.0, 99.0);
        // Bar strictly inside the established range
        let result = det.detect_from_values(100.5, 99.5);
        assert_eq!(result, None, "bar inside range should produce no BOS");
    }

    #[test]
    fn reset_clears_state() {
        let mut det = BosEventDetector::new(5);
        warmup(&mut det, 5, 101.0, 99.0);
        det.detect_from_values(115.0, 110.0);
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.value(), IndicatorValue::Signal(0));
    }
}
