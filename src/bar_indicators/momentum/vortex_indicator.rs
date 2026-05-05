//! Vortex Indicator (VI).

use arrayvec::ArrayVec;
use crate::bar_indicators::volatility::atr::Atr;
use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Vortex Indicator (VI) - trend identification indicator by Ethan Baumand and Douglas Siepman.
///
/// VI+ = Sum(|High - Previous Low|) / Sum(True Range) over N periods
/// VI- = Sum(|Low - Previous High|) / Sum(True Range) over N periods
///
/// Captures positive and negative trend movement. The indicator identifies
/// the start of new trends and helps confirm existing ones.
///
/// Interpretation:
/// - VI+ > VI-: Uptrend
/// - VI- > VI+: Downtrend
/// - VI+ crossing above VI-: Bullish signal
/// - VI- crossing above VI+: Bearish signal
/// - Values above 1.0: Strong trend
///
/// # Parameters
/// - `period`: Lookback period (typically 14)
///
/// # Implementation
///
/// Uses True Range for normalization. O(1) per update with rolling sums.
#[derive(Clone)]
pub struct VortexIndicator {
    period: usize,

    vi_plus_values: ArrayVec<f64, 512>,
    vi_minus_values: ArrayVec<f64, 512>,

    positive_vortex_sum: f64,
    negative_vortex_sum: f64,
    true_range_sum: f64,

    positive_vortex_buffer: ArrayVec<f64, 512>,
    negative_vortex_buffer: ArrayVec<f64, 512>,
    true_range_buffer: ArrayVec<f64, 512>,

    atr: Atr,

    prev_high: f64,
    prev_low: f64,

    vi_plus: f64,
    vi_minus: f64,

    bars_count: usize,
    is_ready: bool,
}

impl VortexIndicator {
    /// Creates a new Vortex Indicator with default period (14).
    pub fn new() -> Self {
        Self::with_period(14)
    }

    /// Creates a new Vortex Indicator with a custom period.
    ///
    /// # Arguments
    /// * `period` - Lookback period (typically 14)
    pub fn with_period(period: usize) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        
        Self {
            period,
            vi_plus_values: ArrayVec::new(),
            vi_minus_values: ArrayVec::new(),
            positive_vortex_sum: 0.0,
            negative_vortex_sum: 0.0,
            true_range_sum: 0.0,
            positive_vortex_buffer: ArrayVec::new(),
            negative_vortex_buffer: ArrayVec::new(),
            true_range_buffer: ArrayVec::new(),
            atr: Atr::new(period, MovingAverageType::RMA),
            prev_high: 0.0,
            prev_low: 0.0,
            vi_plus: 1.0,
            vi_minus: 1.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Updates the Vortex Indicator with a new bar and returns (VI+, VI-).
    ///
    /// Uses `high`, `low`, and `close` prices.
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> (f64, f64) {
        self.bars_count += 1;
        
        if self.bars_count == 1 {
            // First bar - initialize previous values
            self.prev_high = high;
            self.prev_low = low;
            return (self.vi_plus, self.vi_minus);
        }

        // Update ATR for True Range
        let true_range = self.atr.update_bar(_open, high, low, close, _volume);

        // Calculate positive and negative vortex movement
        let positive_vortex = (high - self.prev_low).abs();
        let negative_vortex = (low - self.prev_high).abs();

        // Add to buffers
        if self.positive_vortex_buffer.len() >= self.period {
            let old_positive = self.positive_vortex_buffer.remove(0);
            let old_negative = self.negative_vortex_buffer.remove(0);
            let old_tr = self.true_range_buffer.remove(0);
            
            self.positive_vortex_sum -= old_positive;
            self.negative_vortex_sum -= old_negative;
            self.true_range_sum -= old_tr;
        }
        
        self.positive_vortex_buffer.push(positive_vortex);
        self.negative_vortex_buffer.push(negative_vortex);
        self.true_range_buffer.push(true_range);
        
        self.positive_vortex_sum += positive_vortex;
        self.negative_vortex_sum += negative_vortex;
        self.true_range_sum += true_range;

        // Calculate VI+ and VI-
        if self.true_range_sum.abs() > 1e-12 {
            self.vi_plus = self.positive_vortex_sum / self.true_range_sum;
            self.vi_minus = self.negative_vortex_sum / self.true_range_sum;
        }

        // Add to value buffers
        if self.vi_plus_values.len() >= 512 {
            self.vi_plus_values.remove(0);
        }
        if self.vi_minus_values.len() >= 512 {
            self.vi_minus_values.remove(0);
        }
        
        self.vi_plus_values.push(self.vi_plus);
        self.vi_minus_values.push(self.vi_minus);

        // Update previous values
        self.prev_high = high;
        self.prev_low = low;

        // Check readiness
        if self.bars_count > self.period {
            self.is_ready = true;
        }

        (self.vi_plus, self.vi_minus)
    }

    /// Returns the VI+ value.
    #[inline]
    pub fn vi_plus(&self) -> f64 {
        self.vi_plus
    }

    /// Returns the VI- value.
    #[inline]
    pub fn vi_minus(&self) -> f64 {
        self.vi_minus
    }

    /// Returns both (VI+, VI-) values as tuple.
    #[inline]
    pub fn values(&self) -> (f64, f64) {
        (self.vi_plus, self.vi_minus)
    }

    /// Returns both (VI+, VI-) values as IndicatorValue::Double.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.vi_plus, self.vi_minus)
    }

    /// Returns `true` if the indicator has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Returns the period.
    #[inline]
    pub fn period(&self) -> usize {
        self.period
    }

    /// Resets the indicator to its initial state.
    pub fn reset(&mut self) {
        self.vi_plus_values.clear();
        self.vi_minus_values.clear();
        self.positive_vortex_sum = 0.0;
        self.negative_vortex_sum = 0.0;
        self.true_range_sum = 0.0;
        self.positive_vortex_buffer.clear();
        self.negative_vortex_buffer.clear();
        self.true_range_buffer.clear();
        self.atr.reset();
        self.prev_high = 0.0;
        self.prev_low = 0.0;
        self.vi_plus = 1.0;
        self.vi_minus = 1.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    /// Returns the current trend condition.
    pub fn trend_condition(&self) -> &'static str {
        if self.vi_plus > self.vi_minus && self.vi_plus > 1.0 {
            "Strong Uptrend"
        } else if self.vi_plus > self.vi_minus {
            "Uptrend"
        } else if self.vi_minus > self.vi_plus && self.vi_minus > 1.0 {
            "Strong Downtrend"
        } else if self.vi_minus > self.vi_plus {
            "Downtrend"
        } else {
            "Sideways"
        }
    }

    /// Returns trading signal (1 = buy, -1 = sell, 0 = neutral).
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        // Signals based on line crossover and 1.0 level
        if self.vi_plus > self.vi_minus && self.vi_plus > 1.0 {
            1  // Buy - VI+ above VI- and above 1.0
        } else if self.vi_minus > self.vi_plus && self.vi_minus > 1.0 {
            -1 // Sell - VI- above VI+ and above 1.0
        } else {
            0
        }
    }

    /// Returns advanced signal with crossover confirmation.
    pub fn advanced_signal(&self) -> i8 {
        if !self.is_ready() || self.vi_plus_values.len() < 3 || self.vi_minus_values.len() < 3 {
            return 0;
        }
        
        let len = self.vi_plus_values.len();
        let current_plus = self.vi_plus;
        let current_minus = self.vi_minus;
        let prev_plus = if len >= 2 { self.vi_plus_values[len - 2] } else { 1.0 };
        let prev_minus = if len >= 2 { self.vi_minus_values[len - 2] } else { 1.0 };

        // Buy signal: VI+ crosses above VI- and VI+ > 1.0
        if prev_plus <= prev_minus && current_plus > current_minus && current_plus > 1.0 {
            return 1;
        }

        // Sell signal: VI- crosses above VI+ and VI- > 1.0
        if prev_minus <= prev_plus && current_minus > current_plus && current_minus > 1.0 {
            return -1;
        }

        0
    }

    /// Returns trend strength (absolute difference between VI+ and VI-).
    pub fn trend_strength(&self) -> f64 {
        if !self.is_ready() {
            return 0.0;
        }
        
        (self.vi_plus - self.vi_minus).abs()
    }

    /// Returns trend direction (1 = up, -1 = down, 0 = sideways).
    pub fn trend_direction(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        if self.vi_plus > self.vi_minus {
            1
        } else if self.vi_minus > self.vi_plus {
            -1
        } else {
            0
        }
    }

    /// Checks for new trend start signal.
    ///
    /// Returns: 1 = uptrend starting, -1 = downtrend starting, 0 = no clear signal.
    pub fn trend_start_signal(&self) -> i8 {
        if !self.is_ready() || self.vi_plus_values.len() < 5 || self.vi_minus_values.len() < 5 {
            return 0;
        }
        
        let len = self.vi_plus_values.len();

        // Check last 3 bars for crossover and strengthening
        let mut bullish_count = 0;
        let mut bearish_count = 0;

        for i in (len - 3)..len {
            let plus = self.vi_plus_values[i];
            let minus = self.vi_minus_values[i];
            
            if plus > minus && plus > 1.0 {
                bullish_count += 1;
            } else if minus > plus && minus > 1.0 {
                bearish_count += 1;
            }
        }
        
        if bullish_count >= 2 {
            1
        } else if bearish_count >= 2 {
            -1
        } else {
            0
        }
    }

    /// Returns average VI values over specified periods.
    pub fn average_vi(&self, periods: usize) -> (f64, f64) {
        if !self.is_ready() || self.vi_plus_values.len() < periods || self.vi_minus_values.len() < periods {
            return (1.0, 1.0);
        }
        
        let start_idx = self.vi_plus_values.len() - periods;
        
        let avg_plus: f64 = self.vi_plus_values[start_idx..].iter().sum::<f64>() / periods as f64;
        let avg_minus: f64 = self.vi_minus_values[start_idx..].iter().sum::<f64>() / periods as f64;
        
        (avg_plus, avg_minus)
    }

    /// Returns information about the indicator state.
    pub fn info(&self) -> String {
        format!(
            "VI+: {:.3}, VI-: {:.3}, Trend: {}, Strength: {:.3}, Direction: {}",
            self.vi_plus,
            self.vi_minus,
            self.trend_condition(),
            self.trend_strength(),
            match self.trend_direction() {
                1 => "Up",
                -1 => "Down",
                _ => "Sideways"
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_vortex_basic_calculation() {
        let mut vi = VortexIndicator::new();

        // Feed uptrend data
        for i in 1..=20 {
            let base = 100.0 + i as f64;
            vi.update_bar(base, base + 1.0, base - 0.5, base + 0.5, 0.0);
        }

        assert!(vi.is_ready());
        let (vi_plus, vi_minus) = vi.values();
        assert!(vi_plus > 0.0 && vi_plus.is_finite());
        assert!(vi_minus > 0.0 && vi_minus.is_finite());
    }

    #[test]
    fn test_vortex_uptrend() {
        let mut vi = VortexIndicator::new();

        // Strong uptrend
        for i in 1..=30 {
            let base = 100.0 + i as f64 * 2.0;
            vi.update_bar(base, base + 2.0, base - 0.5, base + 1.5, 0.0);
        }

        assert!(vi.is_ready());
        // In strong uptrend, VI+ should be greater than VI-
        assert!(vi.vi_plus() > vi.vi_minus(), "VI+ should be greater than VI- in uptrend");
    }

    #[test]
    fn test_vortex_downtrend() {
        let mut vi = VortexIndicator::new();

        // Strong downtrend
        for i in 1..=30 {
            let base = 200.0 - i as f64 * 2.0;
            vi.update_bar(base, base + 0.5, base - 2.0, base - 1.5, 0.0);
        }

        assert!(vi.is_ready());
        // In strong downtrend, VI- should be greater than VI+
        assert!(vi.vi_minus() > vi.vi_plus(), "VI- should be greater than VI+ in downtrend");
    }

    #[test]
    fn test_vortex_reset() {
        let mut vi = VortexIndicator::new();

        for i in 1..=20 {
            let base = 100.0 + i as f64;
            vi.update_bar(base, base + 1.0, base - 0.5, base, 0.0);
        }
        assert!(vi.is_ready());

        vi.reset();
        assert!(!vi.is_ready());
        assert_eq!(vi.vi_plus(), 1.0);
        assert_eq!(vi.vi_minus(), 1.0);
    }

    #[test]
    fn test_vortex_period() {
        let vi = VortexIndicator::with_period(21);
        assert_eq!(vi.period(), 21);
    }

    #[test]
    fn test_vortex_trend_condition() {
        let mut vi = VortexIndicator::new();

        for i in 1..=30 {
            let base = 100.0 + i as f64;
            vi.update_bar(base, base + 1.0, base - 0.5, base, 0.0);
        }

        assert!(vi.is_ready());
        let condition = vi.trend_condition();
        assert!(
            condition == "Strong Uptrend"
                || condition == "Uptrend"
                || condition == "Strong Downtrend"
                || condition == "Downtrend"
                || condition == "Sideways"
        );
    }

    #[test]
    fn test_vortex_trading_signal() {
        let mut vi = VortexIndicator::new();

        for i in 1..=30 {
            let base = 100.0 + i as f64;
            vi.update_bar(base, base + 1.0, base - 0.5, base, 0.0);
        }

        assert!(vi.is_ready());
        let signal = vi.trading_signal();
        assert!(signal >= -1 && signal <= 1);
    }

    #[test]
    fn test_vortex_trend_strength() {
        let mut vi = VortexIndicator::new();

        for i in 1..=30 {
            let base = 100.0 + i as f64;
            vi.update_bar(base, base + 1.0, base - 0.5, base, 0.0);
        }

        assert!(vi.is_ready());
        let strength = vi.trend_strength();
        assert!(strength >= 0.0);
    }

    #[test]
    fn test_vortex_trend_direction() {
        let mut vi = VortexIndicator::new();

        // Uptrend
        for i in 1..=30 {
            let base = 100.0 + i as f64 * 2.0;
            vi.update_bar(base, base + 2.0, base - 0.5, base + 1.5, 0.0);
        }

        assert!(vi.is_ready());
        let direction = vi.trend_direction();
        assert_eq!(direction, 1, "Should indicate uptrend");
    }

    #[test]
    fn test_vortex_average_vi() {
        let mut vi = VortexIndicator::new();

        for i in 1..=30 {
            let base = 100.0 + i as f64;
            vi.update_bar(base, base + 1.0, base - 0.5, base, 0.0);
        }

        assert!(vi.is_ready());
        let (avg_plus, avg_minus) = vi.average_vi(5);
        assert!(avg_plus.is_finite() && avg_plus > 0.0);
        assert!(avg_minus.is_finite() && avg_minus > 0.0);
    }
}



















