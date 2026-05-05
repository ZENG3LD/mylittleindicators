//! Exponential Moving Average (EMA) indicator.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Exponential Moving Average (EMA) - weighted average giving more weight to recent prices.
///
/// EMA = α × Price + (1 - α) × EMA_prev, where α = 2 / (period + 1)
///
/// # Implementation
///
/// O(1) update complexity, no buffer needed.
#[derive(Debug, Clone)]
pub struct Ema {
    period: usize,
    alpha: f64,
    source: OhlcvField,
    value: f64,
    count: usize,
}

impl Ema {
    /// Returns the period of this EMA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Creates a new EMA with the specified period.
    ///
    /// # Arguments
    /// * `period` - Smoothing period (α = 2/(period+1))
    pub fn new(period: usize) -> Self {
        Self::with_source(period, OhlcvField::Close)
    }

    /// Creates a new EMA with the specified period and source.
    ///
    /// # Arguments
    /// * `period` - Smoothing period (α = 2/(period+1))
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period: usize, source: OhlcvField) -> Self {
        let alpha = 2.0 / (period as f64 + 1.0);
        Self {
            period,
            alpha,
            source,
            value: 0.0,
            count: 0,
        }
    }

    /// Updates the EMA with a new bar and returns the current value.
    ///
    /// Uses the configured source field to extract the value from OHLCV data.
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let value = self.source.extract(open, high, low, close, volume);

        if self.count == 0 {
            self.value = value;
        } else {
            self.value = self.alpha * value + (1.0 - self.alpha) * self.value;
        }
        self.count += 1;
        self.value
    }
    /// Returns the current EMA value as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns the current EMA value as `f64`.
    pub fn value_f64(&self) -> f64 {
        self.value
    }

    /// Returns `true` if the EMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    /// Resets the EMA to its initial state.
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ema_basic_calculation() {
        let mut ema = Ema::new(3);
        // α = 2/(3+1) = 0.5

        // First bar: EMA = close
        let v1 = ema.update_bar(0.0, 0.0, 0.0, 10.0, 0.0);
        assert!((v1 - 10.0).abs() < 1e-10);

        // Second bar: EMA = 0.5*20 + 0.5*10 = 15
        let v2 = ema.update_bar(0.0, 0.0, 0.0, 20.0, 0.0);
        assert!((v2 - 15.0).abs() < 1e-10);

        // Third bar: EMA = 0.5*30 + 0.5*15 = 22.5
        let v3 = ema.update_bar(0.0, 0.0, 0.0, 30.0, 0.0);
        assert!(ema.is_ready());
        assert!((v3 - 22.5).abs() < 1e-10);
    }

    #[test]
    fn test_ema_alpha_calculation() {
        // Period 9: α = 2/10 = 0.2
        let ema = Ema::new(9);
        assert!((ema.alpha - 0.2).abs() < 1e-10);

        // Period 19: α = 2/20 = 0.1
        let ema = Ema::new(19);
        assert!((ema.alpha - 0.1).abs() < 1e-10);
    }

    #[test]
    fn test_ema_reset() {
        let mut ema = Ema::new(3);

        ema.update_bar(0.0, 0.0, 0.0, 10.0, 0.0);
        ema.update_bar(0.0, 0.0, 0.0, 20.0, 0.0);
        ema.update_bar(0.0, 0.0, 0.0, 30.0, 0.0);
        assert!(ema.is_ready());

        ema.reset();
        assert!(!ema.is_ready());
        assert!((ema.value_f64()).abs() < 1e-10);
    }

    #[test]
    fn test_ema_value_types() {
        let mut ema = Ema::new(2);
        ema.update_bar(0.0, 0.0, 0.0, 10.0, 0.0);
        ema.update_bar(0.0, 0.0, 0.0, 20.0, 0.0);

        let indicator_val = ema.value();
        let f64_val = ema.value_f64();

        if let IndicatorValue::Single(v) = indicator_val {
            assert!((v - f64_val).abs() < 1e-10);
        } else {
            panic!("Expected Single variant");
        }
    }

    #[test]
    fn test_ema_with_different_sources() {
        let bars = vec![
            (100.0, 110.0, 90.0, 105.0, 1000.0),  // (open, high, low, close, volume)
            (105.0, 115.0, 95.0, 110.0, 1200.0),
            (110.0, 120.0, 100.0, 115.0, 800.0),
        ];

        // Test with Close (default)
        let mut ema_close = Ema::new(3);
        for (o, h, l, c, v) in &bars {
            ema_close.update_bar(*o, *h, *l, *c, *v);
        }
        // α = 2/(3+1) = 0.5
        // Bar 1: 105.0
        // Bar 2: 0.5 * 110.0 + 0.5 * 105.0 = 107.5
        // Bar 3: 0.5 * 115.0 + 0.5 * 107.5 = 111.25
        assert!((ema_close.value_f64() - 111.25).abs() < 1e-10);

        // Test with HL2: (high + low) / 2
        let mut ema_hl2 = Ema::with_source(3, OhlcvField::HL2);
        for (o, h, l, c, v) in &bars {
            ema_hl2.update_bar(*o, *h, *l, *c, *v);
        }
        // HL2 values: 100.0, 105.0, 110.0
        // Bar 1: 100.0
        // Bar 2: 0.5 * 105.0 + 0.5 * 100.0 = 102.5
        // Bar 3: 0.5 * 110.0 + 0.5 * 102.5 = 106.25
        assert!((ema_hl2.value_f64() - 106.25).abs() < 1e-10);

        // Test with Open
        let mut ema_open = Ema::with_source(3, OhlcvField::Open);
        for (o, h, l, c, v) in &bars {
            ema_open.update_bar(*o, *h, *l, *c, *v);
        }
        // Open values: 100.0, 105.0, 110.0
        // Bar 1: 100.0
        // Bar 2: 0.5 * 105.0 + 0.5 * 100.0 = 102.5
        // Bar 3: 0.5 * 110.0 + 0.5 * 102.5 = 106.25
        assert!((ema_open.value_f64() - 106.25).abs() < 1e-10);

        // Test with HLC3: (high + low + close) / 3
        let mut ema_hlc3 = Ema::with_source(3, OhlcvField::HLC3);
        for (o, h, l, c, v) in &bars {
            ema_hlc3.update_bar(*o, *h, *l, *c, *v);
        }
        // HLC3 values:
        // Bar 1: (110 + 90 + 105) / 3 = 101.66666...
        // Bar 2: (115 + 95 + 110) / 3 = 106.66666...
        // Bar 3: (120 + 100 + 115) / 3 = 111.66666...
        let hlc3_1 = (110.0 + 90.0 + 105.0) / 3.0;
        let hlc3_2 = (115.0 + 95.0 + 110.0) / 3.0;
        let hlc3_3 = (120.0 + 100.0 + 115.0) / 3.0;
        // EMA calculations:
        // Bar 1: hlc3_1
        // Bar 2: 0.5 * hlc3_2 + 0.5 * hlc3_1
        // Bar 3: 0.5 * hlc3_3 + 0.5 * (0.5 * hlc3_2 + 0.5 * hlc3_1)
        let expected = 0.5 * hlc3_3 + 0.5 * (0.5 * hlc3_2 + 0.5 * hlc3_1);
        assert!((ema_hlc3.value_f64() - expected).abs() < 1e-10);
    }
} 






















