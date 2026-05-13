//! Session VWAP — cumulative VWAP with explicit session reset.
//!
//! Accumulates `(typical_price × volume)` and `volume` since the last
//! `reset_session()` call. The caller is responsible for resetting at
//! session boundaries (e.g. at market open each day). This is the
//! production pattern because bar streams rarely carry reliable session
//! boundary markers in their timestamps.
//!
//! Formula: `VWAP = Σ(typical_price × volume) / Σ(volume)`
//! where `typical_price = (high + low + close) / 3`.

use crate::bar_indicators::indicator_value::IndicatorValue;

/// Cumulative VWAP that resets when the caller invokes [`SessionVwap::reset_session`].
#[derive(Debug, Clone)]
pub struct SessionVwap {
    cumulative_pv: f64,
    cumulative_v: f64,
    last_value: f64,
}

impl SessionVwap {
    /// Create a new `SessionVwap`. No parameters are required — this
    /// indicator accumulates from session start to the last bar fed.
    pub fn new() -> Self {
        Self {
            cumulative_pv: 0.0,
            cumulative_v: 0.0,
            last_value: 0.0,
        }
    }

    /// Reset accumulated state for a new session.
    ///
    /// Call this at the start of each trading session before feeding the
    /// first bar of that session.
    pub fn reset_session(&mut self) {
        self.cumulative_pv = 0.0;
        self.cumulative_v = 0.0;
    }

    /// Feed one OHLCV bar and return the current session VWAP.
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, volume: f64) -> IndicatorValue {
        if volume > 0.0 {
            let typical = (high + low + close) / 3.0;
            self.cumulative_pv += typical * volume;
            self.cumulative_v += volume;
        }

        self.last_value = if self.cumulative_v > 1e-9 {
            self.cumulative_pv / self.cumulative_v
        } else {
            close
        };

        IndicatorValue::Single(self.last_value)
    }

    /// Returns the last computed VWAP without advancing state.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_value)
    }

    /// Returns `true` after the first bar with positive volume has been fed.
    pub fn is_ready(&self) -> bool {
        self.cumulative_v > 1e-9
    }

    /// Full reset (use [`reset_session`] for intraday resets).
    pub fn reset(&mut self) {
        self.cumulative_pv = 0.0;
        self.cumulative_v = 0.0;
        self.last_value = 0.0;
    }
}

impl Default for SessionVwap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_bar_vwap_equals_typical_price() {
        let mut v = SessionVwap::new();
        let r = v.update_bar(100.0, 102.0, 98.0, 101.0, 1000.0);
        // typical = (102 + 98 + 101) / 3 = 100.333...
        let expected = (102.0 + 98.0 + 101.0) / 3.0;
        match r {
            IndicatorValue::Single(val) => {
                assert!((val - expected).abs() < 1e-9, "expected {expected}, got {val}")
            }
            other => panic!("expected Single, got {:?}", other),
        }
    }

    #[test]
    fn two_equal_bars_vwap_equals_typical() {
        let mut v = SessionVwap::new();
        v.update_bar(100.0, 102.0, 98.0, 100.0, 500.0);
        let r = v.update_bar(100.0, 102.0, 98.0, 100.0, 500.0);
        let typical = (102.0 + 98.0 + 100.0) / 3.0;
        match r {
            IndicatorValue::Single(val) => {
                assert!((val - typical).abs() < 1e-9)
            }
            other => panic!("expected Single, got {:?}", other),
        }
    }

    #[test]
    fn reset_session_clears_accumulation() {
        let mut v = SessionVwap::new();
        v.update_bar(100.0, 110.0, 90.0, 100.0, 1000.0);
        v.reset_session();
        // After reset, next bar starts fresh.
        let r = v.update_bar(200.0, 202.0, 198.0, 200.0, 500.0);
        let typical = (202.0 + 198.0 + 200.0) / 3.0;
        match r {
            IndicatorValue::Single(val) => {
                assert!((val - typical).abs() < 1e-9, "expected {typical}, got {val}")
            }
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn zero_volume_bar_falls_back_to_close() {
        let mut v = SessionVwap::new();
        let r = v.update_bar(50.0, 55.0, 45.0, 50.0, 0.0);
        match r {
            IndicatorValue::Single(val) => {
                assert!((val - 50.0).abs() < 1e-9)
            }
            other => panic!("{:?}", other),
        }
    }
}
