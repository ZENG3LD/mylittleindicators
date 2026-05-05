// Gator Oscillator: difference of two smoothed MAs

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Gator Oscillator - difference between fast and slow moving averages.
///
/// Value = Fast MA - Slow MA
///
/// The Gator Oscillator measures the divergence between two moving averages.
/// Positive values indicate the fast MA is above the slow MA (bullish momentum),
/// while negative values indicate bearish momentum.
///
/// # Parameters
/// - `fast_period`: Fast moving average period
/// - `slow_period`: Slow moving average period
/// - `ma_type`: Type of moving average to use (default EMA)
/// - `source`: OHLCV field to use as input (default Close)
///
/// # Implementation
///
/// Uses configurable moving average types. O(1) per update.
#[derive(Debug, Clone)]
pub struct GatorOscillator {
    fast_period: usize,
    slow_period: usize,
    ma_type: MovingAverageType,
    source: OhlcvField,
    fast: MovingAverageProvider,
    slow: MovingAverageProvider,
    value: f64,
}

impl GatorOscillator {
    /// Creates a new Gator Oscillator with default MA type (EMA) and Close source.
    ///
    /// # Arguments
    /// * `fast_period` - Fast moving average period
    /// * `slow_period` - Slow moving average period
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self::new_with_ma_type(fast_period, slow_period, MovingAverageType::EMA)
    }

    /// Creates a new Gator Oscillator with specified MA type.
    ///
    /// # Arguments
    /// * `fast_period` - Fast moving average period
    /// * `slow_period` - Slow moving average period
    /// * `ma_type` - Type of moving average to use
    pub fn new_with_ma_type(fast_period: usize, slow_period: usize, ma_type: MovingAverageType) -> Self {
        let fast = fast_period.max(1);
        let slow = slow_period.max(2);
        Self {
            fast_period: fast,
            slow_period: slow,
            ma_type,
            source: OhlcvField::Close,
            fast: MovingAverageProvider::new(ma_type, fast),
            slow: MovingAverageProvider::new(ma_type, slow),
            value: 0.0,
        }
    }

    /// Creates a new Gator Oscillator with custom source field.
    ///
    /// # Arguments
    /// * `fast_period` - Fast moving average period
    /// * `slow_period` - Slow moving average period
    /// * `source` - OHLCV field to use as input
    pub fn with_source(fast_period: usize, slow_period: usize, source: OhlcvField) -> Self {
        let mut gator = Self::new_with_ma_type(fast_period, slow_period, MovingAverageType::EMA);
        gator.source = source;
        gator
    }

    /// Creates a new Gator Oscillator with full configuration including source and MA type.
    ///
    /// # Arguments
    /// * `fast_period` - Fast moving average period
    /// * `slow_period` - Slow moving average period
    /// * `source` - OHLCV field to use as input
    /// * `ma_type` - Type of moving average to use
    pub fn with_full_config(
        fast_period: usize,
        slow_period: usize,
        source: OhlcvField,
        ma_type: MovingAverageType,
    ) -> Self {
        let fast = fast_period.max(1);
        let slow = slow_period.max(2);
        Self {
            fast_period: fast,
            slow_period: slow,
            ma_type,
            source,
            fast: MovingAverageProvider::new(ma_type, fast),
            slow: MovingAverageProvider::new(ma_type, slow),
            value: 0.0,
        }
    }

    /// Sets the MA type and resets the indicator.
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        if self.ma_type != ma_type {
            self.ma_type = ma_type;
            self.fast = MovingAverageProvider::new(ma_type, self.fast_period);
            self.slow = MovingAverageProvider::new(ma_type, self.slow_period);
            self.value = 0.0;
        }
    }

    /// Returns the MA type used.
    #[inline]
    pub fn get_ma_type(&self) -> MovingAverageType {
        self.ma_type
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

    /// Resets the Gator Oscillator to its initial state.
    #[inline]
    pub fn reset(&mut self) {
        self.fast = MovingAverageProvider::new(self.ma_type, self.fast_period);
        self.slow = MovingAverageProvider::new(self.ma_type, self.slow_period);
        self.value = 0.0;
    }

    /// Returns `true` if the Gator Oscillator has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.fast.is_ready() && self.slow.is_ready()
    }

    /// Returns the Gator Oscillator value as a typed `IndicatorValue`.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Updates the Gator Oscillator with a new bar and returns the value.
    ///
    /// Extracts value from the configured source field.
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let price = self.source.extract(o, h, l, c, v);
        let f = self.fast.update_bar(0.0, 0.0, 0.0, price, 0.0);
        let s = self.slow.update_bar(0.0, 0.0, 0.0, price, 0.0);
        self.value = f - s;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gator_creation() {
        let gator = GatorOscillator::new(5, 13);
        assert!(!gator.is_ready());
        assert_eq!(gator.value().main(), 0.0);
    }

    #[test]
    fn test_gator_uptrend() {
        let mut gator = GatorOscillator::new(5, 13);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            gator.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(gator.is_ready());
        // In uptrend, fast EMA > slow EMA
        assert!(gator.value().main() > 0.0, "Gator should be positive in uptrend, got {}", gator.value().main());
    }

    #[test]
    fn test_gator_downtrend() {
        let mut gator = GatorOscillator::new(5, 13);
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            gator.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(gator.is_ready());
        // In downtrend, fast EMA < slow EMA
        assert!(gator.value().main() < 0.0, "Gator should be negative in downtrend, got {}", gator.value().main());
    }

    #[test]
    fn test_gator_reset() {
        let mut gator = GatorOscillator::new(5, 13);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            gator.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(gator.is_ready());
        gator.reset();
        assert!(!gator.is_ready());
        assert_eq!(gator.value().main(), 0.0);
    }

    #[test]
    fn test_gator_finite_values() {
        let mut gator = GatorOscillator::new(5, 13);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = gator.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "Gator should always be finite");
        }
    }

    #[test]
    fn test_gator_with_sma() {
        let mut gator = GatorOscillator::new_with_ma_type(5, 13, MovingAverageType::SMA);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            gator.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(gator.is_ready());
        assert_eq!(gator.get_ma_type(), MovingAverageType::SMA);
        assert!(gator.value().main() > 0.0);
    }

    #[test]
    fn test_gator_with_high_source() {
        let mut gator = GatorOscillator::with_source(5, 13, OhlcvField::High);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            gator.update_bar(price, price + 10.0, price - 5.0, price + 5.0, 1000.0);
        }
        assert!(gator.is_ready());
        assert_eq!(gator.get_source(), OhlcvField::High);
        assert!(gator.value().main() > 0.0);
    }

    #[test]
    fn test_gator_with_full_config() {
        let mut gator = GatorOscillator::with_full_config(
            5, 13,
            OhlcvField::HL2,
            MovingAverageType::WMA
        );
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            gator.update_bar(price, price + 10.0, price - 5.0, price + 5.0, 1000.0);
        }
        assert!(gator.is_ready());
        assert_eq!(gator.get_source(), OhlcvField::HL2);
        assert_eq!(gator.get_ma_type(), MovingAverageType::WMA);
        assert!(gator.value().main() > 0.0);
    }

    #[test]
    fn test_gator_set_ma_type() {
        let mut gator = GatorOscillator::new(5, 13);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            gator.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(gator.is_ready());

        gator.set_ma_type(MovingAverageType::SMA);
        assert_eq!(gator.get_ma_type(), MovingAverageType::SMA);
        // Value should be reset
        assert_eq!(gator.value().main(), 0.0);
    }

    #[test]
    fn test_gator_set_source() {
        let mut gator = GatorOscillator::new(5, 13);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            gator.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(gator.is_ready());

        gator.set_source(OhlcvField::High);
        assert!(!gator.is_ready()); // Should reset
        assert_eq!(gator.get_source(), OhlcvField::High);
    }

    #[test]
    fn test_gator_default_source_is_close() {
        let mut gator_default = GatorOscillator::new(5, 13);
        let mut gator_close = GatorOscillator::with_source(5, 13, OhlcvField::Close);

        // Both should produce the same result
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            let v1 = gator_default.update_bar(price, price + 10.0, price - 5.0, price, 1000.0);
            let v2 = gator_close.update_bar(price, price + 10.0, price - 5.0, price, 1000.0);
            assert!((v1 - v2).abs() < 1e-10, "Default and Close source should match");
        }
    }
}
