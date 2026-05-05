//! Triangular Moving Average (TMA) indicator.

use crate::bar_indicators::indicator_value::IndicatorValue;
use super::super::ohlcv_field::OhlcvField;

/// Triangular Moving Average (TMA) - double-smoothed SMA.
///
/// TMA = SMA(SMA(price, period), period)
///
/// Provides extra smoothing with more weight given to middle prices.
/// Slower to react than SMA but smoother.
///
/// # Implementation
///
/// Uses two cascaded SMA instances. O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Tma {
    period: usize,
    sma1: super::sma::Sma,
    sma2: super::sma::Sma,
    value: f64,
    initialized: bool,
}

impl Tma {
    /// Returns the period of this TMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Returns `true` if the TMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.sma1.is_ready() && self.sma2.is_ready()
    }

    /// Resets the TMA to its initial state.
    pub fn reset(&mut self) {
        self.sma1.reset();
        self.sma2.reset();
        self.value = 0.0;
        self.initialized = false;
    }

    /// Creates a new TMA with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Smoothing period for both internal SMAs
    pub fn new(period: usize) -> Self {
        Self::with_source(period, OhlcvField::Close)
    }

    /// Creates a new TMA with the specified period and source.
    ///
    /// # Arguments
    /// * `period` - Smoothing period for both internal SMAs
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period: usize, source: OhlcvField) -> Self {
        Self {
            period,
            sma1: super::sma::Sma::with_source(period, source),
            sma2: super::sma::Sma::new(period),
            value: 0.0,
            initialized: false,
        }
    }

    /// Updates the TMA with a new bar and returns the current value.
    ///
    /// Only the `close` price is used; other OHLCV fields are ignored.
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let sma1_val = self.sma1.update_bar(open, high, low, close, volume);
        let sma2_val = self.sma2.update_bar(0.0, 0.0, 0.0, sma1_val, 0.0);
        self.value = sma2_val;
        if !self.initialized && self.sma2.is_ready() {
            self.initialized = true;
        }
        self.value
    }

    /// Returns the current TMA value as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the TMA has been fully initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tma_basic_calculation() {
        let mut tma = Tma::new(3);

        // Need 2*period - 1 bars for TMA to be ready
        for i in 1..=5 {
            tma.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }

        assert!(tma.is_ready());
        assert!(tma.value().main() > 0.0);
    }

    #[test]
    fn test_tma_reset() {
        let mut tma = Tma::new(3);
        for i in 1..=6 {
            tma.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }
        assert!(tma.is_ready());

        tma.reset();
        assert!(!tma.is_ready());
        assert!(!tma.is_initialized());
    }
}
