//! Stochastic RSI indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::momentum::rsi::Rsi;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Stochastic RSI - applies Stochastic formula to RSI values.
///
/// Stoch RSI = (RSI - Lowest RSI) / (Highest RSI - Lowest RSI)
/// %K = SMA(Stoch RSI, k_period)
/// %D = SMA(%K, d_period)
///
/// Combines the sensitivity of Stochastic with RSI to create a more responsive
/// momentum oscillator. Values range from 0 to 100.
///
/// Interpretation:
/// - Stoch RSI >= 80: Overbought
/// - Stoch RSI <= 20: Oversold
/// - %K crossing %D: Trading signals
///
/// # Parameters
/// - `rsi_period`: Period for RSI calculation
/// - `stoch_period`: Period for Stochastic calculation on RSI
/// - `k_period`: %K smoothing period
/// - `d_period`: %D smoothing period
/// - `source`: OHLCV field to use for RSI calculation
/// - `k_ma_type`: Moving average type for %K smoothing
/// - `d_ma_type`: Moving average type for %D smoothing
///
/// # Implementation
///
/// Uses internal RSI indicator and applies Stochastic formula to RSI values.
/// Supports configurable MA types for smoothing %K and %D lines.
#[derive(Clone)]
pub struct StochasticRsi {
    stoch_period: usize,
    k_period: usize,
    #[allow(dead_code)]
    d_period: usize,

    // 🚀 RSI calculation - используем стандартный RSI
    rsi: Rsi,
    rsi_values: ArrayVec<f64, 512>,

    // Stochastic calculation - raw %K values for manual smoothing
    k_values: ArrayVec<f64, 512>,

    // MA-based smoothing for %K and %D
    k_ma: MovingAverageProvider,
    d_ma: MovingAverageProvider,
    #[allow(dead_code)]
    k_ma_type: MovingAverageType,
    #[allow(dead_code)]
    d_ma_type: MovingAverageType,
    #[allow(dead_code)]
    source: OhlcvField,

    // Current values
    current_k: f64,
    current_d: f64,

    // State
    count: usize,
    is_ready: bool,
}

impl StochasticRsi {
    /// Creates a new Stochastic RSI with default settings.
    ///
    /// Uses Close price source and SMA for smoothing %K and %D lines.
    ///
    /// # Arguments
    /// * `rsi_period` - Period for RSI calculation
    /// * `stoch_period` - Period for Stochastic calculation on RSI
    /// * `k_period` - %K smoothing period
    /// * `d_period` - %D smoothing period
    pub fn new(rsi_period: usize, stoch_period: usize, k_period: usize, d_period: usize) -> Self {
        Self::with_ma_types(rsi_period, stoch_period, k_period, d_period, MovingAverageType::SMA, MovingAverageType::SMA)
    }

    /// Creates a new Stochastic RSI with custom source field.
    ///
    /// Uses SMA for smoothing %K and %D lines.
    ///
    /// # Arguments
    /// * `rsi_period` - Period for RSI calculation
    /// * `stoch_period` - Period for Stochastic calculation on RSI
    /// * `k_period` - %K smoothing period
    /// * `d_period` - %D smoothing period
    /// * `source` - OHLCV field to use for RSI calculation
    pub fn with_source(
        rsi_period: usize,
        stoch_period: usize,
        k_period: usize,
        d_period: usize,
        source: OhlcvField
    ) -> Self {
        Self::with_full_config(
            rsi_period,
            stoch_period,
            k_period,
            d_period,
            MovingAverageType::SMA,
            MovingAverageType::SMA,
            source
        )
    }

    /// Creates a new Stochastic RSI with custom MA types for %K and %D smoothing.
    ///
    /// Uses Close price source.
    ///
    /// # Arguments
    /// * `rsi_period` - Period for RSI calculation
    /// * `stoch_period` - Period for Stochastic calculation on RSI
    /// * `k_period` - %K smoothing period
    /// * `d_period` - %D smoothing period
    /// * `k_ma_type` - Moving average type for %K smoothing
    /// * `d_ma_type` - Moving average type for %D smoothing
    pub fn with_ma_types(
        rsi_period: usize,
        stoch_period: usize,
        k_period: usize,
        d_period: usize,
        k_ma_type: MovingAverageType,
        d_ma_type: MovingAverageType
    ) -> Self {
        Self::with_full_config(
            rsi_period,
            stoch_period,
            k_period,
            d_period,
            k_ma_type,
            d_ma_type,
            OhlcvField::Close
        )
    }

    /// Creates a new Stochastic RSI with full configuration.
    ///
    /// Allows customization of source field and MA types for %K and %D smoothing.
    ///
    /// # Arguments
    /// * `rsi_period` - Period for RSI calculation
    /// * `stoch_period` - Period for Stochastic calculation on RSI
    /// * `k_period` - %K smoothing period
    /// * `d_period` - %D smoothing period
    /// * `k_ma_type` - Moving average type for %K smoothing
    /// * `d_ma_type` - Moving average type for %D smoothing
    /// * `source` - OHLCV field to use for RSI calculation
    pub fn with_full_config(
        rsi_period: usize,
        stoch_period: usize,
        k_period: usize,
        d_period: usize,
        k_ma_type: MovingAverageType,
        d_ma_type: MovingAverageType,
        source: OhlcvField
    ) -> Self {
        Self {
            stoch_period,
            k_period,
            d_period,
            rsi: Rsi::with_source(rsi_period, MovingAverageType::RMA, source),
            rsi_values: ArrayVec::new(),
            k_values: ArrayVec::new(),
            k_ma: MovingAverageProvider::new(k_ma_type, k_period),
            d_ma: MovingAverageProvider::new(d_ma_type, d_period),
            k_ma_type,
            d_ma_type,
            source,
            current_k: 50.0,
            current_d: 50.0,
            count: 0,
            is_ready: false,
        }
    }
    
    /// Updates the Stochastic RSI with a new bar and returns (%K, %D).
    ///
    /// Uses the configured source field to extract value from OHLCV data.
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> (f64, f64) {
        // Update RSI using configured source (RSI returns 0-100 scale directly)
        let rsi_value = self.rsi.update_bar(open, high, low, close, volume);

        // Add RSI value to buffer only when RSI is ready
        if self.rsi.is_ready() {
            if self.rsi_values.len() >= self.stoch_period {
                self.rsi_values.remove(0);
            }
            self.rsi_values.push(rsi_value);
        }

        // Calculate raw stochastic value
        if self.rsi_values.len() >= self.stoch_period {
            self.calculate_stochastic(rsi_value);
        }

        // Smooth %K using configured MA
        if self.k_values.len() >= self.k_period {
            self.smooth_k();
        }

        // Smooth %D using configured MA (smoothed %K values)
        if self.k_ma.is_ready() {
            self.smooth_d();
        }

        // Ready when all components are ready
        if self.rsi.is_ready() && self.k_ma.is_ready() && self.d_ma.is_ready() {
            self.is_ready = true;
        }

        self.count += 1;

        (self.current_k, self.current_d)
    }
    
    fn calculate_stochastic(&mut self, current_rsi: f64) {
        if self.rsi_values.len() < self.stoch_period {
            return;
        }

        let len = self.rsi_values.len();
        let start_idx = len - self.stoch_period;
        let rsi_slice = &self.rsi_values[start_idx..];

        let highest_rsi = rsi_slice.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let lowest_rsi = rsi_slice.iter().fold(f64::INFINITY, |a, &b| a.min(b));

        let raw_k = if (highest_rsi - lowest_rsi).abs() < 1e-12 {
            50.0
        } else {
            ((current_rsi - lowest_rsi) / (highest_rsi - lowest_rsi)) * 100.0
        };

        if self.k_values.len() >= self.k_period {
            self.k_values.remove(0);
        }
        self.k_values.push(raw_k);
    }
    
    fn smooth_k(&mut self) {
        if self.k_values.len() >= self.k_period {
            // Get the latest raw %K value
            let raw_k = self.k_values[self.k_values.len() - 1];

            // Smooth using configured MA type
            let _update = self.k_ma.update_bar(0.0, 0.0, 0.0, raw_k, 0.0);
            self.current_k = self.k_ma.value().main();
        }
    }

    fn smooth_d(&mut self) {
        // Smooth %D as MA of smoothed %K values
        let _update = self.d_ma.update_bar(0.0, 0.0, 0.0, self.current_k, 0.0);
        self.current_d = self.d_ma.value().main();
    }
    
    /// Returns the current (%K, %D) values.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.current_k, self.current_d)
    }

    /// Returns `true` if the indicator has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Returns the RSI period.
    #[inline]
    pub fn period(&self) -> usize {
        self.rsi.period()
    }

    /// Resets the indicator to its initial state.
    pub fn reset(&mut self) {
        self.rsi.reset();
        self.rsi_values.clear();
        self.k_values.clear();
        self.k_ma.reset();
        self.d_ma.reset();
        self.current_k = 50.0;
        self.current_d = 50.0;
        self.count = 0;
        self.is_ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_stoch_rsi_basic_calculation() {
        let mut stoch_rsi = StochasticRsi::new(14, 14, 3, 3);

        // Feed uptrend data
        for i in 1..=60 {
            stoch_rsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(stoch_rsi.is_ready());
        if let IndicatorValue::Double(k, d) = stoch_rsi.value() {
            // Values should be in [0, 100]
            assert!(k >= 0.0 && k <= 100.0, "K should be in [0, 100]");
            assert!(d >= 0.0 && d <= 100.0, "D should be in [0, 100]");
        } else { panic!("Expected Double"); }
    }

    #[test]
    fn test_stoch_rsi_range() {
        let mut stoch_rsi = StochasticRsi::new(14, 14, 3, 3);

        // Feed oscillating data to test range
        for i in 1..=80 {
            let price = 100.0 + (i % 20) as f64 * 2.0;
            stoch_rsi.update_bar(0.0, 0.0, 0.0, price, 0.0);
        }

        assert!(stoch_rsi.is_ready());
        if let IndicatorValue::Double(k, d) = stoch_rsi.value() {
            // Values should be in [0, 100]
            assert!(k >= 0.0 && k <= 100.0, "K should be in [0, 100]");
            assert!(d >= 0.0 && d <= 100.0, "D should be in [0, 100]");
        } else { panic!("Expected Double"); }
    }

    #[test]
    fn test_stoch_rsi_k_d_relationship() {
        let mut stoch_rsi = StochasticRsi::new(14, 14, 3, 3);

        // Feed data
        for i in 1..=80 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            stoch_rsi.update_bar(0.0, 0.0, 0.0, price, 0.0);
        }

        assert!(stoch_rsi.is_ready());
        // D is smoothed K, both should be in valid range
        if let IndicatorValue::Double(k, d) = stoch_rsi.value() {
            assert!(k >= 0.0 && k <= 100.0);
            assert!(d >= 0.0 && d <= 100.0);
        } else { panic!("Expected Double"); }
    }

    #[test]
    fn test_stoch_rsi_reset() {
        let mut stoch_rsi = StochasticRsi::new(14, 14, 3, 3);

        for i in 1..=60 {
            stoch_rsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(stoch_rsi.is_ready());

        stoch_rsi.reset();
        assert!(!stoch_rsi.is_ready());
        assert!((stoch_rsi.value().main() - 50.0).abs() < 0.1); // Default K is 50
    }

    #[test]
    fn test_stoch_rsi_period() {
        let stoch_rsi = StochasticRsi::new(14, 14, 3, 3);
        assert_eq!(stoch_rsi.period(), 14);
    }

    #[test]
    fn test_stoch_rsi_with_source_hl2() {
        let mut stoch_rsi = StochasticRsi::with_source(14, 14, 3, 3, OhlcvField::HL2);

        // Feed data with trending HL2 values
        for i in 1..=60 {
            let high = 110.0 + i as f64;
            let low = 90.0 + i as f64;
            stoch_rsi.update_bar(100.0, high, low, 105.0, 0.0);
        }

        assert!(stoch_rsi.is_ready());
        if let IndicatorValue::Double(k, d) = stoch_rsi.value() {
            assert!(k >= 0.0 && k <= 100.0, "K should be in [0, 100]");
            assert!(d >= 0.0 && d <= 100.0, "D should be in [0, 100]");
        } else { panic!("Expected Double"); }
    }

    #[test]
    fn test_stoch_rsi_with_ma_types_ema() {
        let mut stoch_rsi = StochasticRsi::with_ma_types(
            14, 14, 3, 3,
            MovingAverageType::EMA,
            MovingAverageType::EMA
        );

        // Feed uptrend data
        for i in 1..=60 {
            stoch_rsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(stoch_rsi.is_ready());
        if let IndicatorValue::Double(k, d) = stoch_rsi.value() {
            assert!(k >= 0.0 && k <= 100.0);
            assert!(d >= 0.0 && d <= 100.0);
        } else { panic!("Expected Double"); }
    }

    #[test]
    fn test_stoch_rsi_with_full_config() {
        let mut stoch_rsi = StochasticRsi::with_full_config(
            14, 14, 3, 3,
            MovingAverageType::EMA,
            MovingAverageType::SMA,
            OhlcvField::HLC3
        );

        // Feed data where HLC3 is trending
        for i in 1..=60 {
            let base = 100.0 + i as f64;
            stoch_rsi.update_bar(base, base + 10.0, base - 10.0, base + 5.0, 0.0);
        }

        assert!(stoch_rsi.is_ready());
        if let IndicatorValue::Double(k, d) = stoch_rsi.value() {
            assert!(k >= 0.0 && k <= 100.0);
            assert!(d >= 0.0 && d <= 100.0);
        } else { panic!("Expected Double"); }
    }

    #[test]
    fn test_stoch_rsi_different_sources_produce_different_results() {
        let mut stoch_close = StochasticRsi::with_source(5, 5, 3, 3, OhlcvField::Close);
        let mut stoch_open = StochasticRsi::with_source(5, 5, 3, 3, OhlcvField::Open);

        // Feed data with diverging open/close
        for i in 1..=30 {
            let open = 105.0 - (i as f64 * 0.5);   // Trending down
            let close = 100.0 + (i as f64 * 0.5);  // Trending up
            stoch_close.update_bar(open, 120.0, 80.0, close, 1000.0);
            stoch_open.update_bar(open, 120.0, 80.0, close, 1000.0);
        }

        if stoch_close.is_ready() && stoch_open.is_ready() {
            let val_close = stoch_close.value();
            let val_open = stoch_open.value();

            // They should produce different results since sources are trending differently
            if let (IndicatorValue::Double(k_close, _), IndicatorValue::Double(k_open, _)) = (val_close, val_open) {
                // Different sources should yield different K values
                assert_ne!(k_close, k_open, "Different sources should produce different K values");
            }
        }
    }
}


















