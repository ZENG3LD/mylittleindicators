//! Hull Moving Average (HMA) indicator.

use crate::bar_indicators::average::wma::Wma;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Hull Moving Average (HMA) - designed to reduce lag while maintaining smoothness.
///
/// HMA = WMA(2×WMA(period/2) - WMA(period), sqrt(period))
///
/// Created by Alan Hull. Combines two WMAs to reduce lag, then smooths
/// with a shorter WMA.
///
/// # Implementation
///
/// Uses three O(1) WMA instances, so the entire HMA is O(1).
#[derive(Debug, Clone)]
pub struct Hma {
    period: usize,
    source: OhlcvField,
    wma1: Wma,       // period/2
    wma2: Wma,       // period
    wma_final: Wma,  // sqrt(period)
    count: usize,
    value: f64,
}

impl Hma {
    /// Returns the period of this HMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Returns the number of bars processed.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Creates a new HMA with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Main period (internally uses period/2 and sqrt(period))
    pub fn new(period: usize) -> Self {
        Self::with_source(period, OhlcvField::Close)
    }

    /// Creates a new HMA with the specified period and source.
    ///
    /// # Arguments
    /// * `period` - Main period (internally uses period/2 and sqrt(period))
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period: usize, source: OhlcvField) -> Self {
        let p = period.max(1);
        let period2 = (p / 2).max(1);
        let sqrt_period = (p as f64).sqrt().floor() as usize;
        let sqrt_period = sqrt_period.max(1);

        Self {
            period: p,
            source,
            wma1: Wma::new(period2),
            wma2: Wma::new(p),
            wma_final: Wma::new(sqrt_period),
            count: 0,
            value: 0.0,
        }
    }

    /// Updates the HMA with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close).
    /// O(1) operation.
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let value = self.source.extract(open, high, low, close, volume);

        let w1 = self.wma1.update_bar(0.0, 0.0, 0.0, value, 0.0);
        let w2 = self.wma2.update_bar(0.0, 0.0, 0.0, value, 0.0);
        let diff = 2.0 * w1 - w2;
        let hma = self.wma_final.update_bar(0.0, 0.0, 0.0, diff, 0.0);

        self.count += 1;
        self.value = hma;
        self.value
    }

    /// Returns the current HMA value as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the HMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    /// Resets the HMA to its initial state.
    pub fn reset(&mut self) {
        self.wma1.reset();
        self.wma2.reset();
        self.wma_final.reset();
        self.count = 0;
        self.value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hma_small_period() {
        let mut hma = Hma::new(5);

        // Feed some values
        for i in 1..=10 {
            let value = (i * 10) as f64;
            hma.update_bar(0.0, 0.0, 0.0, value, 0.0);
        }

        assert!(hma.is_ready());
        assert!(hma.value().main() > 0.0);
    }

    #[test]
    fn test_hma_period_1() {
        let mut hma = Hma::new(1);

        assert_eq!(hma.update_bar(0.0, 0.0, 0.0, 10.0, 0.0), 10.0);
        assert_eq!(hma.update_bar(0.0, 0.0, 0.0, 20.0, 0.0), 20.0);
    }

    #[test]
    fn test_hma_reset() {
        let mut hma = Hma::new(5);
        for i in 1..=10 {
            hma.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }
        assert!(hma.is_ready());

        hma.reset();
        assert!(!hma.is_ready());
        assert_eq!(hma.count(), 0);
    }
}
