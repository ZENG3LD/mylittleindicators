//! Percentage Price Oscillator (PPO) indicator.

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Percentage Price Oscillator (PPO) - momentum indicator showing MA difference as percentage.
///
/// PPO Line = 100 × (Fast MA - Slow MA) / Slow MA
/// Signal Line = MA(PPO Line)
/// Histogram = PPO Line - Signal Line
///
/// Similar to MACD but expressed as a percentage, making it comparable across
/// different securities regardless of price level.
///
/// Interpretation:
/// - PPO > 0: Bullish momentum (fast MA above slow MA)
/// - PPO < 0: Bearish momentum
/// - Signal line crossovers: Trading signals
/// - Histogram: Shows momentum strength
///
/// # Parameters
/// - `fast_period`: Fast moving average period (typically 12)
/// - `slow_period`: Slow moving average period (typically 26)
/// - `signal_period`: Signal line period (typically 9)
/// - `fast_ma_type`: Type of moving average for fast line (default EMA)
/// - `slow_ma_type`: Type of moving average for slow line (default EMA)
/// - `signal_ma_type`: Type of moving average for signal line (default EMA)
/// - `source`: OHLCV field to use as input (default Close)
///
/// # Implementation
///
/// Uses configurable moving average types. O(1) per update.
#[derive(Debug, Clone)]
pub struct Ppo {
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    fast_ma_type: MovingAverageType,
    slow_ma_type: MovingAverageType,
    signal_ma_type: MovingAverageType,
    source: OhlcvField,
    fast_ma: MovingAverageProvider,
    slow_ma: MovingAverageProvider,
    signal_ma: MovingAverageProvider,
    value: f64,
    signal: f64,
    ready: bool,
}

impl Ppo {
    /// Creates a new PPO with default MA type (EMA) and Close source.
    ///
    /// # Arguments
    /// * `fast_period` - Fast moving average period
    /// * `slow_period` - Slow moving average period
    /// * `signal_period` - Signal line period
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self::new_with_ma_type(fast_period, slow_period, signal_period, MovingAverageType::EMA)
    }

    /// Creates a new PPO with specified MA type for all components.
    ///
    /// # Arguments
    /// * `fast_period` - Fast moving average period
    /// * `slow_period` - Slow moving average period
    /// * `signal_period` - Signal line period
    /// * `ma_type` - Type of moving average to use for all components
    pub fn new_with_ma_type(fast_period: usize, slow_period: usize, signal_period: usize, ma_type: MovingAverageType) -> Self {
        let fast = fast_period.max(1);
        let slow = slow_period.max(1);
        let signal = signal_period.max(1);
        Self {
            fast_period: fast,
            slow_period: slow,
            signal_period: signal,
            fast_ma_type: ma_type,
            slow_ma_type: ma_type,
            signal_ma_type: ma_type,
            source: OhlcvField::Close,
            fast_ma: MovingAverageProvider::new(ma_type, fast),
            slow_ma: MovingAverageProvider::new(ma_type, slow),
            signal_ma: MovingAverageProvider::new(ma_type, signal),
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    /// Creates a new PPO with custom source field.
    ///
    /// # Arguments
    /// * `fast_period` - Fast moving average period
    /// * `slow_period` - Slow moving average period
    /// * `signal_period` - Signal line period
    /// * `source` - OHLCV field to use as input
    pub fn with_source(fast_period: usize, slow_period: usize, signal_period: usize, source: OhlcvField) -> Self {
        let mut ppo = Self::new_with_ma_type(fast_period, slow_period, signal_period, MovingAverageType::EMA);
        ppo.source = source;
        ppo
    }

    /// Creates a new PPO with different MA types for each component.
    ///
    /// # Arguments
    /// * `fast_period` - Fast moving average period
    /// * `slow_period` - Slow moving average period
    /// * `signal_period` - Signal line period
    /// * `fast_ma_type` - MA type for fast line
    /// * `slow_ma_type` - MA type for slow line
    /// * `signal_ma_type` - MA type for signal line
    pub fn with_ma_types(
        fast_period: usize,
        slow_period: usize,
        signal_period: usize,
        fast_ma_type: MovingAverageType,
        slow_ma_type: MovingAverageType,
        signal_ma_type: MovingAverageType,
    ) -> Self {
        let fast = fast_period.max(1);
        let slow = slow_period.max(1);
        let signal = signal_period.max(1);
        Self {
            fast_period: fast,
            slow_period: slow,
            signal_period: signal,
            fast_ma_type,
            slow_ma_type,
            signal_ma_type,
            source: OhlcvField::Close,
            fast_ma: MovingAverageProvider::new(fast_ma_type, fast),
            slow_ma: MovingAverageProvider::new(slow_ma_type, slow),
            signal_ma: MovingAverageProvider::new(signal_ma_type, signal),
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    /// Creates a new PPO with full configuration including source and MA types.
    ///
    /// # Arguments
    /// * `fast_period` - Fast moving average period
    /// * `slow_period` - Slow moving average period
    /// * `signal_period` - Signal line period
    /// * `source` - OHLCV field to use as input
    /// * `fast_ma_type` - MA type for fast line
    /// * `slow_ma_type` - MA type for slow line
    /// * `signal_ma_type` - MA type for signal line
    pub fn with_full_config(
        fast_period: usize,
        slow_period: usize,
        signal_period: usize,
        source: OhlcvField,
        fast_ma_type: MovingAverageType,
        slow_ma_type: MovingAverageType,
        signal_ma_type: MovingAverageType,
    ) -> Self {
        let fast = fast_period.max(1);
        let slow = slow_period.max(1);
        let signal = signal_period.max(1);
        Self {
            fast_period: fast,
            slow_period: slow,
            signal_period: signal,
            fast_ma_type,
            slow_ma_type,
            signal_ma_type,
            source,
            fast_ma: MovingAverageProvider::new(fast_ma_type, fast),
            slow_ma: MovingAverageProvider::new(slow_ma_type, slow),
            signal_ma: MovingAverageProvider::new(signal_ma_type, signal),
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    /// Sets the MA type for all components and resets the indicator.
    ///
    /// # Deprecated
    /// Use `set_fast_ma_type`, `set_slow_ma_type`, and `set_signal_ma_type` for individual control.
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.fast_ma_type = ma_type;
        self.slow_ma_type = ma_type;
        self.signal_ma_type = ma_type;
        self.reset();
    }

    /// Sets the MA type for the fast line (resets ready state).
    pub fn set_fast_ma_type(&mut self, ma_type: MovingAverageType) {
        if self.fast_ma_type != ma_type {
            self.fast_ma = MovingAverageProvider::new(ma_type, self.fast_period);
            self.fast_ma_type = ma_type;
            self.ready = false;
        }
    }

    /// Sets the MA type for the slow line (resets ready state).
    pub fn set_slow_ma_type(&mut self, ma_type: MovingAverageType) {
        if self.slow_ma_type != ma_type {
            self.slow_ma = MovingAverageProvider::new(ma_type, self.slow_period);
            self.slow_ma_type = ma_type;
            self.ready = false;
        }
    }

    /// Sets the MA type for the signal line (resets ready state).
    pub fn set_signal_ma_type(&mut self, ma_type: MovingAverageType) {
        if self.signal_ma_type != ma_type {
            self.signal_ma = MovingAverageProvider::new(ma_type, self.signal_period);
            self.signal_ma_type = ma_type;
            self.ready = false;
        }
    }

    /// Returns the MA type used for the fast line.
    #[inline]
    pub fn get_fast_ma_type(&self) -> MovingAverageType {
        self.fast_ma_type
    }

    /// Returns the MA type used for the slow line.
    #[inline]
    pub fn get_slow_ma_type(&self) -> MovingAverageType {
        self.slow_ma_type
    }

    /// Returns the MA type used for the signal line.
    #[inline]
    pub fn get_signal_ma_type(&self) -> MovingAverageType {
        self.signal_ma_type
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

    /// Updates the PPO with a new bar and returns the PPO line value.
    ///
    /// Extracts value from the configured source field.
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let price = self.source.extract(open, high, low, close, volume);

        self.fast_ma.update_bar(0.0, 0.0, 0.0, price, 0.0);
        self.slow_ma.update_bar(0.0, 0.0, 0.0, price, 0.0);
        let fast = self.fast_ma.value().main();
        let slow = self.slow_ma.value().main();
        let denom = if slow.abs() < 1e-12 { 1e-12 } else { slow };
        self.value = 100.0 * (fast - slow) / denom;
        if self.fast_ma.is_ready() && self.slow_ma.is_ready() {
            self.signal_ma.update_bar(0.0, 0.0, 0.0, self.value, 0.0);
            self.signal = self.signal_ma.value().main();
        }
        self.ready =
            self.fast_ma.is_ready() && self.slow_ma.is_ready() && self.signal_ma.is_ready();
        self.value
    }

    /// Returns all PPO values as a typed `IndicatorValue`.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Macd {
            line: self.value,
            signal: self.signal,
            histogram: self.value - self.signal,
        }
    }

    /// Returns the PPO line value.
    #[inline]
    pub fn value_ppo(&self) -> f64 {
        self.value
    }

    /// Returns the signal line value.
    #[inline]
    pub fn value_signal(&self) -> f64 {
        self.signal
    }

    /// Returns the histogram value (PPO - Signal).
    #[inline]
    pub fn value_histogram(&self) -> f64 {
        self.value - self.signal
    }

    /// Returns `true` if the PPO has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Resets the PPO to its initial state.
    pub fn reset(&mut self) {
        self.fast_ma = MovingAverageProvider::new(self.fast_ma_type, self.fast_period);
        self.slow_ma = MovingAverageProvider::new(self.slow_ma_type, self.slow_period);
        self.signal_ma = MovingAverageProvider::new(self.signal_ma_type, self.signal_period);
        self.value = 0.0;
        self.signal = 0.0;
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
    fn test_ppo_basic_calculation() {
        let mut ppo = Ppo::new(12, 26, 9);

        // Feed uptrend data
        for i in 1..=50 {
            ppo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(ppo.is_ready());
        // In uptrend, PPO should be positive
        assert!(ppo.value_ppo() > 0.0, "PPO in uptrend should be positive");
    }

    #[test]
    fn test_ppo_downtrend() {
        let mut ppo = Ppo::new(12, 26, 9);

        // Feed downtrend data
        for i in 1..=50 {
            ppo.update_bar(0.0, 0.0, 0.0, 200.0 - i as f64, 0.0);
        }

        assert!(ppo.is_ready());
        // In downtrend, PPO should be negative
        assert!(ppo.value_ppo() < 0.0, "PPO in downtrend should be negative");
    }

    #[test]
    fn test_ppo_constant_price() {
        let mut ppo = Ppo::new(5, 10, 3);

        // Constant price = PPO approaches 0
        for _ in 1..=30 {
            ppo.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        }

        assert!(ppo.is_ready());
        assert!(ppo.value_ppo().abs() < 0.1, "PPO with constant price should be near 0");
    }

    #[test]
    fn test_ppo_signal_line() {
        let mut ppo = Ppo::new(12, 26, 9);

        // Feed trending data
        for i in 1..=60 {
            ppo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64 * 0.5, 0.0);
        }

        assert!(ppo.is_ready());
        // Signal should be smoothed version of PPO, both positive in uptrend
        assert!(ppo.value_signal() > 0.0);
    }

    #[test]
    fn test_ppo_histogram() {
        let mut ppo = Ppo::new(12, 26, 9);

        for i in 1..=50 {
            ppo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(ppo.is_ready());
        // Histogram = PPO - Signal
        let expected_histogram = ppo.value_ppo() - ppo.value_signal();
        assert!((ppo.value_histogram() - expected_histogram).abs() < 1e-10);
    }

    #[test]
    fn test_ppo_reset() {
        let mut ppo = Ppo::new(12, 26, 9);

        for i in 1..=50 {
            ppo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(ppo.is_ready());

        ppo.reset();
        assert!(!ppo.is_ready());
        assert!(ppo.value_ppo().abs() < 1e-10);
        assert!(ppo.value_signal().abs() < 1e-10);
    }

    #[test]
    fn test_ppo_value_returns_macd_type() {
        let mut ppo = Ppo::new(12, 26, 9);

        for i in 1..=50 {
            ppo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        match ppo.value() {
            IndicatorValue::Macd { line, signal, histogram } => {
                assert!((line - ppo.value_ppo()).abs() < 1e-10);
                assert!((signal - ppo.value_signal()).abs() < 1e-10);
                assert!((histogram - ppo.value_histogram()).abs() < 1e-10);
            }
            _ => panic!("Expected Macd variant"),
        }
    }

    #[test]
    fn test_ppo_with_sma() {
        let mut ppo = Ppo::new_with_ma_type(5, 10, 3, MovingAverageType::SMA);

        for i in 1..=30 {
            ppo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(ppo.is_ready());
        assert!(ppo.value_ppo() > 0.0);
    }

    #[test]
    fn test_ppo_with_different_ma_types() {
        let mut ppo = Ppo::with_ma_types(
            12, 26, 9,
            MovingAverageType::EMA,
            MovingAverageType::SMA,
            MovingAverageType::WMA
        );

        for i in 1..=50 {
            ppo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(ppo.is_ready());
        assert_eq!(ppo.get_fast_ma_type(), MovingAverageType::EMA);
        assert_eq!(ppo.get_slow_ma_type(), MovingAverageType::SMA);
        assert_eq!(ppo.get_signal_ma_type(), MovingAverageType::WMA);
    }

    #[test]
    fn test_ppo_with_high_source() {
        let mut ppo = Ppo::with_source(12, 26, 9, OhlcvField::High);

        // Feed bars with varying High prices in uptrend
        for i in 1..=50 {
            ppo.update_bar(100.0, 100.0 + i as f64, 90.0, 105.0, 1000.0);
        }

        assert!(ppo.is_ready());
        assert_eq!(ppo.get_source(), OhlcvField::High);
        // PPO should be positive in uptrend
        assert!(ppo.value_ppo() > 0.0);
    }

    #[test]
    fn test_ppo_with_hl2_source() {
        let mut ppo = Ppo::with_source(12, 26, 9, OhlcvField::HL2);

        // Feed bars where HL2 = (high+low)/2 is trending up
        for i in 1..=50 {
            let base = 100.0 + i as f64;
            ppo.update_bar(base, base + 10.0, base - 10.0, base + 5.0, 1000.0);
        }

        assert!(ppo.is_ready());
        assert_eq!(ppo.get_source(), OhlcvField::HL2);
        // Should show positive PPO in uptrend
        assert!(ppo.value_ppo() > 0.0);
    }

    #[test]
    fn test_ppo_default_source_is_close() {
        let mut ppo_default = Ppo::new(12, 26, 9);
        let mut ppo_close = Ppo::with_source(12, 26, 9, OhlcvField::Close);

        // Both should produce the same result
        for i in 1..=50 {
            let v1 = ppo_default.update_bar(100.0, 110.0, 90.0, 100.0 + i as f64, 1000.0);
            let v2 = ppo_close.update_bar(100.0, 110.0, 90.0, 100.0 + i as f64, 1000.0);
            assert!((v1 - v2).abs() < 1e-10, "Default and Close source should match");
        }

        // Verify final values match
        assert!((ppo_default.value_ppo() - ppo_close.value_ppo()).abs() < 1e-10);
        assert!((ppo_default.value_signal() - ppo_close.value_signal()).abs() < 1e-10);
        assert!((ppo_default.value_histogram() - ppo_close.value_histogram()).abs() < 1e-10);
    }

    #[test]
    fn test_ppo_with_full_config() {
        let mut ppo = Ppo::with_full_config(
            12, 26, 9,
            OhlcvField::HL2,
            MovingAverageType::EMA,
            MovingAverageType::SMA,
            MovingAverageType::WMA
        );

        for i in 1..=50 {
            let base = 100.0 + i as f64;
            ppo.update_bar(base, base + 10.0, base - 10.0, base + 5.0, 1000.0);
        }

        assert!(ppo.is_ready());
        assert_eq!(ppo.get_source(), OhlcvField::HL2);
        assert_eq!(ppo.get_fast_ma_type(), MovingAverageType::EMA);
        assert_eq!(ppo.get_slow_ma_type(), MovingAverageType::SMA);
        assert_eq!(ppo.get_signal_ma_type(), MovingAverageType::WMA);
        assert!(ppo.value_ppo() > 0.0);
    }

    #[test]
    fn test_ppo_set_individual_ma_types() {
        let mut ppo = Ppo::new(12, 26, 9);

        for i in 1..=50 {
            ppo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(ppo.is_ready());

        ppo.set_fast_ma_type(MovingAverageType::SMA);
        assert!(!ppo.is_ready()); // Should reset
        assert_eq!(ppo.get_fast_ma_type(), MovingAverageType::SMA);

        ppo.set_slow_ma_type(MovingAverageType::WMA);
        assert_eq!(ppo.get_slow_ma_type(), MovingAverageType::WMA);

        ppo.set_signal_ma_type(MovingAverageType::SMA);
        assert_eq!(ppo.get_signal_ma_type(), MovingAverageType::SMA);
    }

    #[test]
    fn test_ppo_set_source() {
        let mut ppo = Ppo::new(12, 26, 9);

        for i in 1..=50 {
            ppo.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(ppo.is_ready());

        ppo.set_source(OhlcvField::High);
        assert!(!ppo.is_ready()); // Should reset
        assert_eq!(ppo.get_source(), OhlcvField::High);
    }
}
