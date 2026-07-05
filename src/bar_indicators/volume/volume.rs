//! Raw per-bar volume passthrough.
//!
//! First-class catalog entry for plain traded volume, rendered as a
//! TradingView-style bottom-glued histogram in the main pane.

use crate::bar_indicators::indicator_value::IndicatorValue;

/// Trivial passthrough of per-bar volume.
#[derive(Debug, Clone, Copy, Default)]
pub struct Volume {
    current: f64,
    ready: bool,
}

impl Volume {
    /// Create a new `Volume` indicator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Feed one OHLCV bar and return `Single(volume)`.
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, _close: f64, volume: f64) -> IndicatorValue {
        self.current = volume;
        self.ready = true;
        IndicatorValue::Single(self.current)
    }

    /// Returns the last computed value without advancing state.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current)
    }

    /// Returns `true` after at least one bar has been fed.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Clears all accumulated state.
    pub fn reset(&mut self) {
        self.current = 0.0;
        self.ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_returns_volume() {
        let mut v = Volume::new();
        let r = v.update_bar(100.0, 101.0, 99.0, 100.5, 12345.0);
        match r {
            IndicatorValue::Single(x) => assert!((x - 12345.0).abs() < 1e-9),
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn not_ready_before_first_bar() {
        let v = Volume::new();
        assert!(!v.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut v = Volume::new();
        v.update_bar(100.0, 101.0, 99.0, 100.5, 500.0);
        v.reset();
        assert!(!v.is_ready());
        match v.value() {
            IndicatorValue::Single(x) => assert!((x - 0.0).abs() < 1e-9),
            other => panic!("{:?}", other),
        }
    }
}
