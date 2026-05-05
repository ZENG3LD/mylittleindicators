//! Rate of Change (ROC) indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Rate of Change (ROC) - measures percentage change over a specified period.
///
/// ROC = (Value - Value_n) / Value_n  (as decimal, multiply by 100 for percentage)
///
/// or with logarithmic mode:
///
/// ROC = log10(Value / Value_n)
///
/// ROC is an unbounded momentum oscillator:
/// - Positive values: Price is higher than n periods ago
/// - Negative values: Price is lower than n periods ago
/// - Zero crossings: Potential trend changes
///
/// # Parameters
/// - `period`: Lookback period
/// - `use_log`: Use logarithmic calculation
/// - `source`: OHLCV field to use (default: Close)
///
/// # Implementation
///
/// Uses ring buffer. O(1) update complexity. Maximum period is 512.
#[derive(Clone)]
pub struct Roc {
    period: usize,
    buffer: ArrayVec<f64, 512>,
    index: usize,
    filled: bool,
    value: f64,
    use_log: bool,
    source: OhlcvField,
}

impl Roc {
    /// Creates a new ROC with the specified parameters.
    ///
    /// # Arguments
    /// * `period` - Lookback period (1..=512)
    /// * `use_log` - Use logarithmic calculation instead of percentage
    ///
    /// # Note
    /// This constructor uses Close as the default source. Use `with_source()` to specify a different source.
    pub fn new(period: usize, use_log: bool) -> Self {
        Self::with_source(period, use_log, OhlcvField::Close)
    }

    /// Creates a new ROC with a configurable source field.
    ///
    /// # Arguments
    /// * `period` - Lookback period (1..=512)
    /// * `use_log` - Use logarithmic calculation instead of percentage
    /// * `source` - OHLCV field to use for calculations
    pub fn with_source(period: usize, use_log: bool, source: OhlcvField) -> Self {
        assert!(period <= 512, "Roc period must be <= 512");
        Self {
            period,
            buffer: ArrayVec::from([0.0; 512]),
            index: 0,
            filled: false,
            value: 0.0,
            use_log,
            source,
        }
    }

    /// Updates the ROC with a new bar and returns the current value.
    ///
    /// The value is extracted based on the configured source field.
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let value = self.source.extract(open, high, low, close, volume);
        self.buffer[self.index] = value;
        let prev_idx = (self.index + 1) % self.period;
        let ready = self.filled || self.index + 1 >= self.period;
        let prev = if ready { self.buffer[prev_idx] } else { value };

        let roc = if ready {
            if self.use_log {
                (value / prev).log10()
            } else {
                (value - prev) / prev
            }
        } else {
            0.0
        };

        self.value = roc;
        self.index = (self.index + 1) % self.period;
        if self.index == 0 {
            self.filled = true;
        }
        self.value
    }

    /// Returns the current ROC value as an `IndicatorValue`.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the ROC has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Resets the ROC to its initial state.
    pub fn reset(&mut self) {
        self.buffer = ArrayVec::from([0.0; 512]);
        self.index = 0;
        self.filled = false;
        self.value = 0.0;
    }

    /// Returns the period of this ROC.
    #[inline]
    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_roc_basic_calculation() {
        let mut roc = Roc::new(10, false);

        // Feed constant growth data
        for i in 1..=20 {
            roc.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(roc.is_ready());
        // With constant growth, ROC should be positive
        assert!(roc.value().main() > 0.0, "ROC with growth should be positive");
    }

    #[test]
    fn test_roc_decline() {
        let mut roc = Roc::new(10, false);

        // Feed declining data
        for i in 1..=20 {
            roc.update_bar(0.0, 0.0, 0.0, 200.0 - i as f64, 0.0);
        }

        assert!(roc.is_ready());
        // With declining prices, ROC should be negative
        assert!(roc.value().main() < 0.0, "ROC with decline should be negative");
    }

    #[test]
    fn test_roc_percentage_calculation() {
        let mut roc = Roc::new(5, false);

        // Fill buffer with 100, then jump to 110
        // ROC compares current with value 5 bars ago
        roc.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        roc.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        roc.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        roc.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        roc.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        let val = roc.update_bar(0.0, 0.0, 0.0, 110.0, 0.0);

        // (110 - 100) / 100 = 0.10
        assert!((val - 0.10).abs() < 0.01, "Expected ~0.10, got {}", val);
    }

    #[test]
    fn test_roc_log_mode() {
        let mut roc = Roc::new(5, true);

        for i in 1..=10 {
            roc.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64 * 2.0, 0.0);
        }

        assert!(roc.is_ready());
        // Log ROC should be positive for growth
        assert!(roc.value().main() > 0.0);
    }

    #[test]
    fn test_roc_reset() {
        let mut roc = Roc::new(10, false);

        for i in 1..=20 {
            roc.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(roc.is_ready());

        roc.reset();
        assert!(!roc.is_ready());
        assert!((roc.value().main()).abs() < 1e-10);
    }

    #[test]
    fn test_roc_period_getter() {
        let roc = Roc::new(14, false);
        assert_eq!(roc.period(), 14);
    }

    #[test]
    fn test_roc_constant_price() {
        let mut roc = Roc::new(10, false);

        // Constant price = 0% change
        for _ in 1..=20 {
            roc.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        }

        assert!(roc.is_ready());
        assert!((roc.value().main()).abs() < 1e-10, "ROC with constant price should be 0");
    }
}
