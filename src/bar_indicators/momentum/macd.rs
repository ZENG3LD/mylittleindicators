//! Moving Average Convergence Divergence (MACD) indicator.

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Moving Average Convergence Divergence (MACD) - trend-following momentum indicator.
///
/// MACD Line = Fast MA - Slow MA
/// Signal Line = MA(MACD Line)
/// Histogram = MACD Line - Signal Line
///
/// Traditional settings: 12/26/9 (fast/slow/signal periods).
///
/// # Signals
/// - MACD crossing above signal: Bullish
/// - MACD crossing below signal: Bearish
/// - Histogram expanding: Trend strengthening
/// - Histogram contracting: Trend weakening
///
/// # Implementation
///
/// Uses configurable moving averages for all three components. O(1) update complexity.
#[derive(Clone)]
pub struct Macd {
    fast_ma: MovingAverageProvider,
    slow_ma: MovingAverageProvider,
    signal_ma: MovingAverageProvider,
    fast_ma_type: MovingAverageType,
    slow_ma_type: MovingAverageType,
    signal_ma_type: MovingAverageType,
    source: OhlcvField,
    fast_source: OhlcvField,
    slow_source: OhlcvField,
    value: f64,
    signal: f64,
    ready: bool,
}

impl Macd {
    /// Creates a new MACD with default signal period (9) using EMA and Close source.
    ///
    /// # Arguments
    /// * `fast_period` - Fast MA period (typically 12)
    /// * `slow_period` - Slow MA period (typically 26)
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self::with_signal(fast_period, slow_period, 9, MovingAverageType::EMA)
    }

    /// Creates a new MACD with custom source field.
    ///
    /// # Arguments
    /// * `fast_period` - Fast MA period (typically 12)
    /// * `slow_period` - Slow MA period (typically 26)
    /// * `source` - OHLCV field to use as input
    pub fn with_source(fast_period: usize, slow_period: usize, source: OhlcvField) -> Self {
        let mut macd = Self::with_signal(fast_period, slow_period, 9, MovingAverageType::EMA);
        macd.source = source;
        macd
    }

    /// Creates a new MACD with custom signal period using EMA.
    ///
    /// # Arguments
    /// * `fast_period` - Fast MA period
    /// * `slow_period` - Slow MA period
    /// * `signal_period` - Signal line MA period
    pub fn new_with_signal(fast_period: usize, slow_period: usize, signal_period: usize) -> Self {
        Self::with_signal(fast_period, slow_period, signal_period, MovingAverageType::EMA)
    }

    /// Creates a new MACD with custom MA type for all components.
    ///
    /// # Arguments
    /// * `fast_period` - Fast MA period
    /// * `slow_period` - Slow MA period
    /// * `signal_period` - Signal line MA period
    /// * `ma_type` - Moving average type for all components
    pub fn with_signal(fast_period: usize, slow_period: usize, signal_period: usize, ma_type: MovingAverageType) -> Self {
        Self {
            fast_ma: MovingAverageProvider::new(ma_type, fast_period),
            slow_ma: MovingAverageProvider::new(ma_type, slow_period),
            signal_ma: MovingAverageProvider::new(ma_type, signal_period),
            fast_ma_type: ma_type,
            slow_ma_type: ma_type,
            signal_ma_type: ma_type,
            source: OhlcvField::Close,
            fast_source: OhlcvField::Close,
            slow_source: OhlcvField::Close,
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    /// Creates a new MACD with different MA types for each component.
    ///
    /// # Arguments
    /// * `fast_period` - Fast MA period
    /// * `slow_period` - Slow MA period
    /// * `signal_period` - Signal line MA period
    /// * `fast_ma_type` - MA type for fast line
    /// * `slow_ma_type` - MA type for slow line
    /// * `signal_ma_type` - MA type for signal line
    pub fn with_different_ma_types(
        fast_period: usize, slow_period: usize, signal_period: usize,
        fast_ma_type: MovingAverageType,
        slow_ma_type: MovingAverageType,
        signal_ma_type: MovingAverageType
    ) -> Self {
        Self {
            fast_ma: MovingAverageProvider::new(fast_ma_type, fast_period),
            slow_ma: MovingAverageProvider::new(slow_ma_type, slow_period),
            signal_ma: MovingAverageProvider::new(signal_ma_type, signal_period),
            fast_ma_type,
            slow_ma_type,
            signal_ma_type,
            source: OhlcvField::Close,
            fast_source: OhlcvField::Close,
            slow_source: OhlcvField::Close,
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    /// Creates a new MACD with different MA types for each component and custom source.
    ///
    /// # Arguments
    /// * `fast_period` - Fast MA period
    /// * `slow_period` - Slow MA period
    /// * `signal_period` - Signal line MA period
    /// * `fast_ma_type` - MA type for fast line
    /// * `slow_ma_type` - MA type for slow line
    /// * `signal_ma_type` - MA type for signal line
    /// * `source` - OHLCV field to use as input
    pub fn with_different_ma_types_and_source(
        fast_period: usize,
        slow_period: usize,
        signal_period: usize,
        fast_ma_type: MovingAverageType,
        slow_ma_type: MovingAverageType,
        signal_ma_type: MovingAverageType,
        source: OhlcvField,
    ) -> Self {
        let mut macd = Self::with_different_ma_types(
            fast_period, slow_period, signal_period,
            fast_ma_type, slow_ma_type, signal_ma_type
        );
        macd.source = source;
        macd.fast_source = source;
        macd.slow_source = source;
        macd
    }

    /// Creates a new MACD with full configuration including per-component sources.
    ///
    /// This is the most flexible constructor, allowing different MA types AND
    /// different source fields for each moving average component.
    ///
    /// # Arguments
    /// * `fast_period` - Fast MA period
    /// * `slow_period` - Slow MA period
    /// * `signal_period` - Signal line MA period
    /// * `fast_ma_type` - MA type for fast line
    /// * `slow_ma_type` - MA type for slow line
    /// * `signal_ma_type` - MA type for signal line
    /// * `fast_source` - OHLCV field for fast MA
    /// * `slow_source` - OHLCV field for slow MA
    pub fn with_full_config(
        fast_period: usize,
        slow_period: usize,
        signal_period: usize,
        fast_ma_type: MovingAverageType,
        slow_ma_type: MovingAverageType,
        signal_ma_type: MovingAverageType,
        fast_source: OhlcvField,
        slow_source: OhlcvField,
    ) -> Self {
        Self {
            fast_ma: MovingAverageProvider::new(fast_ma_type, fast_period),
            slow_ma: MovingAverageProvider::new(slow_ma_type, slow_period),
            signal_ma: MovingAverageProvider::new(signal_ma_type, signal_period),
            fast_ma_type,
            slow_ma_type,
            signal_ma_type,
            source: OhlcvField::Close,
            fast_source,
            slow_source,
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    /// Updates the MACD with a new bar and returns the MACD line value.
    ///
    /// Extracts values from the configured source fields for each component.
    /// Fast and slow MAs can use different sources (e.g., High for fast, Low for slow).
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let fast_price = self.fast_source.extract(open, high, low, close, volume);
        let slow_price = self.slow_source.extract(open, high, low, close, volume);

        let _fast_update = self.fast_ma.update_bar(0.0, 0.0, 0.0, fast_price, 0.0);
        let _slow_update = self.slow_ma.update_bar(0.0, 0.0, 0.0, slow_price, 0.0);

        let fast = self.fast_ma.value().main();
        let slow = self.slow_ma.value().main();
        self.value = fast - slow;

        if self.fast_ma.is_ready() && self.slow_ma.is_ready() {
            let _signal_update = self.signal_ma.update_bar(0.0, 0.0, 0.0, self.value, 0.0);
            self.signal = self.signal_ma.value().main();
        }

        self.ready = self.fast_ma.is_ready() && self.slow_ma.is_ready() && self.signal_ma.is_ready();
        self.value
    }

    /// Returns all MACD values as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Macd {
            line: self.value,
            signal: self.signal,
            histogram: self.value - self.signal,
        }
    }

    /// Returns the MACD line value.
    #[inline]
    pub fn value_macd(&self) -> f64 {
        self.value
    }

    /// Returns the signal line value.
    #[inline]
    pub fn value_signal(&self) -> f64 {
        self.signal
    }

    /// Returns the histogram value (MACD - Signal).
    #[inline]
    pub fn value_histogram(&self) -> f64 {
        self.value - self.signal
    }

    /// Returns `true` if the MACD has received enough bars to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Resets the MACD to its initial state.
    pub fn reset(&mut self) {
        self.fast_ma.reset();
        self.slow_ma.reset();
        self.signal_ma.reset();
        self.value = 0.0;
        self.signal = 0.0;
        self.ready = false;
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

    /// Sets the MA type for the fast line (resets ready state).
    pub fn set_fast_ma_type(&mut self, ma_type: MovingAverageType) {
        if self.fast_ma_type != ma_type {
            let period = self.fast_ma.period();
            self.fast_ma = MovingAverageProvider::new(ma_type, period);
            self.fast_ma_type = ma_type;
            self.ready = false;
        }
    }

    /// Sets the MA type for the slow line (resets ready state).
    pub fn set_slow_ma_type(&mut self, ma_type: MovingAverageType) {
        if self.slow_ma_type != ma_type {
            let period = self.slow_ma.period();
            self.slow_ma = MovingAverageProvider::new(ma_type, period);
            self.slow_ma_type = ma_type;
            self.ready = false;
        }
    }

    /// Sets the MA type for the signal line (resets ready state).
    pub fn set_signal_ma_type(&mut self, ma_type: MovingAverageType) {
        if self.signal_ma_type != ma_type {
            let period = self.signal_ma.period();
            self.signal_ma = MovingAverageProvider::new(ma_type, period);
            self.signal_ma_type = ma_type;
            self.ready = false;
        }
    }

    /// Returns the MACD configuration (periods and MA types).
    pub fn get_config(&self) -> (usize, usize, usize, MovingAverageType, MovingAverageType, MovingAverageType) {
        (
            self.fast_ma.period(),
            self.slow_ma.period(),
            self.signal_ma.period(),
            self.fast_ma_type,
            self.slow_ma_type,
            self.signal_ma_type
        )
    }
}

impl std::fmt::Debug for Macd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Macd")
            .field("fast_period", &self.fast_ma.period())
            .field("slow_period", &self.slow_ma.period())
            .field("signal_period", &self.signal_ma.period())
            .field("fast_ma_type", &self.fast_ma_type)
            .field("slow_ma_type", &self.slow_ma_type)
            .field("signal_ma_type", &self.signal_ma_type)
            .field("source", &self.source)
            .field("fast_source", &self.fast_source)
            .field("slow_source", &self.slow_source)
            .field("value", &self.value)
            .field("signal", &self.signal)
            .field("histogram", &(self.value - self.signal))
            .field("ready", &self.ready)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_macd_basic_calculation() {
        let mut macd = Macd::new(12, 26);

        // Feed uptrend data
        for i in 1..=40 {
            macd.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(macd.is_ready());
        // In uptrend, MACD line should be positive (fast > slow)
        assert!(macd.value_macd() > 0.0, "MACD in uptrend should be positive");
    }

    #[test]
    fn test_macd_downtrend() {
        let mut macd = Macd::new(12, 26);

        // Feed downtrend data
        for i in 1..=40 {
            macd.update_bar(0.0, 0.0, 0.0, 200.0 - i as f64, 0.0);
        }

        assert!(macd.is_ready());
        // In downtrend, MACD line should be negative (fast < slow)
        assert!(macd.value_macd() < 0.0, "MACD in downtrend should be negative");
    }

    #[test]
    fn test_macd_value_types() {
        let mut macd = Macd::new(12, 26);

        for i in 1..=40 {
            macd.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        // Check IndicatorValue structure
        if let IndicatorValue::Macd { line, signal, histogram } = macd.value() {
            assert!((line - macd.value_macd()).abs() < 1e-10);
            assert!((signal - macd.value_signal()).abs() < 1e-10);
            assert!((histogram - macd.value_histogram()).abs() < 1e-10);
        } else {
            panic!("Expected Macd variant");
        }
    }

    #[test]
    fn test_macd_histogram() {
        let mut macd = Macd::new(12, 26);

        for i in 1..=40 {
            macd.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(macd.is_ready());
        let hist = macd.value_histogram();
        let expected = macd.value_macd() - macd.value_signal();
        assert!((hist - expected).abs() < 1e-10);
    }

    #[test]
    fn test_macd_custom_signal_period() {
        let mut macd = Macd::new_with_signal(12, 26, 5);

        for i in 1..=40 {
            macd.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(macd.is_ready());
    }

    #[test]
    fn test_macd_with_ma_type() {
        let mut macd = Macd::with_signal(12, 26, 9, MovingAverageType::SMA);

        for i in 1..=40 {
            macd.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(macd.is_ready());
        assert_eq!(macd.get_fast_ma_type(), MovingAverageType::SMA);
    }

    #[test]
    fn test_macd_reset() {
        let mut macd = Macd::new(12, 26);

        for i in 1..=40 {
            macd.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(macd.is_ready());

        macd.reset();
        assert!(!macd.is_ready());
        assert!((macd.value_macd()).abs() < 1e-10);
        assert!((macd.value_signal()).abs() < 1e-10);
    }

    #[test]
    fn test_macd_get_config() {
        let macd = Macd::new_with_signal(12, 26, 9);
        let (fast, slow, signal, fast_type, slow_type, signal_type) = macd.get_config();

        assert_eq!(fast, 12);
        assert_eq!(slow, 26);
        assert_eq!(signal, 9);
        assert_eq!(fast_type, MovingAverageType::EMA);
        assert_eq!(slow_type, MovingAverageType::EMA);
        assert_eq!(signal_type, MovingAverageType::EMA);
    }

    #[test]
    fn test_macd_set_ma_types() {
        let mut macd = Macd::new(12, 26);

        for i in 1..=40 {
            macd.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(macd.is_ready());

        macd.set_fast_ma_type(MovingAverageType::SMA);
        assert!(!macd.is_ready()); // Should reset
        assert_eq!(macd.get_fast_ma_type(), MovingAverageType::SMA);
    }

    #[test]
    fn test_macd_different_ma_types() {
        let mut macd = Macd::with_different_ma_types(
            12, 26, 9,
            MovingAverageType::EMA,
            MovingAverageType::SMA,
            MovingAverageType::WMA
        );

        for i in 1..=40 {
            macd.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(macd.is_ready());
        assert_eq!(macd.get_fast_ma_type(), MovingAverageType::EMA);
        assert_eq!(macd.get_slow_ma_type(), MovingAverageType::SMA);
        assert_eq!(macd.get_signal_ma_type(), MovingAverageType::WMA);
    }

    #[test]
    fn test_macd_with_high_source() {
        let mut macd = Macd::with_source(12, 26, OhlcvField::High);

        // Feed bars with varying High prices in uptrend
        for i in 1..=40 {
            macd.update_bar(100.0, 100.0 + i as f64, 90.0, 105.0, 1000.0);
        }

        assert!(macd.is_ready());
        // MACD should be positive in uptrend (fast MA > slow MA)
        assert!(macd.value_macd() > 0.0);
    }

    #[test]
    fn test_macd_with_hl2_source() {
        let mut macd = Macd::with_source(12, 26, OhlcvField::HL2);

        // Feed bars where HL2 = (high+low)/2 is trending up
        for i in 1..=40 {
            let base = 100.0 + i as f64;
            macd.update_bar(base, base + 10.0, base - 10.0, base + 5.0, 1000.0);
        }

        assert!(macd.is_ready());
        // Should show positive MACD in uptrend
        assert!(macd.value_macd() > 0.0);
    }

    #[test]
    fn test_macd_default_source_is_close() {
        let mut macd_default = Macd::new(12, 26);
        let mut macd_close = Macd::with_source(12, 26, OhlcvField::Close);

        // Both should produce the same result
        for i in 1..=40 {
            let v1 = macd_default.update_bar(100.0, 110.0, 90.0, 100.0 + i as f64, 1000.0);
            let v2 = macd_close.update_bar(100.0, 110.0, 90.0, 100.0 + i as f64, 1000.0);
            assert!((v1 - v2).abs() < 1e-10, "Default and Close source should match");
        }

        // Verify final values match
        assert!((macd_default.value_macd() - macd_close.value_macd()).abs() < 1e-10);
        assert!((macd_default.value_signal() - macd_close.value_signal()).abs() < 1e-10);
        assert!((macd_default.value_histogram() - macd_close.value_histogram()).abs() < 1e-10);
    }
}
