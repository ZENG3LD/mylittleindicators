//! Commodity Channel Index (CCI) indicator.

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Commodity Channel Index (CCI) - measures price deviation from statistical mean.
///
/// CCI = (Typical Price - MA) / (scalar × Mean Deviation)
///
/// where Typical Price = (High + Low + Close) / 3
/// and Mean Deviation = average of |TP - MA| over the period.
///
/// CCI typically oscillates between -100 and +100:
/// - Above +100: Overbought / strong uptrend
/// - Below -100: Oversold / strong downtrend
/// - Zero line crossings: Potential trend changes
///
/// # Parameters
/// - `period`: Lookback period (typically 20)
/// - `scalar`: Lambert's constant (typically 0.015)
/// - `ma_type`: Moving average type for smoothing
///
/// # Implementation
///
/// Uses configurable MA and ring buffer for mean deviation. O(period) per update.
/// Maximum period is 512.
#[derive(Clone)]
pub struct Cci {
    period: usize,
    scalar: f64,
    ma: MovingAverageProvider,
    typical_buf: ArrayVec<f64, 512>,
    idx: usize,
    count: usize,
    value: f64,
    filled: bool,
}

impl Cci {
    /// Creates a new CCI with the specified parameters.
    ///
    /// # Arguments
    /// * `period` - Lookback period (1..=512)
    /// * `scalar` - Lambert's constant (typically 0.015)
    /// * `ma_type` - Optional MA type (defaults to SMA)
    pub fn new(period: usize, scalar: f64, ma_type: Option<MovingAverageType>) -> Self {
        let ma_type = ma_type.unwrap_or(MovingAverageType::SMA);
        Self {
            period,
            scalar,
            ma: MovingAverageProvider::new(ma_type, period),
            typical_buf: ArrayVec::new(),
            idx: 0,
            count: 0,
            value: 0.0,
            filled: false,
        }
    }

    /// Updates the CCI with a new bar and returns the current value.
    ///
    /// Uses `high`, `low`, and `close` prices. Volume is ignored.
    pub fn update_bar(&mut self, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        let typical = (high + low + close) / 3.0;

        if self.count < self.period {
            self.typical_buf.push(typical);
            self.count += 1;
            self.idx = self.count % self.period;
        } else {
            self.typical_buf[self.idx] = typical;
            self.idx = (self.idx + 1) % self.period;
        }

        self.ma.update_bar(0.0, 0.0, 0.0, typical, 0.0);

        if self.count >= self.period {
            let mean = self.ma.value().main();
            let mad = self.typical_buf.iter().map(|&v| (v - mean).abs()).sum::<f64>() / self.period as f64;
            if mad.abs() < 1e-12 {
                self.value = 0.0;
            } else {
                self.value = (typical - mean) / (self.scalar * mad);
            }
            self.filled = true;
        } else {
            self.value = 0.0;
            self.filled = false;
        }
        self.value
    }

    /// Returns the current CCI value as an `IndicatorValue`.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the CCI has enough data to produce valid values.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Resets the CCI to its initial state.
    pub fn reset(&mut self) {
        self.typical_buf.clear();
        self.idx = 0;
        self.count = 0;
        self.value = 0.0;
        self.filled = false;
        self.ma.reset();
    }

    /// Returns the period of this CCI.
    #[inline]
    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_cci_basic_calculation() {
        let mut cci = Cci::new(20, 0.015, None);

        // Feed uptrend data
        for i in 1..=30 {
            let base = 100.0 + i as f64;
            cci.update_bar(base + 2.0, base - 2.0, base, 0.0);
        }

        assert!(cci.is_ready());
        // In strong uptrend, CCI should be positive
        assert!(cci.value().main() > 0.0, "CCI in uptrend should be positive");
    }

    #[test]
    fn test_cci_downtrend() {
        let mut cci = Cci::new(20, 0.015, None);

        // Feed downtrend data
        for i in 1..=30 {
            let base = 200.0 - i as f64;
            cci.update_bar(base + 2.0, base - 2.0, base, 0.0);
        }

        assert!(cci.is_ready());
        // In strong downtrend, CCI should be negative
        assert!(cci.value().main() < 0.0, "CCI in downtrend should be negative");
    }

    #[test]
    fn test_cci_constant_price() {
        let mut cci = Cci::new(20, 0.015, None);

        // Feed constant price data
        for _ in 1..=30 {
            cci.update_bar(102.0, 98.0, 100.0, 0.0);
        }

        assert!(cci.is_ready());
        // With constant typical price, CCI should be ~0
        assert!(cci.value().main().abs() < 1.0, "CCI with constant price should be ~0");
    }

    #[test]
    fn test_cci_with_ma_type() {
        let mut cci = Cci::new(20, 0.015, Some(MovingAverageType::EMA));

        for i in 1..=30 {
            let base = 100.0 + i as f64;
            cci.update_bar(base + 2.0, base - 2.0, base, 0.0);
        }

        assert!(cci.is_ready());
    }

    #[test]
    fn test_cci_reset() {
        let mut cci = Cci::new(20, 0.015, None);

        for i in 1..=30 {
            let base = 100.0 + i as f64;
            cci.update_bar(base + 2.0, base - 2.0, base, 0.0);
        }
        assert!(cci.is_ready());

        cci.reset();
        assert!(!cci.is_ready());
        assert!((cci.value().main()).abs() < 1e-10);
    }

    #[test]
    fn test_cci_period_getter() {
        let cci = Cci::new(20, 0.015, None);
        assert_eq!(cci.period(), 20);
    }

    #[test]
    fn test_cci_not_ready_before_period() {
        let mut cci = Cci::new(20, 0.015, None);

        for i in 1..=15 {
            let base = 100.0 + i as f64;
            cci.update_bar(base + 2.0, base - 2.0, base, 0.0);
        }

        assert!(!cci.is_ready());
    }
}
