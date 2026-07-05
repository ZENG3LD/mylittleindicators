//! Cumulative Volume Delta — running sum of real aggressor-side buy/sell delta.
//!
//! CVD only has meaning when fed real taker buy/sell volume (from trade-stream
//! aggregation upstream). Bar OHLCV alone carries no aggressor-side
//! information, so [`CumulativeVolumeDelta::update_bar`] cannot compute a
//! delta — it is a no-new-information feed that returns the current
//! cumulative value unchanged. Real updates arrive exclusively through
//! [`CumulativeVolumeDelta::update_with_delta`].

use crate::bar_indicators::indicator_value::IndicatorValue;

/// Cumulative Volume Delta.
///
/// Output is `Single(cumulative_delta)` — an unbounded running total that
/// trends positive when real buy pressure dominates and negative when real
/// sell pressure dominates.
#[derive(Debug, Clone, Default)]
pub struct CumulativeVolumeDelta {
    cumulative: f64,
    ready: bool,
}

impl CumulativeVolumeDelta {
    /// Create a new `CumulativeVolumeDelta`.
    pub fn new(_window: usize) -> Self {
        Self::default()
    }

    /// Feed real aggressor-side buy/sell volume for one bar/trade window and
    /// return the updated running total.
    pub fn update_with_delta(&mut self, buy_volume: f64, sell_volume: f64) -> f64 {
        self.cumulative += buy_volume - sell_volume;
        self.ready = true;
        self.cumulative
    }

    /// No-new-information feed: bar OHLCV carries no aggressor-side data, so
    /// this returns the current cumulative value unchanged. Real delta
    /// arrives via [`Self::update_with_delta`].
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, _close: f64, _volume: f64) -> IndicatorValue {
        IndicatorValue::Single(self.cumulative)
    }

    /// Returns the last computed value without advancing state.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.cumulative)
    }

    /// Returns `true` after at least one real delta update has been fed.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Clears all accumulated state.
    pub fn reset(&mut self) {
        self.cumulative = 0.0;
        self.ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_delta_accumulates() {
        let mut cvd = CumulativeVolumeDelta::new(10);
        let r = cvd.update_with_delta(500.0, 200.0);
        assert!((r - 300.0).abs() < 1e-9);
        let r = cvd.update_with_delta(100.0, 400.0);
        assert!((r - 0.0).abs() < 1e-9);
    }

    #[test]
    fn update_bar_does_not_mutate_or_fabricate() {
        let mut cvd = CumulativeVolumeDelta::new(10);
        cvd.update_with_delta(500.0, 0.0);
        // Bar feed must not change the cumulative value regardless of candle direction.
        let r = cvd.update_bar(100.0, 102.0, 99.0, 101.0, 999.0);
        match r {
            IndicatorValue::Single(v) => assert!((v - 500.0).abs() < 1e-9),
            other => panic!("{:?}", other),
        }
        let r = cvd.update_bar(101.0, 102.0, 99.0, 100.0, 999.0);
        match r {
            IndicatorValue::Single(v) => assert!((v - 500.0).abs() < 1e-9),
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn not_ready_before_first_real_delta() {
        let mut cvd = CumulativeVolumeDelta::new(5);
        assert!(!cvd.is_ready());
        cvd.update_bar(100.0, 101.0, 99.0, 100.5, 100.0);
        assert!(!cvd.is_ready(), "bar feed alone must not mark CVD ready");
        cvd.update_with_delta(10.0, 5.0);
        assert!(cvd.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut cvd = CumulativeVolumeDelta::new(5);
        cvd.update_with_delta(100.0, 0.0);
        cvd.reset();
        assert!(!cvd.is_ready());
        match cvd.value() {
            IndicatorValue::Single(v) => assert!((v - 0.0).abs() < 1e-9),
            other => panic!("{:?}", other),
        }
    }
}
