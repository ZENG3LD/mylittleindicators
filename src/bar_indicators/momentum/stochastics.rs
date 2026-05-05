//! Stochastic Oscillator (%K, %D) indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Stochastic Oscillator - momentum indicator comparing close to high-low range.
///
/// %K = 100 × (Close - Lowest Low) / (Highest High - Lowest Low)
/// %D = MA(%K)
///
/// Oscillates between 0 and 100. Traditional interpretation:
/// - Above 80: Overbought
/// - Below 20: Oversold
/// - %K crossing %D: Trading signal
///
/// # Parameters
/// - `period_k`: Lookback for %K calculation (typically 14)
/// - `period_d`: Smoothing period for %D (typically 3)
/// - `ma_type`: Moving average type for %D smoothing
///
/// # Implementation
///
/// Uses ring buffer for high/low tracking. O(period_k) per update for min/max scan.
/// Maximum period is 512.
#[derive(Clone)]
pub struct Stochastics {
    period_k: usize,
    period_d: usize,
    highs: ArrayVec<f64, 512>,
    lows: ArrayVec<f64, 512>,
    closes: ArrayVec<f64, 512>,
    d_ma: MovingAverageProvider,
    ma_type: MovingAverageType,
    index: usize,
    filled: bool,
    value_k: f64,
    value_d: f64,
}

impl Stochastics {
    /// Creates a new Stochastic Oscillator with SMA smoothing for %D.
    ///
    /// # Arguments
    /// * `period_k` - Lookback period for %K (1..=512)
    /// * `period_d` - Smoothing period for %D (1..=512)
    pub fn new(period_k: usize, period_d: usize) -> Self {
        Self::with_ma_type(period_k, period_d, MovingAverageType::SMA)
    }

    /// Creates a new Stochastic Oscillator with custom MA type for %D.
    ///
    /// # Arguments
    /// * `period_k` - Lookback period for %K
    /// * `period_d` - Smoothing period for %D
    /// * `ma_type` - Moving average type for %D calculation
    pub fn with_ma_type(period_k: usize, period_d: usize, ma_type: MovingAverageType) -> Self {
        assert!(period_k <= 512, "period_k must be <= 512");
        assert!(period_d <= 512, "period_d must be <= 512");
        Self {
            period_k,
            period_d,
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            closes: ArrayVec::new(),
            d_ma: MovingAverageProvider::new(ma_type, period_d),
            ma_type,
            index: 0,
            filled: false,
            value_k: 0.0,
            value_d: 0.0,
        }
    }

    /// Updates the Stochastic with a new bar and returns (%K, %D).
    ///
    /// Uses `high`, `low`, and `close` prices. Volume is ignored.
    pub fn update_bar(&mut self, high: f64, low: f64, close: f64, _volume: f64) -> (f64, f64) {
        if self.highs.len() < self.period_k {
            self.highs.push(high);
            self.lows.push(low);
            self.closes.push(close);
        } else {
            self.highs[self.index] = high;
            self.lows[self.index] = low;
            self.closes[self.index] = close;
        }

        self.index = (self.index + 1) % self.period_k;
        let ready_k = self.highs.len() == self.period_k;

        if !ready_k {
            self.value_k = 0.0;
            self.value_d = 0.0;
            return (self.value_k, self.value_d);
        }

        // Calculate %K
        let (k_min_low, k_max_high) = self.highs.iter()
            .zip(self.lows.iter())
            .fold((f64::INFINITY, f64::NEG_INFINITY),
                  |(min, max), (&h, &l)| (min.min(l), max.max(h)));

        if (k_max_high - k_min_low).abs() < 1e-12 {
            self.value_k = 0.0;
        } else {
            self.value_k = 100.0 * ((close - k_min_low) / (k_max_high - k_min_low));
        }

        // Calculate %D = MA(%K)
        let _d_value = self.d_ma.update_bar(0.0, 0.0, 0.0, self.value_k, 0.0);
        self.value_d = self.d_ma.value().main();

        if self.d_ma.is_ready() {
            self.filled = true;
        }

        (self.value_k, self.value_d)
    }

    /// Returns the current values as IndicatorValue.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.value_k, self.value_d)
    }

    /// Returns the current %K value.
    #[inline]
    pub fn value_k(&self) -> f64 {
        self.value_k
    }

    /// Returns the current %D value.
    #[inline]
    pub fn value_d(&self) -> f64 {
        self.value_d
    }

    /// Returns `true` if the indicator has enough data for valid output.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Resets the indicator to its initial state.
    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.closes.clear();
        self.d_ma.reset();
        self.index = 0;
        self.filled = false;
        self.value_k = 0.0;
        self.value_d = 0.0;
    }

    /// Returns the %D smoothing period.
    #[inline]
    pub fn get_period_d(&self) -> usize {
        self.period_d
    }

    /// Returns the MA type used for %D smoothing.
    #[inline]
    pub fn get_ma_type(&self) -> MovingAverageType {
        self.ma_type
    }

    /// Returns the %K lookback period.
    #[inline]
    pub fn get_period_k(&self) -> usize {
        self.period_k
    }

    /// Sets a new %D smoothing period (recreates the MA).
    pub fn set_period_d(&mut self, new_period: usize) {
        assert!(new_period <= 512, "period_d must be <= 512");
        self.period_d = new_period;
        self.d_ma = MovingAverageProvider::new(self.ma_type, self.period_d);
    }

    /// Sets a new MA type for %D smoothing (recreates the MA).
    pub fn set_ma_type(&mut self, new_ma_type: MovingAverageType) {
        self.ma_type = new_ma_type;
        self.d_ma = MovingAverageProvider::new(self.ma_type, self.period_d);
    }

    /// Returns the full indicator configuration.
    pub fn get_config(&self) -> StochasticsConfig {
        StochasticsConfig {
            period_k: self.period_k,
            period_d: self.period_d,
            ma_type: self.ma_type,
        }
    }

    /// Sets a new configuration (recreates MA if needed).
    pub fn set_config(&mut self, config: StochasticsConfig) {
        let ma_changed = config.period_d != self.period_d || config.ma_type != self.ma_type;

        self.period_k = config.period_k;
        self.period_d = config.period_d;
        self.ma_type = config.ma_type;

        if ma_changed {
            self.d_ma = MovingAverageProvider::new(self.ma_type, self.period_d);
        }
    }
}

/// Configuration for the Stochastic Oscillator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StochasticsConfig {
    /// %K lookback period
    pub period_k: usize,
    /// %D smoothing period
    pub period_d: usize,
    /// MA type for %D calculation
    pub ma_type: MovingAverageType,
}

impl std::fmt::Debug for Stochastics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stochastics")
            .field("period_k", &self.period_k)
            .field("period_d", &self.period_d)
            .field("ma_type", &self.ma_type)
            .field("value_k", &self.value_k)
            .field("value_d", &self.value_d)
            .field("filled", &self.filled)
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
    fn test_stochastics_basic_calculation() {
        let mut stoch = Stochastics::new(14, 3);

        // Feed uptrend data with increasing highs and lows
        for i in 1..=20 {
            let base = 100.0 + i as f64;
            stoch.update_bar(base + 2.0, base - 2.0, base, 0.0);
        }

        // In strong uptrend, %K should be high
        assert!(stoch.value_k() > 50.0, "Stoch %K in uptrend should be above 50, got {}", stoch.value_k());
    }

    #[test]
    fn test_stochastics_range() {
        let mut stoch = Stochastics::new(14, 3);

        // Feed data
        for i in 1..=20 {
            let base = 100.0 + (i % 10) as f64;
            stoch.update_bar(base + 5.0, base - 5.0, base, 0.0);
        }

        // %K and %D should be in [0, 100]
        let k = stoch.value_k();
        let d = stoch.value_d();
        assert!(k >= 0.0 && k <= 100.0, "%K should be in [0, 100], got {}", k);
        assert!(d >= 0.0 && d <= 100.0, "%D should be in [0, 100], got {}", d);
    }

    #[test]
    fn test_stochastics_with_ma_type() {
        let mut stoch = Stochastics::with_ma_type(14, 3, MovingAverageType::EMA);

        for i in 1..=20 {
            let base = 100.0 + i as f64;
            stoch.update_bar(base + 2.0, base - 2.0, base, 0.0);
        }

        assert_eq!(stoch.get_ma_type(), MovingAverageType::EMA);
    }

    #[test]
    fn test_stochastics_reset() {
        let mut stoch = Stochastics::new(14, 3);

        for i in 1..=20 {
            let base = 100.0 + i as f64;
            stoch.update_bar(base + 2.0, base - 2.0, base, 0.0);
        }

        stoch.reset();
        assert!(!stoch.is_ready());
        assert!((stoch.value_k()).abs() < 1e-10);
        assert!((stoch.value_d()).abs() < 1e-10);
    }

    #[test]
    fn test_stochastics_config() {
        let stoch = Stochastics::new(14, 3);
        let config = stoch.get_config();

        assert_eq!(config.period_k, 14);
        assert_eq!(config.period_d, 3);
        assert_eq!(config.ma_type, MovingAverageType::SMA);
    }

    #[test]
    fn test_stochastics_set_config() {
        let mut stoch = Stochastics::new(14, 3);

        let new_config = StochasticsConfig {
            period_k: 21,
            period_d: 5,
            ma_type: MovingAverageType::EMA,
        };
        stoch.set_config(new_config);

        assert_eq!(stoch.get_period_k(), 21);
        assert_eq!(stoch.get_period_d(), 5);
        assert_eq!(stoch.get_ma_type(), MovingAverageType::EMA);
    }

    #[test]
    fn test_stochastics_at_high() {
        let mut stoch = Stochastics::new(5, 3);

        // Fill with constant price, then close at the high
        for _ in 0..5 {
            stoch.update_bar(110.0, 90.0, 100.0, 0.0);
        }

        // Close at the highest high
        stoch.update_bar(110.0, 90.0, 110.0, 0.0);
        assert!((stoch.value_k() - 100.0).abs() < 1.0, "%K at high should be ~100, got {}", stoch.value_k());
    }

    #[test]
    fn test_stochastics_at_low() {
        let mut stoch = Stochastics::new(5, 3);

        // Fill with constant price, then close at the low
        for _ in 0..5 {
            stoch.update_bar(110.0, 90.0, 100.0, 0.0);
        }

        // Close at the lowest low
        stoch.update_bar(110.0, 90.0, 90.0, 0.0);
        assert!(stoch.value_k() < 1.0, "%K at low should be ~0, got {}", stoch.value_k());
    }
}
