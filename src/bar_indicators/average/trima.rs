//! Triangular Moving Average (TRIMA) indicator.

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Triangular Moving Average (TRIMA) - double-smoothed SMA.
///
/// TRIMA = SMA(SMA(price, period), period)
///
/// Applies two SMA passes to achieve a triangular-weighted average
/// that gives more weight to the middle of the period.
///
/// Note: This is similar to TMA but uses MovingAverageProvider.
///
/// # Implementation
///
/// Uses two cascaded SMA instances. O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Trima {
    period: usize,
    sma1: MovingAverageProvider,
    sma2: MovingAverageProvider,
    value: f64,
    source: OhlcvField,
}

impl Trima {
    /// Creates a new TRIMA with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Smoothing period for both internal SMAs
    pub fn new(period: usize) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            sma1: MovingAverageProvider::new(MovingAverageType::SMA, p),
            sma2: MovingAverageProvider::new(MovingAverageType::SMA, p),
            value: 0.0,
            source: OhlcvField::Close,
        }
    }

    /// Creates a new TRIMA with the specified period and source field.
    ///
    /// # Arguments
    /// * `period` - Smoothing period for both internal SMAs
    /// * `source` - OHLCV field to use as input (Close, HL2, HLC3, etc.)
    pub fn with_source(period: usize, source: OhlcvField) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            sma1: MovingAverageProvider::new(MovingAverageType::SMA, p),
            sma2: MovingAverageProvider::new(MovingAverageType::SMA, p),
            value: 0.0,
            source,
        }
    }

    /// Resets the TRIMA to its initial state.
    #[inline]
    pub fn reset(&mut self) {
        self.sma1.reset();
        self.sma2.reset();
        self.value = 0.0;
    }

    /// Returns `true` if the TRIMA has received enough bars to produce a valid value.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.sma1.is_ready() && self.sma2.is_ready()
    }

    /// Returns the current TRIMA value as an `IndicatorValue`.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Updates the TRIMA with a new bar and returns the current value.
    ///
    /// Uses the configured source field (default: close) to extract the input value.
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let price = self.source.extract(o, h, l, c, v);
        let a = self.sma1.update_bar(0.0, 0.0, 0.0, price, 0.0);
        self.value = self.sma2.update_bar(0.0, 0.0, 0.0, a, 0.0);
        self.value
    }

    /// Returns the period of this TRIMA.
    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trima_basic_calculation() {
        let mut trima = Trima::new(3);

        for i in 1..=6 {
            trima.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }

        assert!(trima.is_ready());
        assert!(trima.value().main() > 0.0);
    }

    #[test]
    fn test_trima_reset() {
        let mut trima = Trima::new(3);
        for i in 1..=6 {
            trima.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }
        assert!(trima.is_ready());

        trima.reset();
        assert!(!trima.is_ready());
    }

}
