//! Cumulative Volume Delta — rolling sum of estimated buy/sell delta.
//!
//! Without a real tick stream the delta per bar is estimated from the candle
//! direction:
//!
//! - `close > open` → bullish bar → `+volume` (buy pressure)
//! - `close < open` → bearish bar → `−volume` (sell pressure)
//! - `close == open` → doji → `0`
//!
//! The rolling window (`window` bars) keeps the CVD anchored to recent
//! history rather than accumulating from the beginning of time.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;

/// Rolling Cumulative Volume Delta.
///
/// Output is `Single(cumulative_delta)` — an unbounded oscillator that
/// trends positive when buy pressure dominates and negative when sell
/// pressure dominates.
#[derive(Debug, Clone)]
pub struct CumulativeVolumeDelta {
    window: usize,
    delta_history: VecDeque<f64>,
    cumulative: f64,
}

impl CumulativeVolumeDelta {
    /// Create a new `CumulativeVolumeDelta` with the given rolling `window`.
    ///
    /// Minimum window is 1.
    pub fn new(window: usize) -> Self {
        let w = window.max(1);
        Self {
            window: w,
            delta_history: VecDeque::with_capacity(w + 1),
            cumulative: 0.0,
        }
    }

    /// Feed one OHLCV bar and return `Single(cumulative_delta)`.
    ///
    /// Uses a synthetic estimate: `delta = +volume` if close > open,
    /// `-volume` if close < open, else `0`.
    pub fn update_bar(&mut self, open: f64, _high: f64, _low: f64, close: f64, volume: f64) -> IndicatorValue {
        const EPS: f64 = 1e-12;
        let delta = if close > open + EPS {
            volume
        } else if close < open - EPS {
            -volume
        } else {
            0.0
        };

        self.delta_history.push_back(delta);
        if self.delta_history.len() > self.window {
            if let Some(old) = self.delta_history.pop_front() {
                self.cumulative -= old;
            }
        }
        self.cumulative += delta;

        IndicatorValue::Single(self.cumulative)
    }

    /// Returns the last computed value without advancing state.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.cumulative)
    }

    /// Returns `true` after at least one bar has been fed.
    pub fn is_ready(&self) -> bool {
        !self.delta_history.is_empty()
    }

    /// Clears all accumulated state.
    pub fn reset(&mut self) {
        self.delta_history.clear();
        self.cumulative = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bullish_bar_adds_volume() {
        let mut cvd = CumulativeVolumeDelta::new(10);
        let r = cvd.update_bar(100.0, 102.0, 99.0, 101.0, 500.0);
        match r {
            IndicatorValue::Single(v) => assert!((v - 500.0).abs() < 1e-9),
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn bearish_bar_subtracts_volume() {
        let mut cvd = CumulativeVolumeDelta::new(10);
        let r = cvd.update_bar(101.0, 102.0, 99.0, 100.0, 500.0);
        match r {
            IndicatorValue::Single(v) => assert!((v - (-500.0)).abs() < 1e-9),
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn doji_bar_zero_delta() {
        let mut cvd = CumulativeVolumeDelta::new(10);
        let r = cvd.update_bar(100.0, 102.0, 98.0, 100.0, 500.0);
        match r {
            IndicatorValue::Single(v) => assert!((v - 0.0).abs() < 1e-9),
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn rolling_window_evicts_old_delta() {
        let mut cvd = CumulativeVolumeDelta::new(3);
        // Feed 3 bullish bars (+100 each) to fill window.
        for _ in 0..3 {
            cvd.update_bar(100.0, 101.0, 99.0, 100.5, 100.0);
        }
        // cumulative = 300
        // Feed a 4th bar (bearish, -200): window evicts first +100, adds -200 → 300 - 100 - 200 = 0... wait:
        // After eviction: cumulative was 300, remove oldest (+100) → 200, add -200 → 0.
        let r = cvd.update_bar(100.5, 101.0, 99.0, 100.0, 200.0);
        match r {
            IndicatorValue::Single(v) => assert!((v - 0.0).abs() < 1e-9, "expected 0, got {v}"),
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn not_ready_before_first_bar() {
        let cvd = CumulativeVolumeDelta::new(5);
        assert!(!cvd.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut cvd = CumulativeVolumeDelta::new(5);
        cvd.update_bar(100.0, 101.0, 99.0, 100.5, 100.0);
        cvd.reset();
        assert!(!cvd.is_ready());
        match cvd.value() {
            IndicatorValue::Single(v) => assert!((v - 0.0).abs() < 1e-9),
            other => panic!("{:?}", other),
        }
    }
}
