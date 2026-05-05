//! Absolute Price Oscillator (APO) indicator.

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Absolute Price Oscillator (APO) - measures momentum using the difference between two MAs.
///
/// APO = Fast MA - Slow MA
///
/// Unlike PPO which expresses the difference as a percentage, APO shows the
/// absolute price difference. This makes APO values dependent on the price level.
///
/// Interpretation:
/// - APO > 0: Bullish momentum (fast MA above slow MA)
/// - APO < 0: Bearish momentum (fast MA below slow MA)
/// - Zero crossovers: Potential trend change signals
///
/// # Parameters
/// - `fast_period`: Fast moving average period (typically 12)
/// - `slow_period`: Slow moving average period (typically 26)
/// - `ma_type`: Type of moving average (default EMA)
///
/// # Implementation
///
/// Uses configurable moving average types. O(1) per update.
#[derive(Debug, Clone)]
pub struct Apo {
    fast_period: usize,
    slow_period: usize,
    ma_type: MovingAverageType,
    fast_ma: MovingAverageProvider,
    slow_ma: MovingAverageProvider,
    value: f64,
    ready: bool,
}

impl Apo {
    /// Creates a new APO with default MA type (EMA).
    ///
    /// # Arguments
    /// * `fast_period` - Fast moving average period
    /// * `slow_period` - Slow moving average period
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self::new_with_ma_type(fast_period, slow_period, MovingAverageType::EMA)
    }

    /// Creates a new APO with specified MA type.
    ///
    /// # Arguments
    /// * `fast_period` - Fast moving average period
    /// * `slow_period` - Slow moving average period
    /// * `ma_type` - Type of moving average to use
    pub fn new_with_ma_type(fast_period: usize, slow_period: usize, ma_type: MovingAverageType) -> Self {
        let fast = fast_period.max(1);
        let slow = slow_period.max(1);
        Self {
            fast_period: fast,
            slow_period: slow,
            ma_type,
            fast_ma: MovingAverageProvider::new(ma_type, fast),
            slow_ma: MovingAverageProvider::new(ma_type, slow),
            value: 0.0,
            ready: false,
        }
    }

    /// Sets the MA type and resets the indicator.
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }

    /// Updates the APO with a new bar and returns the current value.
    ///
    /// Only the `close` price is used; other OHLCV fields are ignored.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let fast = self.fast_ma.update_bar(0.0, 0.0, 0.0, c, 0.0);
        let slow = self.slow_ma.update_bar(0.0, 0.0, 0.0, c, 0.0);
        self.value = fast - slow;
        self.ready = self.fast_ma.is_ready() && self.slow_ma.is_ready();
        self.value
    }

    /// Returns the current APO value as an `IndicatorValue`.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the APO has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Resets the APO to its initial state.
    pub fn reset(&mut self) {
        self.fast_ma = MovingAverageProvider::new(self.ma_type, self.fast_period);
        self.slow_ma = MovingAverageProvider::new(self.ma_type, self.slow_period);
        self.value = 0.0;
        self.ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_apo_basic_calculation() {
        let mut apo = Apo::new(12, 26);

        // Feed uptrend data
        for i in 1..=50 {
            apo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(apo.is_ready());
        // In uptrend, fast MA > slow MA, so APO should be positive
        assert!(apo.value().main() > 0.0, "APO in uptrend should be positive");
    }

    #[test]
    fn test_apo_downtrend() {
        let mut apo = Apo::new(12, 26);

        // Feed downtrend data
        for i in 1..=50 {
            apo.update_bar(0.0, 0.0, 0.0, 200.0 - i as f64, 0.0);
        }

        assert!(apo.is_ready());
        // In downtrend, fast MA < slow MA, so APO should be negative
        assert!(apo.value().main() < 0.0, "APO in downtrend should be negative");
    }

    #[test]
    fn test_apo_constant_price() {
        let mut apo = Apo::new(5, 10);

        // Constant price = MAs converge, APO approaches 0
        for _ in 1..=30 {
            apo.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        }

        assert!(apo.is_ready());
        assert!(apo.value().main().abs() < 0.01, "APO with constant price should be near 0");
    }

    #[test]
    fn test_apo_reset() {
        let mut apo = Apo::new(12, 26);

        for i in 1..=50 {
            apo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(apo.is_ready());

        apo.reset();
        assert!(!apo.is_ready());
        assert!(apo.value().main().abs() < 1e-10);
    }

    #[test]
    fn test_apo_with_sma() {
        let mut apo = Apo::new_with_ma_type(5, 10, MovingAverageType::SMA);

        for i in 1..=20 {
            apo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(apo.is_ready());
        assert!(apo.value().main() > 0.0);
    }

    #[test]
    fn test_apo_set_ma_type() {
        let mut apo = Apo::new(5, 10);

        for i in 1..=20 {
            apo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(apo.is_ready());

        apo.set_ma_type(MovingAverageType::SMA);
        assert!(!apo.is_ready()); // Should reset
    }
}
