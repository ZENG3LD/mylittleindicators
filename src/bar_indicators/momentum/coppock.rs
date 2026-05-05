//! Coppock Curve indicator.

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::ohlcv_field::OhlcvField;
use std::collections::VecDeque;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Coppock Curve - long-term momentum indicator for market bottoms.
///
/// Coppock = WMA(ROC(roc1_len) + ROC(roc2_len), wma_period)
///
/// Developed by Edwin Coppock. Originally designed to identify buying opportunities
/// in the S&P 500. Best used for identifying long-term bottoms rather than tops.
///
/// Interpretation:
/// - Curve crosses above zero: Buy signal (market bottom)
/// - Curve crosses below zero: Caution (but not a sell signal per original design)
/// - Positive slope while negative: Potential upcoming buy signal
///
/// # Parameters
/// - `roc1_len`: First ROC period (typically 11 months for monthly data)
/// - `roc2_len`: Second ROC period (typically 14 months for monthly data)
/// - `wma_period`: WMA smoothing period (typically 10)
/// - `ma_type`: Type of moving average (default WMA)
/// - `source`: OHLCV field to use as input (default Close)
///
/// # Implementation
///
/// Uses ROC calculation and weighted moving average smoothing.
#[derive(Debug, Clone)]
pub struct CoppockCurve {
    roc1_len: usize,
    roc2_len: usize,
    wma_period: usize,
    ma_type: MovingAverageType,
    source: OhlcvField,
    wma: MovingAverageProvider,
    closes: VecDeque<f64>,
    value: f64,
}

impl CoppockCurve {
    /// Creates a Coppock Curve with default MA type (WMA).
    ///
    /// # Arguments
    /// * `roc1_len` - First ROC period
    /// * `roc2_len` - Second ROC period
    /// * `wma_period` - WMA smoothing period
    pub fn new(roc1_len: usize, roc2_len: usize, wma_period: usize) -> Self {
        Self::new_with_ma_type(roc1_len, roc2_len, wma_period, MovingAverageType::WMA)
    }

    /// Creates a Coppock Curve with specified MA type.
    ///
    /// # Arguments
    /// * `roc1_len` - First ROC period
    /// * `roc2_len` - Second ROC period
    /// * `wma_period` - WMA smoothing period
    /// * `ma_type` - Type of moving average to use
    pub fn new_with_ma_type(roc1_len: usize, roc2_len: usize, wma_period: usize, ma_type: MovingAverageType) -> Self {
        let r1 = roc1_len.max(1);
        let r2 = roc2_len.max(1);
        let wp = wma_period.max(1);
        let cap = r1.max(r2) + 1;
        Self {
            roc1_len: r1,
            roc2_len: r2,
            wma_period: wp,
            ma_type,
            source: OhlcvField::Close,
            wma: MovingAverageProvider::new(ma_type, wp),
            closes: VecDeque::with_capacity(cap),
            value: 0.0,
        }
    }

    /// Creates a new Coppock Curve with custom source field.
    ///
    /// # Arguments
    /// * `roc1_len` - First ROC period
    /// * `roc2_len` - Second ROC period
    /// * `wma_period` - WMA smoothing period
    /// * `source` - OHLCV field to use as input
    pub fn with_source(roc1_len: usize, roc2_len: usize, wma_period: usize, source: OhlcvField) -> Self {
        let mut coppock = Self::new_with_ma_type(roc1_len, roc2_len, wma_period, MovingAverageType::WMA);
        coppock.source = source;
        coppock
    }

    /// Sets the MA type and resets the indicator.
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }

    /// Returns the source field used for calculation.
    #[inline]
    pub fn get_source(&self) -> OhlcvField {
        self.source
    }

    /// Sets the source field and resets the indicator.
    pub fn set_source(&mut self, source: OhlcvField) {
        if self.source != source {
            self.source = source;
            self.reset();
        }
    }

    /// Calculates ROC for given period.
    fn roc(&self, len: usize, c: f64) -> f64 {
        if self.closes.len() <= len {
            return 0.0;
        }
        let base = self.closes[self.closes.len() - 1 - len];
        if base.abs() < 1e-12 {
            0.0
        } else {
            100.0 * (c - base) / base
        }
    }

    /// Updates the Coppock Curve with a new bar and returns the current value.
    ///
    /// Extracts value from the configured source field.
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let price = self.source.extract(open, high, low, close, volume);

        self.closes.push_back(price);
        let max_need = self.roc1_len.max(self.roc2_len) + 1;
        if self.closes.len() > max_need {
            self.closes.pop_front();
        }

        let roc1 = self.roc(self.roc1_len, price);
        let roc2 = self.roc(self.roc2_len, price);
        let s = roc1 + roc2;
        self.value = self.wma.update_bar(0.0, 0.0, 0.0, s, 0.0);
        self.value
    }

    /// Returns the current Coppock Curve value as an `IndicatorValue`.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the Coppock Curve has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.wma.is_ready()
    }

    /// Resets the Coppock Curve to its initial state.
    pub fn reset(&mut self) {
        self.wma = MovingAverageProvider::new(self.ma_type, self.wma_period);
        self.closes.clear();
        self.value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_coppock_basic_calculation() {
        let mut coppock = CoppockCurve::new(11, 14, 10);

        // Feed uptrend data
        for i in 1..=50 {
            coppock.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64 * 2.0, 0.0);
        }

        assert!(coppock.is_ready());
        // In strong uptrend, Coppock should be positive
        assert!(coppock.value().main() > 0.0, "Coppock in uptrend should be positive");
    }

    #[test]
    fn test_coppock_downtrend() {
        let mut coppock = CoppockCurve::new(11, 14, 10);

        // Feed downtrend data
        for i in 1..=50 {
            coppock.update_bar(0.0, 0.0, 0.0, 300.0 - i as f64 * 2.0, 0.0);
        }

        assert!(coppock.is_ready());
        // In strong downtrend, Coppock should be negative
        assert!(coppock.value().main() < 0.0, "Coppock in downtrend should be negative");
    }

    #[test]
    fn test_coppock_reset() {
        let mut coppock = CoppockCurve::new(11, 14, 10);

        for i in 1..=50 {
            coppock.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(coppock.is_ready());

        coppock.reset();
        assert!(!coppock.is_ready());
        assert!(coppock.value().main().abs() < 1e-10);
    }

    #[test]
    fn test_coppock_constant_price() {
        let mut coppock = CoppockCurve::new(5, 7, 5);

        // Constant price = 0 ROC
        for _ in 1..=30 {
            coppock.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        }

        assert!(coppock.is_ready());
        assert!(coppock.value().main().abs() < 0.1, "Coppock with constant price should be near 0");
    }

    #[test]
    fn test_coppock_with_ema() {
        let mut coppock = CoppockCurve::new_with_ma_type(5, 7, 5, MovingAverageType::EMA);

        for i in 1..=30 {
            coppock.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(coppock.is_ready());
        assert!(coppock.value().main() > 0.0);
    }
}
