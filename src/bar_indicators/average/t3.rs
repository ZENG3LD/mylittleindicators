//! Tillson T3 Moving Average indicator.

use super::ema::Ema;
use crate::bar_indicators::indicator_value::IndicatorValue;
use super::super::ohlcv_field::OhlcvField;

/// Tillson T3 Moving Average - ultra-smooth, low-lag moving average.
///
/// T3 = c1×EMA6 + c2×EMA5 + c3×EMA4 + c4×EMA3
///
/// where coefficients depend on the volume factor `a` (default: 0.7).
///
/// Uses 6 cascaded EMAs combined with polynomial coefficients to achieve
/// exceptional smoothness while minimizing lag.
///
/// # Parameters
/// - `period`: EMA period for each of the 6 stages
/// - `a`: Volume factor (default: 0.7, range 0-1)
///
/// # Implementation
///
/// Uses six O(1) EMA instances, so the entire T3 is O(1).
#[derive(Debug, Clone)]
pub struct T3 {
    period: usize,
    a: f64,
    e1: Ema,
    e2: Ema,
    e3: Ema,
    e4: Ema,
    e5: Ema,
    e6: Ema,
    value: f64,
}

impl T3 {
    /// Creates a new T3 with default volume factor (a=0.7).
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - EMA period for each cascade stage
    pub fn new(period: usize) -> Self {
        Self::with_alpha(period, 0.7)
    }

    /// Creates a new T3 with custom volume factor.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - EMA period for each cascade stage
    /// * `a` - Volume factor (0.0-1.0, higher = smoother)
    pub fn with_alpha(period: usize, a: f64) -> Self {
        Self::with_source(period, a, OhlcvField::Close)
    }

    /// Creates a new T3 with custom volume factor and source.
    ///
    /// # Arguments
    /// * `period` - EMA period for each cascade stage
    /// * `a` - Volume factor (0.0-1.0, higher = smoother)
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period: usize, a: f64, source: OhlcvField) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            a,
            e1: Ema::with_source(p, source),
            e2: Ema::new(p),
            e3: Ema::new(p),
            e4: Ema::new(p),
            e5: Ema::new(p),
            e6: Ema::new(p),
            value: 0.0,
        }
    }

    /// Updates the T3 with a new bar and returns the current value.
    ///
    /// Only the `close` price is used; other OHLCV fields are ignored.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let e1 = self.e1.update_bar(0.0, 0.0, 0.0, c, 0.0);
        let e2 = self.e2.update_bar(0.0, 0.0, 0.0, e1, 0.0);
        let e3 = self.e3.update_bar(0.0, 0.0, 0.0, e2, 0.0);
        let e4 = self.e4.update_bar(0.0, 0.0, 0.0, e3, 0.0);
        let e5 = self.e5.update_bar(0.0, 0.0, 0.0, e4, 0.0);
        let e6 = self.e6.update_bar(0.0, 0.0, 0.0, e5, 0.0);
        let a = self.a;
        self.value = e6 * (a * a * a)
            + e5 * (3.0 * a * a * (1.0 - a))
            + e4 * (3.0 * a * (1.0 - a) * (1.0 - a))
            + e3 * ((1.0 - a) * (1.0 - a) * (1.0 - a));
        self.value
    }

    /// Returns the current T3 value as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the T3 has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.e6.is_ready()
    }

    /// Resets the T3 to its initial state.
    pub fn reset(&mut self) {
        self.e1.reset();
        self.e2.reset();
        self.e3.reset();
        self.e4.reset();
        self.e5.reset();
        self.e6.reset();
        self.value = 0.0;
    }

    /// Returns the period of this T3.
    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_t3_basic_calculation() {
        let mut t3 = T3::new(5);

        for i in 1..=20 {
            t3.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }

        assert!(t3.is_ready());
        assert!(t3.value().main() > 0.0);
    }

    #[test]
    fn test_t3_custom_alpha() {
        let mut t3 = T3::with_alpha(5, 0.9);

        for i in 1..=20 {
            t3.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }

        assert!(t3.is_ready());
    }

    #[test]
    fn test_t3_reset() {
        let mut t3 = T3::new(3);
        for i in 1..=20 {
            t3.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }
        assert!(t3.is_ready());

        t3.reset();
        assert!(!t3.is_ready());
    }

}
