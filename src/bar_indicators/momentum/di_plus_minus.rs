//! Directional Indicator (+DI/-DI)
//!
//! Wrapper around ADX that exposes +DI and -DI as a Double value.
//! +DI measures upward price movement strength
//! -DI measures downward price movement strength

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::momentum::adx::Adx;

/// Directional Indicator (+DI/-DI)
///
/// Returns Double(plus_di, minus_di) for trend direction analysis.
/// Values range from 0 to 100.
/// - When +DI > -DI: uptrend
/// - When -DI > +DI: downtrend
/// - Crossovers signal potential trend changes

#[derive(Clone)]
pub struct DiPlusMinus {
    adx: Adx,
}

impl DiPlusMinus {
    /// Create with default period (14)
    pub fn new() -> Self {
        Self::with_period(14)
    }

    /// Create with custom period
    pub fn with_period(period: usize) -> Self {
        Self {
            adx: Adx::new(period),
        }
    }

    /// Update with new bar data
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) {
        self.adx.update_bar(0.0, high, low, close, 0.0);
    }

    /// Get current +DI value
    pub fn plus_di(&self) -> f64 {
        self.adx.plus_di()
    }

    /// Get current -DI value
    pub fn minus_di(&self) -> f64 {
        self.adx.minus_di()
    }

    /// Get ADX value (trend strength)
    pub fn adx_value(&self) -> f64 {
        if let IndicatorValue::Single(v) = self.adx.value() {
            v
        } else {
            0.0
        }
    }

    /// Returns Double(plus_di, minus_di)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.adx.plus_di(), self.adx.minus_di())
    }

    /// Check if indicator is ready
    pub fn is_ready(&self) -> bool {
        self.adx.is_ready()
    }

    /// Reset indicator state
    pub fn reset(&mut self) {
        self.adx.reset();
    }

    /// Get period
    pub fn period(&self) -> usize {
        self.adx.period()
    }
}

impl Default for DiPlusMinus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_di_plus_minus_creation() {
        let di = DiPlusMinus::new();
        assert_eq!(di.period(), 14);
        assert!(!di.is_ready());
    }

    #[test]
    fn test_di_plus_minus_value() {
        let mut di = DiPlusMinus::with_period(5);

        // Feed some data
        for i in 0..20 {
            let price = 100.0 + (i as f64) * 0.5;
            di.update_bar(price, price + 1.0, price - 1.0, price + 0.5, 1000.0);
        }

        let value = di.value();
        match value {
            IndicatorValue::Double(plus, minus) => {
                assert!(plus >= 0.0 && plus <= 100.0);
                assert!(minus >= 0.0 && minus <= 100.0);
            }
            _ => panic!("Expected Double value"),
        }
    }
}
