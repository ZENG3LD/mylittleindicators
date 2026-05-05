//! On-Balance Volume (OBV) indicator.

use crate::bar_indicators::indicator_value::IndicatorValue;

/// On-Balance Volume (OBV) - cumulative volume-based momentum indicator.
///
/// OBV = Previous OBV + Volume (if close > previous close)
/// OBV = Previous OBV - Volume (if close < previous close)
/// OBV = Previous OBV (if close = previous close)
///
/// Developed by Joseph Granville. OBV shows the cumulative buying and selling
/// pressure by adding volume on up days and subtracting on down days.
///
/// Interpretation:
/// - Rising OBV: Accumulation (buying pressure)
/// - Falling OBV: Distribution (selling pressure)
/// - OBV divergence with price: Potential trend reversal
/// - OBV trend confirmation: Validates price trends
///
/// # Implementation
///
/// Cumulative calculation. O(1) per update.
#[derive(Clone)]
pub struct Obv {
    value: f64,
    prev_close: f64,
    ready: bool,
    count: usize,
}

impl Obv {
    /// Creates a new OBV indicator.
    pub fn new() -> Self {
        Self {
            value: 0.0,
            prev_close: 0.0,
            ready: false,
            count: 0,
        }
    }

    /// Updates the OBV with a new bar and returns the current value.
    ///
    /// Uses `close` and `volume` prices.
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, volume: f64) -> f64 {
        if self.count == 0 {
            self.prev_close = close;
            self.count += 1;
            return self.value;
        }
        if close > self.prev_close {
            self.value += volume;
        } else if close < self.prev_close {
            self.value -= volume;
        }
        self.prev_close = close;
        self.count += 1;
        self.ready = self.count > 1;
        self.value
    }

    /// Returns the current OBV value as an `IndicatorValue`.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the OBV has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Resets the OBV to its initial state.
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.prev_close = 0.0;
        self.ready = false;
        self.count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_obv_basic_calculation() {
        let mut obv = Obv::new();

        // First bar
        obv.update_bar(0.0, 0.0, 0.0, 100.0, 1000.0);

        // Price up - should add volume
        obv.update_bar(0.0, 0.0, 0.0, 105.0, 500.0);
        assert!(obv.is_ready());
        assert_eq!(obv.value().main(), 500.0);

        // Price up again - should add more volume
        obv.update_bar(0.0, 0.0, 0.0, 110.0, 300.0);
        assert_eq!(obv.value().main(), 800.0);

        // Price down - should subtract volume
        obv.update_bar(0.0, 0.0, 0.0, 105.0, 200.0);
        assert_eq!(obv.value().main(), 600.0);
    }

    #[test]
    fn test_obv_unchanged_price() {
        let mut obv = Obv::new();

        obv.update_bar(0.0, 0.0, 0.0, 100.0, 1000.0);
        obv.update_bar(0.0, 0.0, 0.0, 105.0, 500.0);
        let prev_obv = obv.value().main();

        // Price unchanged - OBV should stay same
        obv.update_bar(0.0, 0.0, 0.0, 105.0, 300.0);
        assert_eq!(obv.value().main(), prev_obv);
    }

    #[test]
    fn test_obv_downtrend() {
        let mut obv = Obv::new();

        obv.update_bar(0.0, 0.0, 0.0, 100.0, 1000.0);

        // Consecutive down days
        for i in 1..=5 {
            obv.update_bar(0.0, 0.0, 0.0, 100.0 - i as f64, 100.0);
        }

        assert!(obv.is_ready());
        // OBV should be negative after downtrend
        assert!(obv.value().main() < 0.0, "OBV should be negative in downtrend");
    }

    #[test]
    fn test_obv_reset() {
        let mut obv = Obv::new();

        obv.update_bar(0.0, 0.0, 0.0, 100.0, 1000.0);
        obv.update_bar(0.0, 0.0, 0.0, 105.0, 500.0);
        assert!(obv.is_ready());

        obv.reset();
        assert!(!obv.is_ready());
        assert_eq!(obv.value().main(), 0.0);
    }
}


















