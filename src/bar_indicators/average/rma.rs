//! Wilder's Moving Average (RMA) indicator.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Wilder's Moving Average (RMA) - also known as Smoothed Moving Average (SMMA).
///
/// RMA = (RMA_prev × (period - 1) + Price) / period
///
/// Equivalent to EMA with α = 1/period (vs EMA's α = 2/(period+1)).
/// Used in RSI, ATR, and ADX calculations.
///
/// # Implementation
///
/// O(1) update complexity, no buffer needed.
#[derive(Debug, Clone)]
pub struct Rma {
    period: usize,
    value: f64,
    count: usize,
    source: OhlcvField,
}

impl Rma {
    /// Returns the period of this RMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Creates a new RMA with the specified period.
    ///
    /// # Arguments
    /// * `period` - Smoothing period (α = 1/period)
    pub fn new(period: usize) -> Self {
        Self::with_source(period, OhlcvField::Close)
    }

    /// Creates a new RMA with the specified period and source field.
    ///
    /// # Arguments
    /// * `period` - Smoothing period (α = 1/period)
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period: usize, source: OhlcvField) -> Self {
        Self {
            period,
            value: 0.0,
            count: 0,
            source,
        }
    }

    /// Updates the RMA with a new bar and returns the current value.
    ///
    /// Uses the configured source field (default: Close).
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let value = self.source.extract(open, high, low, close, volume);

        if self.count == 0 {
            self.value = value;
        } else {
            self.value = (self.value * (self.period as f64 - 1.0) + value) / self.period as f64;
        }
        self.count += 1;
        self.value
    }

    /// Returns the current RMA value as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the RMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    /// Resets the RMA to its initial state.
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rma_basic_calculation() {
        let mut rma = Rma::new(3);
        // α = 1/3

        // First bar: RMA = close
        let v1 = rma.update_bar(0.0, 0.0, 0.0, 10.0, 0.0);
        assert!((v1 - 10.0).abs() < 1e-10);

        // Second bar: RMA = (10*2 + 20)/3 = 40/3 ≈ 13.33
        let v2 = rma.update_bar(0.0, 0.0, 0.0, 20.0, 0.0);
        assert!((v2 - 13.333333).abs() < 0.001);

        // Third bar: RMA = (13.33*2 + 30)/3 ≈ 18.89
        let v3 = rma.update_bar(0.0, 0.0, 0.0, 30.0, 0.0);
        assert!(rma.is_ready());
        assert!((v3 - 18.888888).abs() < 0.001);
    }

    #[test]
    fn test_rma_vs_ema_smoothing() {
        // RMA with period 14 has α = 1/14 ≈ 0.0714
        // EMA with period 27 has α = 2/28 ≈ 0.0714
        // So RMA(14) ≈ EMA(27) in terms of smoothing
        let _rma = Rma::new(14);
        let alpha = 1.0 / 14.0;
        assert!((alpha - 0.0714_f64).abs() < 0.001);
    }

    #[test]
    fn test_rma_reset() {
        let mut rma = Rma::new(3);
        rma.update_bar(0.0, 0.0, 0.0, 10.0, 0.0);
        rma.update_bar(0.0, 0.0, 0.0, 20.0, 0.0);
        rma.update_bar(0.0, 0.0, 0.0, 30.0, 0.0);
        assert!(rma.is_ready());

        rma.reset();
        assert!(!rma.is_ready());
    }

    #[test]
    fn test_rma_with_source_hl2() {
        let mut rma = Rma::with_source(3, OhlcvField::HL2);
        // α = 1/3

        // First bar: HL2 = (110 + 90) / 2 = 100
        let v1 = rma.update_bar(0.0, 110.0, 90.0, 105.0, 0.0);
        assert!((v1 - 100.0).abs() < 1e-10);

        // Second bar: HL2 = (120 + 80) / 2 = 100, RMA = (100*2 + 100)/3 = 100
        let v2 = rma.update_bar(0.0, 120.0, 80.0, 110.0, 0.0);
        assert!((v2 - 100.0).abs() < 1e-10);

        // Third bar: HL2 = (130 + 70) / 2 = 100, RMA = (100*2 + 100)/3 = 100
        let v3 = rma.update_bar(0.0, 130.0, 70.0, 115.0, 0.0);
        assert!(rma.is_ready());
        assert!((v3 - 100.0).abs() < 1e-10);
    }

    #[test]
    fn test_rma_with_source_high() {
        let mut rma = Rma::with_source(2, OhlcvField::High);
        // α = 1/2

        // First bar: High = 110
        let v1 = rma.update_bar(100.0, 110.0, 90.0, 105.0, 0.0);
        assert!((v1 - 110.0).abs() < 1e-10);

        // Second bar: High = 120, RMA = (110*1 + 120)/2 = 115
        let v2 = rma.update_bar(105.0, 120.0, 95.0, 110.0, 0.0);
        assert!((v2 - 115.0).abs() < 1e-10);
    }
}
