//! McGinley Dynamic indicator.

use crate::bar_indicators::indicator_value::IndicatorValue;
use super::super::ohlcv_field::OhlcvField;

/// McGinley Dynamic - self-adjusting moving average.
///
/// MD = MD_prev + (Price - MD_prev) / (k × (Price/MD_prev)^4)
///
/// where k ≈ period.
///
/// Created by John McGinley. Automatically adjusts its speed based on
/// market conditions. When price moves quickly away from the average,
/// it speeds up to catch up. When price is near the average, it slows down.
///
/// # Implementation
///
/// O(1) update complexity using direct formula calculation.
#[derive(Debug, Clone)]
pub struct McGinleyDynamic {
    period: usize,
    source: OhlcvField,
    value: f64,
    initialized: bool,
}

impl McGinleyDynamic {
    /// Creates a new McGinley Dynamic with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Base smoothing factor (similar to EMA period)
    pub fn new(period: usize) -> Self {
        Self::with_source(period, OhlcvField::Close)
    }

    /// Creates a new McGinley Dynamic with the specified period and source.
    ///
    /// # Arguments
    /// * `period` - Base smoothing factor (similar to EMA period)
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period: usize, source: OhlcvField) -> Self {
        Self {
            period: period.max(1),
            source,
            value: 0.0,
            initialized: false,
        }
    }

    /// Updates the McGinley Dynamic with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close).
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let value = self.source.extract(open, high, low, close, volume);
        if !self.initialized {
            self.value = value;
            self.initialized = true;
            return self.value;
        }
        let md_prev = self.value;
        if md_prev == 0.0 {
            self.value = value;
            return self.value;
        }
        let ratio = (value / md_prev).abs();
        let denom = (self.period as f64) * ratio.powi(4);
        self.value = md_prev + (value - md_prev) / denom.max(1e-9);
        self.value
    }

    /// Returns the current McGinley Dynamic value as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the indicator has received at least one bar.
    pub fn is_ready(&self) -> bool {
        self.initialized
    }

    /// Resets the indicator to its initial state.
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.initialized = false;
    }

    /// Returns the period of this McGinley Dynamic.
    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcginley_basic_calculation() {
        let mut md = McGinleyDynamic::new(10);

        for i in 1..=20 {
            md.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(md.is_ready());
        assert!(md.value().main() > 100.0);
    }

    #[test]
    fn test_mcginley_first_value() {
        let mut md = McGinleyDynamic::new(10);
        let v = md.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);

        assert!(md.is_ready());
        assert_eq!(v, 100.0);
    }

    #[test]
    fn test_mcginley_reset() {
        let mut md = McGinleyDynamic::new(10);
        md.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        md.update_bar(0.0, 0.0, 0.0, 110.0, 0.0);
        assert!(md.is_ready());

        md.reset();
        assert!(!md.is_ready());
    }

}
