use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Percentage Volume Oscillator (PVO) - volume-based momentum indicator.
///
/// PVO Line = 100 × (Fast MA - Slow MA) / Slow MA
/// Signal Line = MA(PVO Line)
/// Histogram = PVO Line - Signal Line
///
/// Similar to PPO but applied to volume instead of price. Helps identify
/// volume trends and divergences with price action.
///
/// Interpretation:
/// - PVO > 0: Volume above average (increased activity)
/// - PVO < 0: Volume below average (decreased activity)
/// - Signal line crossovers: Volume trend changes
/// - Histogram: Shows volume momentum strength
///
/// # Parameters
/// - `fast_period`: Fast moving average period (typically 12)
/// - `slow_period`: Slow moving average period (typically 26)
/// - `signal_period`: Signal line period (typically 9)
/// - `fast_ma_type`: Type of moving average for fast line (default EMA)
/// - `slow_ma_type`: Type of moving average for slow line (default EMA)
/// - `signal_ma_type`: Type of moving average for signal line (default EMA)
///
/// # Implementation
///
/// Uses configurable moving average types. O(1) per update.
#[derive(Debug, Clone)]
pub struct Pvo {
    fast_period: usize,
    slow_period: usize,
    signal_period: usize,
    fast_ma_type: MovingAverageType,
    slow_ma_type: MovingAverageType,
    signal_ma_type: MovingAverageType,
    fast_ma: MovingAverageProvider,
    slow_ma: MovingAverageProvider,
    signal_ma: MovingAverageProvider,
    value: f64,
    signal: f64,
    ready: bool,
}

impl Pvo {
    /// Creates a new PVO with default MA type (EMA).
    ///
    /// # Arguments
    /// * `fast_period` - Fast moving average period
    /// * `slow_period` - Slow moving average period
    /// * `signal_period` - Signal line period
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self::new_with_ma_type(fast_period, slow_period, signal_period, MovingAverageType::EMA)
    }

    /// Creates a new PVO with specified MA type for all components.
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
            fast_ma: MovingAverageProvider::new(ma_type, fast),
            slow_ma: MovingAverageProvider::new(ma_type, slow),
            signal_ma: MovingAverageProvider::new(ma_type, signal),
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    /// Creates a new PVO with different MA types for each component.
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
            fast_ma: MovingAverageProvider::new(fast_ma_type, fast),
            slow_ma: MovingAverageProvider::new(slow_ma_type, slow),
            signal_ma: MovingAverageProvider::new(signal_ma_type, signal),
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    /// Update with new bar (uses volume only)
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, v: f64) -> f64 {
        let _ = self.fast_ma.update_bar(0.0, 0.0, 0.0, v, 0.0);
        let _ = self.slow_ma.update_bar(0.0, 0.0, 0.0, v, 0.0);
        let fast = self.fast_ma.value().main();
        let slow = self.slow_ma.value().main();
        let denom = if slow.abs() < 1e-12 { 1e-12 } else { slow };
        self.value = 100.0 * (fast - slow) / denom;
        if self.fast_ma.is_ready() && self.slow_ma.is_ready() {
            let _ = self.signal_ma.update_bar(0.0, 0.0, 0.0, self.value, 0.0);
            self.signal = self.signal_ma.value().main();
        }
        self.ready =
            self.fast_ma.is_ready() && self.slow_ma.is_ready() && self.signal_ma.is_ready();
        self.value
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

    /// Returns all PVO values as a typed `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Macd {
            line: self.value,
            signal: self.signal,
            histogram: self.value - self.signal,
        }
    }

    /// Returns the PVO line value.
    pub fn value_pvo(&self) -> f64 {
        self.value
    }

    /// Returns the signal line value.
    pub fn value_signal(&self) -> f64 {
        self.signal
    }

    /// Returns the histogram value (PVO - Signal).
    pub fn value_histogram(&self) -> f64 {
        self.value - self.signal
    }

    /// Returns `true` if the PVO has enough data to produce valid values.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Resets the PVO to its initial state.
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

    #[test]
    fn test_pvo_creation() {
        let pvo = Pvo::new(12, 26, 9);
        assert!(!pvo.is_ready());
        assert_eq!(pvo.value_pvo(), 0.0);
        assert_eq!(pvo.value_signal(), 0.0);
    }

    #[test]
    fn test_pvo_warmup() {
        let mut pvo = Pvo::new(12, 26, 9);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pvo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0 + i as f64 * 10.0);
        }
        assert!(pvo.is_ready());
    }

    #[test]
    fn test_pvo_values_finite() {
        let mut pvo = Pvo::new(12, 26, 9);
        for i in 0..50 {
            let price = 100.0 + i as f64;
            let volume = 1000.0 + (i as f64 * 0.2).sin() * 500.0;
            let value = pvo.update_bar(price, price + 1.0, price - 1.0, price, volume);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_pvo_histogram() {
        let mut pvo = Pvo::new(12, 26, 9);
        for i in 0..50 {
            pvo.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0 + i as f64 * 100.0);
        }
        let histogram = pvo.value_histogram();
        assert!(histogram.is_finite());
        assert_eq!(histogram, pvo.value_pvo() - pvo.value_signal());
    }

    #[test]
    fn test_pvo_reset() {
        let mut pvo = Pvo::new(12, 26, 9);
        for i in 0..50 {
            pvo.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0 + i as f64 * 10.0);
        }
        pvo.reset();
        assert!(!pvo.is_ready());
        assert_eq!(pvo.value_pvo(), 0.0);
        assert_eq!(pvo.value_signal(), 0.0);
    }

    #[test]
    fn test_pvo_with_sma() {
        let mut pvo = Pvo::new_with_ma_type(12, 26, 9, MovingAverageType::SMA);
        for i in 0..50 {
            pvo.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0 + i as f64 * 100.0);
        }
        assert!(pvo.is_ready());
        assert_eq!(pvo.get_fast_ma_type(), MovingAverageType::SMA);
        assert_eq!(pvo.get_slow_ma_type(), MovingAverageType::SMA);
        assert_eq!(pvo.get_signal_ma_type(), MovingAverageType::SMA);
    }

    #[test]
    fn test_pvo_with_different_ma_types() {
        let mut pvo = Pvo::with_ma_types(
            12, 26, 9,
            MovingAverageType::EMA,
            MovingAverageType::SMA,
            MovingAverageType::WMA
        );
        for i in 0..50 {
            pvo.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0 + i as f64 * 100.0);
        }
        assert!(pvo.is_ready());
        assert_eq!(pvo.get_fast_ma_type(), MovingAverageType::EMA);
        assert_eq!(pvo.get_slow_ma_type(), MovingAverageType::SMA);
        assert_eq!(pvo.get_signal_ma_type(), MovingAverageType::WMA);
    }

    #[test]
    fn test_pvo_set_individual_ma_types() {
        let mut pvo = Pvo::new(12, 26, 9);
        for i in 0..50 {
            pvo.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0 + i as f64 * 100.0);
        }
        assert!(pvo.is_ready());

        pvo.set_fast_ma_type(MovingAverageType::SMA);
        assert!(!pvo.is_ready()); // Should reset ready state
        assert_eq!(pvo.get_fast_ma_type(), MovingAverageType::SMA);

        pvo.set_slow_ma_type(MovingAverageType::WMA);
        assert_eq!(pvo.get_slow_ma_type(), MovingAverageType::WMA);

        pvo.set_signal_ma_type(MovingAverageType::SMA);
        assert_eq!(pvo.get_signal_ma_type(), MovingAverageType::SMA);
    }

    #[test]
    fn test_pvo_set_ma_type_all() {
        let mut pvo = Pvo::new(12, 26, 9);
        for i in 0..50 {
            pvo.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0 + i as f64 * 100.0);
        }
        assert!(pvo.is_ready());

        pvo.set_ma_type(MovingAverageType::WMA);
        assert!(!pvo.is_ready()); // Should reset
        assert_eq!(pvo.get_fast_ma_type(), MovingAverageType::WMA);
        assert_eq!(pvo.get_slow_ma_type(), MovingAverageType::WMA);
        assert_eq!(pvo.get_signal_ma_type(), MovingAverageType::WMA);
    }

    #[test]
    fn test_pvo_value_returns_macd_type() {
        let mut pvo = Pvo::new(12, 26, 9);
        for i in 0..50 {
            pvo.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0 + i as f64 * 100.0);
        }
        match pvo.value() {
            IndicatorValue::Macd { line, signal, histogram } => {
                assert!((line - pvo.value_pvo()).abs() < 1e-10);
                assert!((signal - pvo.value_signal()).abs() < 1e-10);
                assert!((histogram - pvo.value_histogram()).abs() < 1e-10);
            }
            _ => panic!("Expected Macd variant"),
        }
    }
}
