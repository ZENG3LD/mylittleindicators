//! Triple Exponential Moving Average (TEMA) indicator.

use crate::bar_indicators::indicator_value::IndicatorValue;
use super::super::ohlcv_field::OhlcvField;

/// Triple Exponential Moving Average (TEMA) - further reduces lag compared to DEMA.
///
/// TEMA = 3 × EMA1 - 3 × EMA2 + EMA3
///
/// where EMA1 = EMA(price), EMA2 = EMA(EMA1), EMA3 = EMA(EMA2)
///
/// Created by Patrick Mulloy. Provides even faster response than DEMA
/// while maintaining smoothness.
///
/// # Implementation
///
/// Uses three cascaded EMA instances. O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Tema {
    period: usize,
    ema1: super::ema::Ema,
    ema2: super::ema::Ema,
    ema3: super::ema::Ema,
    value: f64,
    initialized: bool,
}

impl Tema {
    /// Returns the period of this TEMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Returns `true` if the TEMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.ema1.is_ready() && self.ema2.is_ready() && self.ema3.is_ready()
    }

    /// Resets the TEMA to its initial state.
    pub fn reset(&mut self) {
        self.ema1.reset();
        self.ema2.reset();
        self.ema3.reset();
        self.value = 0.0;
        self.initialized = false;
    }

    /// Creates a new TEMA with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Smoothing period for all three internal EMAs
    pub fn new(period: usize) -> Self {
        Self::with_source(period, OhlcvField::Close)
    }

    /// Creates a new TEMA with the specified period and source.
    ///
    /// # Arguments
    /// * `period` - Smoothing period for all three internal EMAs
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period: usize, source: OhlcvField) -> Self {
        Self {
            period,
            ema1: super::ema::Ema::with_source(period, source),
            ema2: super::ema::Ema::new(period),
            ema3: super::ema::Ema::new(period),
            value: 0.0,
            initialized: false,
        }
    }

    /// Updates the TEMA with a new bar and returns the current value.
    ///
    /// Only the `close` price is used; other OHLCV fields are ignored.
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let ema1_val = self.ema1.update_bar(open, high, low, close, volume);
        let ema2_val = self.ema2.update_bar(0.0, 0.0, 0.0, ema1_val, 0.0);
        let ema3_val = self.ema3.update_bar(0.0, 0.0, 0.0, ema2_val, 0.0);
        self.value = 3.0 * ema1_val - 3.0 * ema2_val + ema3_val;
        if !self.initialized && self.ema3.is_ready() {
            self.initialized = true;
        }
        self.value
    }

    /// Returns the current TEMA value as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the TEMA has been fully initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tema_basic_calculation() {
        let mut tema = Tema::new(3);

        tema.update_bar(0.0, 0.0, 0.0, 10.0, 0.0);
        tema.update_bar(0.0, 0.0, 0.0, 20.0, 0.0);
        let v3 = tema.update_bar(0.0, 0.0, 0.0, 30.0, 0.0);

        assert!(tema.is_ready());
        // TEMA should be even more responsive than DEMA
        assert!(v3 > 20.0);
    }

    #[test]
    fn test_tema_reset() {
        let mut tema = Tema::new(3);
        tema.update_bar(0.0, 0.0, 0.0, 10.0, 0.0);
        tema.update_bar(0.0, 0.0, 0.0, 20.0, 0.0);
        tema.update_bar(0.0, 0.0, 0.0, 30.0, 0.0);
        assert!(tema.is_ready());

        tema.reset();
        assert!(!tema.is_ready());
        assert!(!tema.is_initialized());
    }
}
