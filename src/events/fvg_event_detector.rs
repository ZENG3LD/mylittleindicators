//! Fair Value Gap (FVG) event detector.
//!
//! Detects the 3-bar imbalance pattern between the FIRST and THIRD bars of
//! the triplet. The middle (displacement) candle's own high/low take no
//! part in the test:
//! - Bullish FVG: `low[newer] > high[older]`, zone `[high[older], low[newer]]`
//! - Bearish FVG: `high[newer] < low[older]`, zone `[high[newer], low[older]]`
//!
//! Corrected 2026-08-25 — the prior condition tested
//! `low[middle] > high[older] && low[middle] > high[newer]`, which
//! describes a bar gapped away from BOTH neighbours (an island), not a fair
//! value gap. See Appendix A, `docs/mlc/plans/markup-engines-2026-08-25.md`.
//!
//! `fvg_direction` is the shared predicate this streaming detector and
//! `smc::zones::fair_value_gaps` both call, so the two can never disagree
//! about what a FVG is again.
//!
//! Output: `Option<(SignalKind::Structure(StructureSub::FVG), Direction)>`
//! - `Direction::Up` for bullish FVG, `Direction::Down` for bearish.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::signal::kind::StructureSub;
use crate::core::signal::{Direction, SignalKind};
use std::collections::VecDeque;

/// Pure fair-value-gap predicate shared by `FvgEventDetector` and
/// `smc::zones::fair_value_gaps`. Bullish requires the newer bar's low
/// above the older bar's high; bearish requires the newer bar's high below
/// the older bar's low — the middle (displacement) candle plays no part.
pub(crate) fn fvg_direction(
    older_high: f64,
    older_low: f64,
    newer_high: f64,
    newer_low: f64,
) -> (bool, bool) {
    let bull = newer_low > older_high;
    let bear = newer_high < older_low;
    (bull, bear)
}

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
        let (h2, l2) = self.bars[2]; // newer — self.bars[1] (middle) plays no part in the test

        let (bull, bear) = fvg_direction(h0, l0, h2, l2);

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
        _o1: f64, _h1: f64, _l1: f64, _c1: f64,
        _o2: f64, h2: f64, l2: f64, _c2: f64,
    ) -> (bool, bool) {
        let (bull, bear) = fvg_direction(h0, l0, h2, l2);
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
        // Bar 0 (older): high=100, low=98
        // Bar 1 (middle/displacement): high=108, low=105 — plays no part in the test
        // Bar 2 (newer): high=112, low=103 — low[newer]=103 > high[older]=100 ✓
        det.detect_from_values(100.0, 98.0);
        det.detect_from_values(108.0, 105.0);
        let result = det.detect_from_values(112.0, 103.0);
        assert_eq!(
            result,
            Some((SignalKind::Structure(StructureSub::FVG), Direction::Up)),
            "should detect bullish FVG"
        );
    }

    #[test]
    fn bear_fvg_detected() {
        let mut det = FvgEventDetector::new();
        // Bar 0 (older): high=102, low=100
        // Bar 1 (middle/displacement): high=95, low=90 — plays no part in the test
        // Bar 2 (newer): high=97, low=90 — high[newer]=97 < low[older]=100 ✓
        det.detect_from_values(102.0, 100.0);
        det.detect_from_values(95.0, 90.0);
        let result = det.detect_from_values(97.0, 90.0);
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
