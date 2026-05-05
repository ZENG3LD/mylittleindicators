//! Weighted Moving Average (WMA) indicator.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Weighted Moving Average (WMA) - linear weights giving more importance to recent prices.
///
/// WMA = (n×Pn + (n-1)×Pn-1 + ... + 1×P1) / (n + (n-1) + ... + 1)
///
/// # Implementation
///
/// Fast O(1) WMA implementation using running sums.
///
/// Instead of recalculating weighted sum on each update (O(n)),
/// we maintain two running sums:
/// - weighted_sum: sum of (value * weight)
/// - unweighted_sum: sum of values (no weights)
///
/// When sliding the window:
/// 1. Remove old value's contribution
/// 2. All remaining weights decrease by 1 (clever math optimization)
/// 3. Add new value with weight = period
///
/// Time complexity: O(1) per update vs O(period) in naive implementation
const WMA_MAX_PERIOD: usize = 256;

#[derive(Debug, Clone)]
pub struct Wma {
    period: usize,
    buf: Vec<f64>,           // ring buffer
    idx: usize,              // current write position (oldest element)
    filled: bool,
    weight_sum: f64,         // pre-calculated sum of weights: 1+2+...+period
    weighted_sum: f64,       // running weighted sum: w₁·x₁ + w₂·x₂ + ... + wₙ·xₙ
    unweighted_sum: f64,     // running unweighted sum: x₁ + x₂ + ... + xₙ
    value: f64,
    source: OhlcvField,
}

impl Wma {
    /// Creates a new WMA with the specified period.
    ///
    /// # Arguments
    /// * `period` - Number of bars to weight (1..=256)
    ///
    /// # Panics
    /// Panics if period > 256.
    pub fn new(period: usize) -> Self {
        Self::with_source(period, OhlcvField::Close)
    }

    /// Creates a new WMA with the specified period and source field.
    ///
    /// # Arguments
    /// * `period` - Number of bars to weight (1..=256)
    /// * `source` - OHLCV field to use as input
    ///
    /// # Panics
    /// Panics if period > 256.
    pub fn with_source(period: usize, source: OhlcvField) -> Self {
        assert!(period <= WMA_MAX_PERIOD, "period {} exceeds WMA_MAX_PERIOD {}", period, WMA_MAX_PERIOD);
        let period = period.max(1);

        // Weight sum for WMA: 1 + 2 + 3 + ... + n = n(n+1)/2
        let weight_sum = (period * (period + 1)) as f64 / 2.0;

        Self {
            period,
            buf: Vec::with_capacity(period),
            idx: 0,
            filled: false,
            weight_sum,
            weighted_sum: 0.0,
            unweighted_sum: 0.0,
            value: 0.0,
            source,
        }
    }

    /// Returns the period of this WMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Returns the current WMA value as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the WMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Resets the WMA to its initial state.
    pub fn reset(&mut self) {
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.weighted_sum = 0.0;
        self.unweighted_sum = 0.0;
        self.value = 0.0;
    }

    /// Updates the WMA with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close).
    /// O(1) operation after warmup phase.
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let value = self.source.extract(open, high, low, close, volume);

        if self.buf.len() < self.period {
            // Warmup phase: fill buffer and compute initial sums
            self.buf.push(value);
            self.unweighted_sum += value;

            if self.buf.len() == self.period {
                self.filled = true;
                // Calculate initial weighted sum: w₁·x₁ + w₂·x₂ + ... + wₙ·xₙ
                self.weighted_sum = 0.0;
                for (i, &val) in self.buf.iter().enumerate() {
                    let weight = (i + 1) as f64;  // weights: 1, 2, 3, ..., period
                    self.weighted_sum += val * weight;
                }
                self.value = self.weighted_sum / self.weight_sum;
            } else {
                self.value = value;  // Not ready yet
            }
        } else {
            // O(1) sliding window update
            let old_value = self.buf[self.idx];

            // Key optimization: instead of recalculating entire weighted sum,
            // we use the fact that when we remove oldest and add newest:
            // 1. All elements shift their weights down by 1
            // 2. This is equivalent to: new_sum = old_sum - unweighted_sum + period * new_value
            //
            // Proof:
            // Old: w₁·x₁ + w₂·x₂ + ... + wₙ·xₙ
            // New: w₁·x₂ + w₂·x₃ + ... + wₙ·new
            //    = (w₂-1)·x₂ + (w₃-1)·x₃ + ... + (wₙ-1)·xₙ + wₙ·new
            //    = (w₂·x₂ + w₃·x₃ + ... + wₙ·xₙ) - (x₂ + x₃ + ... + xₙ) + wₙ·new
            //    = (old_sum - w₁·x₁) - (unweighted_sum - x₁) + wₙ·new
            //    = old_sum - unweighted_sum + wₙ·new

            self.weighted_sum = self.weighted_sum - self.unweighted_sum + (self.period as f64) * value;
            self.unweighted_sum = self.unweighted_sum - old_value + value;

            // Update ring buffer
            self.buf[self.idx] = value;
            self.idx = (self.idx + 1) % self.period;

            self.value = self.weighted_sum / self.weight_sum;
        }

        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wma_correctness() {
        let mut wma = Wma::new(5);

        // Feed values: [10, 20, 30, 40, 50]
        wma.update_bar(0.0, 0.0, 0.0, 10.0, 0.0);
        wma.update_bar(0.0, 0.0, 0.0, 20.0, 0.0);
        wma.update_bar(0.0, 0.0, 0.0, 30.0, 0.0);
        wma.update_bar(0.0, 0.0, 0.0, 40.0, 0.0);
        let result1 = wma.update_bar(0.0, 0.0, 0.0, 50.0, 0.0);

        // Expected: (1*10 + 2*20 + 3*30 + 4*40 + 5*50) / (1+2+3+4+5)
        //         = (10 + 40 + 90 + 160 + 250) / 15 = 550 / 15 = 36.666...
        assert!((result1 - 36.666666).abs() < 0.001, "Expected ~36.67, got {}", result1);

        // Add new value 60, oldest (10) should be removed
        let result2 = wma.update_bar(0.0, 0.0, 0.0, 60.0, 0.0);

        // Expected: (1*20 + 2*30 + 3*40 + 4*50 + 5*60) / 15
        //         = (20 + 60 + 120 + 200 + 300) / 15 = 700 / 15 = 46.666...
        assert!((result2 - 46.666666).abs() < 0.001, "Expected ~46.67, got {}", result2);
    }

    #[test]
    fn test_wma_period_1() {
        let mut wma = Wma::new(1);

        assert_eq!(wma.update_bar(0.0, 0.0, 0.0, 10.0, 0.0), 10.0);
        assert_eq!(wma.update_bar(0.0, 0.0, 0.0, 20.0, 0.0), 20.0);
        assert_eq!(wma.update_bar(0.0, 0.0, 0.0, 30.0, 0.0), 30.0);
    }

    #[test]
    fn test_wma_reset() {
        let mut wma = Wma::new(3);
        wma.update_bar(0.0, 0.0, 0.0, 10.0, 0.0);
        wma.update_bar(0.0, 0.0, 0.0, 20.0, 0.0);
        wma.update_bar(0.0, 0.0, 0.0, 30.0, 0.0);
        assert!(wma.is_ready());

        wma.reset();
        assert!(!wma.is_ready());
    }

    #[test]
    fn test_wma_with_source_hl2() {
        let mut wma = Wma::with_source(3, OhlcvField::HL2);

        // Bar 1: HL2 = (110 + 90) / 2 = 100
        wma.update_bar(0.0, 110.0, 90.0, 105.0, 0.0);
        // Bar 2: HL2 = (120 + 80) / 2 = 100
        wma.update_bar(0.0, 120.0, 80.0, 110.0, 0.0);
        // Bar 3: HL2 = (130 + 70) / 2 = 100
        let result = wma.update_bar(0.0, 130.0, 70.0, 115.0, 0.0);

        // WMA = (1*100 + 2*100 + 3*100) / (1+2+3) = 600 / 6 = 100
        assert!((result - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_wma_with_source_high() {
        let mut wma = Wma::with_source(3, OhlcvField::High);

        // Bar 1: High = 110
        wma.update_bar(100.0, 110.0, 90.0, 105.0, 0.0);
        // Bar 2: High = 120
        wma.update_bar(105.0, 120.0, 95.0, 110.0, 0.0);
        // Bar 3: High = 130
        let result = wma.update_bar(110.0, 130.0, 100.0, 115.0, 0.0);

        // WMA = (1*110 + 2*120 + 3*130) / (1+2+3) = (110 + 240 + 390) / 6 = 740 / 6 ≈ 123.33
        assert!((result - 123.333333).abs() < 0.001);
    }
}
