//! Arnaud Legoux Moving Average (ALMA) indicator.

use std::collections::VecDeque;
use crate::bar_indicators::indicator_value::IndicatorValue;
use super::super::ohlcv_field::OhlcvField;

/// Arnaud Legoux Moving Average (ALMA) - Gaussian-weighted moving average.
///
/// ALMA = Σ(Price × Weight) / Σ(Weight)
///
/// where weights follow a Gaussian distribution centered at `offset × (period-1)`.
///
/// Designed to reduce lag while maintaining smoothness. The offset parameter
/// controls where the center of the Gaussian is placed (0=oldest, 1=newest).
/// Sigma controls the width of the Gaussian curve.
///
/// # Parameters
/// - `period`: Number of bars
/// - `offset`: Center position, 0-1 (default: 0.85)
/// - `sigma`: Gaussian width (default: 6.0)
///
/// # Implementation
///
/// Precomputes Gaussian weights at construction. O(period) per update.
#[derive(Debug, Clone)]
pub struct Alma {
    period: usize,
    source: OhlcvField,
    #[allow(dead_code)]
    offset: f64,
    #[allow(dead_code)]
    sigma: f64,
    buffer: VecDeque<f64>,
    weights: Vec<f64>,
    weight_sum: f64,
    value: f64,
}

impl Alma {
    /// Creates a new ALMA with default parameters (offset=0.85, sigma=6.0).
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Number of bars to include
    pub fn new(period: usize) -> Self {
        Self::with_params(period, 0.85, 6.0)
    }

    /// Creates a new ALMA with custom offset and sigma.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Number of bars to include
    /// * `offset` - Gaussian center position (0.0-1.0, higher = more recent)
    /// * `sigma` - Gaussian width (higher = smoother)
    pub fn with_params(period: usize, offset: f64, sigma: f64) -> Self {
        Self::with_source(period, OhlcvField::Close, offset, sigma)
    }

    /// Creates a new ALMA with custom source, offset, and sigma.
    ///
    /// # Arguments
    /// * `period` - Number of bars to include
    /// * `source` - OHLCV field to use as input
    /// * `offset` - Gaussian center position (0.0-1.0, higher = more recent)
    /// * `sigma` - Gaussian width (higher = smoother)
    pub fn with_source(period: usize, source: OhlcvField, offset: f64, sigma: f64) -> Self {
        let period = period.max(1);
        let (weights, weight_sum) = Self::build_weights(period, offset, sigma);
        Self {
            period,
            source,
            offset,
            sigma,
            buffer: VecDeque::with_capacity(period),
            weights,
            weight_sum,
            value: 0.0,
        }
    }

    #[inline]
    fn build_weights(period: usize, offset: f64, sigma: f64) -> (Vec<f64>, f64) {
        let m = offset * (period as f64 - 1.0);
        let s = (period as f64 / sigma).max(1e-9);
        let mut w = Vec::with_capacity(period);
        let mut sum = 0.0;
        for i in 0..period {
            let x = (i as f64 - m) / s;
            let wi = (-0.5 * x * x).exp();
            w.push(wi);
            sum += wi;
        }
        (w, sum)
    }

    /// Updates the ALMA with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close).
    pub fn update_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> f64 {
        let value = self.source.extract(open, high, low, close, volume);
        self.buffer.push_back(value);
        if self.buffer.len() > self.period {
            self.buffer.pop_front();
        }
        if self.buffer.len() == self.period {
            let mut acc = 0.0;
            for (i, &p) in self.buffer.iter().enumerate() {
                acc += p * self.weights[i];
            }
            self.value = acc / self.weight_sum;
        }
        self.value
    }

    /// Returns the current ALMA value as an `IndicatorValue`.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the ALMA has received enough bars to produce a valid value.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.buffer.len() == self.period
    }

    /// Resets the ALMA to its initial state.
    #[inline]
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.value = 0.0;
    }

    /// Returns the period of this ALMA.
    #[inline]
    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alma_basic_calculation() {
        let mut alma = Alma::new(5);

        for i in 1..=5 {
            alma.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }

        assert!(alma.is_ready());
        assert!(alma.value().main() > 0.0);
    }

    #[test]
    fn test_alma_custom_params() {
        let mut alma = Alma::with_params(5, 0.5, 3.0);

        for i in 1..=5 {
            alma.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }

        assert!(alma.is_ready());
    }

    #[test]
    fn test_alma_reset() {
        let mut alma = Alma::new(3);
        alma.update_bar(0.0, 0.0, 0.0, 10.0, 0.0);
        alma.update_bar(0.0, 0.0, 0.0, 20.0, 0.0);
        alma.update_bar(0.0, 0.0, 0.0, 30.0, 0.0);
        assert!(alma.is_ready());

        alma.reset();
        assert!(!alma.is_ready());
    }
}
