//! Fractal Adaptive Moving Average (FRAMA) indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;
use super::super::ohlcv_field::OhlcvField;

/// Fractal Adaptive Moving Average (FRAMA) - adapts to market fractal dimension.
///
/// α = exp(-4.6 × (D - 1))
/// FRAMA = α × Price + (1-α) × FRAMA_prev
///
/// where D is the fractal dimension estimated from price movements.
///
/// Created by John Ehlers. Uses fractal geometry to determine how trending
/// or choppy the market is, adjusting smoothing accordingly. Trending markets
/// get faster response, choppy markets get more smoothing.
///
/// # Implementation
///
/// Uses sliding window for fractal dimension calculation. O(period) per update.
/// Maximum period is 512 bars.
#[derive(Debug, Clone)]
pub struct Frama {
    period: usize,
    source: OhlcvField,
    window: ArrayVec<f64, 512>,
    alpha: f64,
    value: f64,
    initialized: bool,
}

impl Frama {
    /// Returns the period of this FRAMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Returns `true` if the FRAMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.window.len() >= self.period
    }

    /// Resets the FRAMA to its initial state.
    pub fn reset(&mut self) {
        self.window.clear();
        self.alpha = 0.0;
        self.value = 0.0;
        self.initialized = false;
    }

    /// Creates a new FRAMA with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Number of bars for fractal dimension calculation (1..=512)
    pub fn new(period: usize) -> Self {
        Self::with_source(period, OhlcvField::Close)
    }

    /// Creates a new FRAMA with the specified period and source.
    ///
    /// # Arguments
    /// * `period` - Number of bars for fractal dimension calculation (1..=512)
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period: usize, source: OhlcvField) -> Self {
        Self {
            period,
            source,
            window: ArrayVec::new(),
            alpha: 0.0,
            value: 0.0,
            initialized: false,
        }
    }

    /// Updates the FRAMA with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close).
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let price = self.source.extract(open, high, low, close, volume);
        if self.window.len() == self.period {
            self.window.remove(0);
        }
        self.window.push(price);
        if self.window.len() < self.period {
            self.value = price;
            return self.value;
        }
        let n = self.period as f64;
        let min1 = self.window.iter().cloned().fold(f64::INFINITY, f64::min);
        let max1 = self.window.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let n1 = (max1 - min1) / n;
        let n2 = self.window.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / (n - 1.0);
        let dim = if n2 > 0.0 && n1 > 0.0 {
            (n2 / n1).ln() / (n - 1.0).ln()
        } else {
            1.0
        };
        self.alpha = (-4.6 * (dim - 1.0)).exp();
        self.value = self.alpha * price + (1.0 - self.alpha) * self.value;
        if !self.initialized {
            self.initialized = true;
        }
        self.value
    }

    /// Returns the current FRAMA value as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the FRAMA has been fully initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frama_basic_calculation() {
        let mut frama = Frama::new(10);

        for i in 1..=20 {
            frama.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }

        assert!(frama.is_ready());
        assert!(frama.value().main() > 0.0);
    }

    #[test]
    fn test_frama_reset() {
        let mut frama = Frama::new(5);
        for i in 1..=10 {
            frama.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }
        assert!(frama.is_ready());

        frama.reset();
        assert!(!frama.is_ready());
        assert!(!frama.is_initialized());
    }

}
