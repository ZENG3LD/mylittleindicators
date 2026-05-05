//! Aroon indicator.

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Aroon indicator - measures time since highest high and lowest low.
///
/// Aroon Up = 100 × (period - bars since highest high) / period
/// Aroon Down = 100 × (period - bars since lowest low) / period
/// Aroon Oscillator = Aroon Up - Aroon Down
///
/// Both Aroon Up and Down oscillate between 0 and 100:
/// - Aroon Up > 70: Strong uptrend
/// - Aroon Down > 70: Strong downtrend
/// - Aroon Oscillator > 0: Bullish
/// - Aroon Oscillator < 0: Bearish
///
/// # Parameters
/// - `period`: Lookback period (typically 25)
///
/// # Implementation
///
/// Uses ring buffer to track recent highs and lows. O(period) per update.
/// Maximum period is 512.
#[derive(Debug, Clone)]
pub struct Aroon {
    period: usize,
    highs: ArrayVec<f64, 512>,
    lows: ArrayVec<f64, 512>,
    filled: bool,
    aroon_up: f64,
    aroon_down: f64,
    aroon_osc: f64,
}

impl Aroon {
    /// Creates a new Aroon indicator with the specified period.
    ///
    /// # Arguments
    /// * `period` - Lookback period (1..=512)
    pub fn new(period: usize) -> Self {
        Self {
            period,
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            filled: false,
            aroon_up: 0.0,
            aroon_down: 0.0,
            aroon_osc: 0.0,
        }
    }

    /// Updates the Aroon with a new bar and returns (Aroon Up, Aroon Down, Oscillator).
    ///
    /// Uses `high` and `low` prices. Other OHLCV fields are ignored.
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, _close: f64, _volume: f64) -> (f64, f64, f64) {
        if self.highs.len() == self.period {
            self.highs.pop();
        }
        if self.lows.len() == self.period {
            self.lows.pop();
        }
        self.highs.insert(0, high);
        self.lows.insert(0, low);

        if self.highs.len() == self.period && self.lows.len() == self.period {
            self.filled = true;
        }

        let len = self.highs.len().min(self.lows.len());
        if len < self.period {
            self.aroon_up = 0.0;
            self.aroon_down = 0.0;
            self.aroon_osc = 0.0;
            return (self.aroon_up, self.aroon_down, self.aroon_osc);
        }

        // Find index of highest high and lowest low (0 = most recent)
        let (mut max_idx, mut max_val) = (0, self.highs[0]);
        let (mut min_idx, mut min_val) = (0, self.lows[0]);

        for (i, &v) in self.highs.iter().enumerate() {
            if v > max_val {
                max_val = v;
                max_idx = i;
            }
        }
        for (i, &v) in self.lows.iter().enumerate() {
            if v < min_val {
                min_val = v;
                min_idx = i;
            }
        }

        self.aroon_up = 100.0 * (self.period - max_idx) as f64 / self.period as f64;
        self.aroon_down = 100.0 * (self.period - min_idx) as f64 / self.period as f64;
        self.aroon_osc = self.aroon_up - self.aroon_down;

        (self.aroon_up, self.aroon_down, self.aroon_osc)
    }

    /// Returns (Aroon Up, Aroon Down, Oscillator).
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.aroon_up, self.aroon_down, self.aroon_osc)
    }

    /// Returns `true` if the indicator has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Resets the indicator to its initial state.
    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.filled = false;
        self.aroon_up = 0.0;
        self.aroon_down = 0.0;
        self.aroon_osc = 0.0;
    }

    /// Returns the period of this Aroon.
    #[inline]
    pub fn period(&self) -> usize {
        self.period
    }

    /// Returns Aroon Up value.
    #[inline]
    pub fn aroon_up(&self) -> f64 {
        self.aroon_up
    }

    /// Returns Aroon Down value.
    #[inline]
    pub fn aroon_down(&self) -> f64 {
        self.aroon_down
    }

    /// Returns Aroon Oscillator value.
    #[inline]
    pub fn oscillator(&self) -> f64 {
        self.aroon_osc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_aroon_basic_calculation() {
        let mut aroon = Aroon::new(25);

        // Feed uptrend data - new highs being made
        for i in 1..=30 {
            let base = 100.0 + i as f64;
            aroon.update_bar(0.0, base + 2.0, base - 2.0, base, 0.0);
        }

        assert!(aroon.is_ready());
        // In uptrend with new highs, Aroon Up should be high
        assert!(aroon.aroon_up() > 80.0, "Aroon Up in uptrend should be > 80, got {}", aroon.aroon_up());
    }

    #[test]
    fn test_aroon_downtrend() {
        let mut aroon = Aroon::new(25);

        // Feed downtrend data - new lows being made
        for i in 1..=30 {
            let base = 200.0 - i as f64;
            aroon.update_bar(0.0, base + 2.0, base - 2.0, base, 0.0);
        }

        assert!(aroon.is_ready());
        // In downtrend with new lows, Aroon Down should be high
        assert!(aroon.aroon_down() > 80.0, "Aroon Down in downtrend should be > 80, got {}", aroon.aroon_down());
    }

    #[test]
    fn test_aroon_range() {
        let mut aroon = Aroon::new(25);

        for i in 1..=30 {
            let base = 100.0 + (i % 10) as f64;
            aroon.update_bar(0.0, base + 3.0, base - 3.0, base, 0.0);
        }

        assert!(aroon.is_ready());
        if let IndicatorValue::Triple(up, down, osc) = aroon.value() {
            assert!(up >= 0.0 && up <= 100.0, "Aroon Up should be in [0, 100]");
            assert!(down >= 0.0 && down <= 100.0, "Aroon Down should be in [0, 100]");
            assert!(osc >= -100.0 && osc <= 100.0, "Oscillator should be in [-100, 100]");
        } else { panic!("Expected Triple"); }
    }

    #[test]
    fn test_aroon_oscillator() {
        let mut aroon = Aroon::new(25);

        // Strong uptrend
        for i in 1..=30 {
            let base = 100.0 + i as f64;
            aroon.update_bar(0.0, base + 2.0, base - 2.0, base, 0.0);
        }

        assert!(aroon.is_ready());
        // Oscillator should be positive in uptrend
        assert!(aroon.oscillator() > 0.0, "Oscillator in uptrend should be positive");
    }

    #[test]
    fn test_aroon_reset() {
        let mut aroon = Aroon::new(25);

        for i in 1..=30 {
            let base = 100.0 + i as f64;
            aroon.update_bar(0.0, base + 2.0, base - 2.0, base, 0.0);
        }
        assert!(aroon.is_ready());

        aroon.reset();
        assert!(!aroon.is_ready());
        assert!((aroon.aroon_up()).abs() < 1e-10);
        assert!((aroon.aroon_down()).abs() < 1e-10);
    }

    #[test]
    fn test_aroon_period_getter() {
        let aroon = Aroon::new(25);
        assert_eq!(aroon.period(), 25);
    }

    #[test]
    fn test_aroon_at_high() {
        let mut aroon = Aroon::new(10);

        // Fill with same prices, then make new high at end
        for _ in 0..9 {
            aroon.update_bar(0.0, 100.0, 90.0, 95.0, 0.0);
        }
        // New high on last bar
        aroon.update_bar(0.0, 120.0, 90.0, 110.0, 0.0);

        assert!(aroon.is_ready());
        // Aroon Up should be 100 (high just made)
        assert!((aroon.aroon_up() - 100.0).abs() < 1.0, "Aroon Up at new high should be 100, got {}", aroon.aroon_up());
    }

    #[test]
    fn test_aroon_at_low() {
        let mut aroon = Aroon::new(10);

        // Fill with same prices, then make new low at end
        for _ in 0..9 {
            aroon.update_bar(0.0, 100.0, 90.0, 95.0, 0.0);
        }
        // New low on last bar
        aroon.update_bar(0.0, 100.0, 70.0, 85.0, 0.0);

        assert!(aroon.is_ready());
        // Aroon Down should be 100 (low just made)
        assert!((aroon.aroon_down() - 100.0).abs() < 1.0, "Aroon Down at new low should be 100, got {}", aroon.aroon_down());
    }
}
