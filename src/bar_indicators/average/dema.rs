//! Double Exponential Moving Average (DEMA) indicator.

use super::ema::Ema;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Double Exponential Moving Average (DEMA) - reduces lag compared to EMA.
///
/// DEMA = 2 × EMA(price) - EMA(EMA(price))
///
/// Created by Patrick Mulloy. Provides faster response to price changes
/// while reducing noise compared to a single EMA.
///
/// # Implementation
///
/// Uses two cascaded EMA instances. O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Dema {
    source: OhlcvField,
    ema1: Ema,
    ema2: Ema,
    value: f64,
    count: usize,
    period: usize,
}

impl Dema {
    /// Returns the period of this DEMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Creates a new DEMA with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Smoothing period for both internal EMAs
    pub fn new(period: usize) -> Self {
        Self::with_source(period, OhlcvField::Close)
    }

    /// Creates a new DEMA with the specified period and source.
    ///
    /// # Arguments
    /// * `period` - Smoothing period for both internal EMAs
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period: usize, source: OhlcvField) -> Self {
        Self {
            source,
            ema1: Ema::new(period),
            ema2: Ema::new(period),
            value: 0.0,
            count: 0,
            period,
        }
    }

    /// Updates the DEMA with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close).
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let value = self.source.extract(open, high, low, close, volume);

        let ema1_val = self.ema1.update_bar(0.0, 0.0, 0.0, value, 0.0);
        let ema2_val = self.ema2.update_bar(0.0, 0.0, 0.0, ema1_val, 0.0);
        self.value = 2.0 * ema1_val - ema2_val;
        self.count += 1;
        self.value
    }

    /// Returns the current DEMA value as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the DEMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    /// Resets the DEMA to its initial state.
    pub fn reset(&mut self) {
        self.ema1.reset();
        self.ema2.reset();
        self.value = 0.0;
        self.count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dema_basic_calculation() {
        let mut dema = Dema::new(3);

        dema.update_bar(0.0, 0.0, 0.0, 10.0, 0.0);
        dema.update_bar(0.0, 0.0, 0.0, 20.0, 0.0);
        let v3 = dema.update_bar(0.0, 0.0, 0.0, 30.0, 0.0);

        assert!(dema.is_ready());
        // DEMA should be more responsive than EMA
        assert!(v3 > 20.0); // Should be closer to recent prices
    }

    #[test]
    fn test_dema_reset() {
        let mut dema = Dema::new(3);
        dema.update_bar(0.0, 0.0, 0.0, 10.0, 0.0);
        dema.update_bar(0.0, 0.0, 0.0, 20.0, 0.0);
        dema.update_bar(0.0, 0.0, 0.0, 30.0, 0.0);
        assert!(dema.is_ready());

        dema.reset();
        assert!(!dema.is_ready());
    }
}
