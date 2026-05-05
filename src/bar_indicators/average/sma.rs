//! Simple Moving Average (SMA) indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Simple Moving Average (SMA) - arithmetic mean of the last N values from a configurable source.
///
/// SMA = (P1 + P2 + ... + Pn) / n
///
/// # Implementation
///
/// Uses a ring buffer with O(1) update complexity. Maximum period is 512.
#[derive(Debug, Clone)]
pub struct Sma {
    period: usize,
    source: OhlcvField,
    sum: f64,
    count: usize,
    value: f64,
    buf: ArrayVec<f64, 512>,
    idx: usize,
}

impl Sma {
    /// Returns the period of this SMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Creates a new SMA with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Number of bars to average (1..=512)
    pub fn new(period: usize) -> Self {
        Self::with_source(period, OhlcvField::Close)
    }

    /// Creates a new SMA with the specified period and source.
    ///
    /// # Arguments
    /// * `period` - Number of bars to average (1..=512)
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period: usize, source: OhlcvField) -> Self {
        Self {
            period,
            source,
            sum: 0.0,
            count: 0,
            value: 0.0,
            buf: ArrayVec::new(),
            idx: 0,
        }
    }
    /// Updates the SMA with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close).
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let value = self.source.extract(open, high, low, close, volume);

        if self.count < self.period {
            self.buf.push(value);
            self.sum += value;
            self.count += 1;
            self.idx = self.count % self.period;
        } else {
            let old = self.buf[self.idx];
            self.sum += value - old;
            self.buf[self.idx] = value;
            self.idx = (self.idx + 1) % self.period;
        }
        self.value = self.sum / self.count as f64;
        self.value
    }
    /// Returns the current SMA value as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns the current SMA value as `f64`.
    pub fn value_f64(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the SMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    /// Resets the SMA to its initial state.
    pub fn reset(&mut self) {
        self.sum = 0.0;
        self.count = 0;
        self.value = 0.0;
        self.buf.fill(0.0);
        self.idx = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_sma_basic_calculation() {
        let mut sma = Sma::new(3);

        // First bar
        let v1 = sma.update_bar(0.0, 0.0, 0.0, 10.0, 0.0);
        assert!(!sma.is_ready());
        assert!((v1 - 10.0).abs() < 1e-10);

        // Second bar
        let v2 = sma.update_bar(0.0, 0.0, 0.0, 20.0, 0.0);
        assert!(!sma.is_ready());
        assert!((v2 - 15.0).abs() < 1e-10); // (10+20)/2

        // Third bar - now ready
        let v3 = sma.update_bar(0.0, 0.0, 0.0, 30.0, 0.0);
        assert!(sma.is_ready());
        assert!((v3 - 20.0).abs() < 1e-10); // (10+20+30)/3

        // Fourth bar - slides window
        let v4 = sma.update_bar(0.0, 0.0, 0.0, 40.0, 0.0);
        assert!((v4 - 30.0).abs() < 1e-10); // (20+30+40)/3
    }

    #[test]
    fn test_sma_period_1() {
        let mut sma = Sma::new(1);

        let v = sma.update_bar(0.0, 0.0, 0.0, 42.0, 0.0);
        assert!(sma.is_ready());
        assert!((v - 42.0).abs() < 1e-10);

        let v2 = sma.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        assert!((v2 - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_sma_reset() {
        let mut sma = Sma::new(3);

        sma.update_bar(0.0, 0.0, 0.0, 10.0, 0.0);
        sma.update_bar(0.0, 0.0, 0.0, 20.0, 0.0);
        sma.update_bar(0.0, 0.0, 0.0, 30.0, 0.0);
        assert!(sma.is_ready());

        sma.reset();
        assert!(!sma.is_ready());
        assert!((sma.value_f64()).abs() < 1e-10);
    }

    #[test]
    fn test_sma_value_types() {
        let mut sma = Sma::new(2);
        sma.update_bar(0.0, 0.0, 0.0, 10.0, 0.0);
        sma.update_bar(0.0, 0.0, 0.0, 20.0, 0.0);

        // Check both value methods return same result
        let indicator_val = sma.value();
        let f64_val = sma.value_f64();

        if let IndicatorValue::Single(v) = indicator_val {
            assert!((v - f64_val).abs() < 1e-10);
        } else {
            panic!("Expected Single variant");
        }
    }

    #[test]
    fn test_sma_period_getter() {
        let sma = Sma::new(14);
        assert_eq!(sma.period(), 14);
    }

    #[test]
    fn test_sma_with_high_source() {
        let mut sma = Sma::with_source(3, OhlcvField::High);

        // Bar 1: High = 110.0
        let v1 = sma.update_bar(100.0, 110.0, 90.0, 105.0, 1000.0);
        assert!((v1 - 110.0).abs() < 1e-10);

        // Bar 2: High = 120.0
        let v2 = sma.update_bar(100.0, 120.0, 90.0, 105.0, 1000.0);
        assert!((v2 - 115.0).abs() < 1e-10); // (110+120)/2

        // Bar 3: High = 130.0
        let v3 = sma.update_bar(100.0, 130.0, 90.0, 105.0, 1000.0);
        assert!((v3 - 120.0).abs() < 1e-10); // (110+120+130)/3
    }

    #[test]
    fn test_sma_with_hl2_source() {
        let mut sma = Sma::with_source(2, OhlcvField::HL2);

        // Bar 1: HL2 = (110+90)/2 = 100.0
        let v1 = sma.update_bar(100.0, 110.0, 90.0, 105.0, 1000.0);
        assert!((v1 - 100.0).abs() < 1e-10);

        // Bar 2: HL2 = (120+80)/2 = 100.0
        let v2 = sma.update_bar(100.0, 120.0, 80.0, 105.0, 1000.0);
        assert!((v2 - 100.0).abs() < 1e-10); // (100+100)/2
    }

    #[test]
    fn test_sma_default_source_is_close() {
        let mut sma_default = Sma::new(2);
        let mut sma_close = Sma::with_source(2, OhlcvField::Close);

        // Both should produce the same result
        let v1 = sma_default.update_bar(100.0, 110.0, 90.0, 105.0, 1000.0);
        let v2 = sma_close.update_bar(100.0, 110.0, 90.0, 105.0, 1000.0);
        assert!((v1 - v2).abs() < 1e-10);

        let v1 = sma_default.update_bar(100.0, 120.0, 80.0, 115.0, 2000.0);
        let v2 = sma_close.update_bar(100.0, 120.0, 80.0, 115.0, 2000.0);
        assert!((v1 - v2).abs() < 1e-10);
    }
}






















