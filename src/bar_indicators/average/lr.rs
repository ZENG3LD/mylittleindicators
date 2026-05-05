//! Linear Regression Moving Average (LR) indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;
use super::super::ohlcv_field::OhlcvField;

/// Linear Regression Moving Average (LR) - least-squares regression line endpoint.
///
/// Fits a least-squares regression line to the last N prices and returns
/// the endpoint value. Also provides slope, intercept, and R² statistics.
///
/// LR provides excellent smoothing with minimal lag since it projects
/// the trend forward.
///
/// # Implementation
///
/// Uses closed-form least squares calculation. O(period) per update.
/// Maximum period is 512 bars.
#[derive(Debug, Clone)]
pub struct LinearRegressionMA {
    period: usize,
    source: OhlcvField,
    slope: f64,
    intercept: f64,
    r2: f64,
    value: f64,
    buf: ArrayVec<f64, 512>,
    initialized: bool,
}

/// Type alias for Linear Regression MA.
pub type Lr = LinearRegressionMA;

impl LinearRegressionMA {
    /// Returns the period of this LR.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Creates a new Linear Regression MA with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Number of bars for regression (1..=512)
    pub fn new(period: usize) -> Self {
        Self::with_source(period, OhlcvField::Close)
    }

    /// Creates a new Linear Regression MA with the specified period and source.
    ///
    /// # Arguments
    /// * `period` - Number of bars for regression (1..=512)
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period: usize, source: OhlcvField) -> Self {
        Self {
            period,
            source,
            slope: 0.0,
            intercept: 0.0,
            r2: 0.0,
            value: 0.0,
            buf: ArrayVec::new(),
            initialized: false,
        }
    }

    /// Updates the LR with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close).
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let value = self.source.extract(open, high, low, close, volume);
        if self.buf.len() == self.period {
            self.buf.remove(0);
        }
        self.buf.push(value);
        if self.buf.len() < self.period {
            self.value = 0.0;
            self.initialized = false;
            return self.value;
        }
        self.initialized = true;
        let x_arr: Vec<f64> = (1..=self.period).map(|x| x as f64).collect();
        let y_arr: Vec<f64> = self.buf.iter().cloned().collect();
        let x_sum: f64 = 0.5 * self.period as f64 * (self.period as f64 + 1.0);
        let x_mul_sum: f64 = x_sum * 2.0f64.mul_add(self.period as f64, 1.0) / 3.0;
        let divisor: f64 = (self.period as f64).mul_add(x_mul_sum, -(x_sum * x_sum));
        let y_sum: f64 = y_arr.iter().sum::<f64>();
        let sum_x_y: f64 = x_arr
            .iter()
            .zip(y_arr.iter())
            .map(|(x, y)| x * y)
            .sum::<f64>();
        self.slope = (self.period as f64).mul_add(sum_x_y, -(x_sum * y_sum)) / divisor;
        self.intercept = y_sum.mul_add(x_mul_sum, -(x_sum * sum_x_y)) / divisor;
        let residuals: Vec<f64> = x_arr
            .iter()
            .zip(y_arr.iter())
            .map(|(x, y)| self.slope.mul_add(*x, self.intercept) - y)
            .collect();
        self.value = residuals.last().unwrap_or(&0.0) + y_arr.last().unwrap_or(&0.0);
        let mean: f64 = y_arr.iter().sum::<f64>() / y_arr.len() as f64;
        self.r2 = 1.0
            - residuals.iter().map(|r| r * r).sum::<f64>()
                / y_arr.iter().map(|y| (y - mean) * (y - mean)).sum::<f64>();
        self.value
    }

    /// Returns the current LR value as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns the slope of the regression line.
    pub fn slope(&self) -> f64 {
        self.slope
    }

    /// Returns the intercept of the regression line.
    pub fn intercept(&self) -> f64 {
        self.intercept
    }

    /// Returns the R² (coefficient of determination) of the regression.
    pub fn r2(&self) -> f64 {
        self.r2
    }

    /// Returns `true` if the LR has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.initialized
    }

    /// Resets the LR to its initial state.
    pub fn reset(&mut self) {
        self.buf.clear();
        self.value = 0.0;
        self.slope = 0.0;
        self.intercept = 0.0;
        self.r2 = 0.0;
        self.initialized = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lr_basic_calculation() {
        let mut lr = LinearRegressionMA::new(5);

        for i in 1..=5 {
            lr.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }

        assert!(lr.is_ready());
        assert!(lr.value().main() > 0.0);
        // Perfect linear data should have high R²
        assert!(lr.r2() > 0.99);
    }

    #[test]
    fn test_lr_slope_positive() {
        let mut lr = LinearRegressionMA::new(5);

        // Upward trend
        for i in 1..=5 {
            lr.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }

        assert!(lr.slope() > 0.0);
    }

    #[test]
    fn test_lr_reset() {
        let mut lr = LinearRegressionMA::new(3);
        for i in 1..=5 {
            lr.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }
        assert!(lr.is_ready());

        lr.reset();
        assert!(!lr.is_ready());
        assert_eq!(lr.slope(), 0.0);
    }

}
