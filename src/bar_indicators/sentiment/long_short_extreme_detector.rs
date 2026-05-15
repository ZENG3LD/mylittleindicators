//! LongShortExtremeDetector — flags extreme positioning as potential reversal signal.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::LongShortRatioConsumer;
use crate::core::types::LongShortRatio;

/// Emits a contrarian signal when `long_ratio` reaches an extreme.
///
/// - `long_ratio > upper` → `Signal(1)` — crowd is extremely long; bearish reversal expected.
/// - `long_ratio < lower` → `Signal(-1)` — crowd is extremely short; bullish reversal expected.
/// - Otherwise → `Signal(0)`.
///
/// Output: `IndicatorValue::Signal(i8)`.
#[derive(Clone)]
pub struct LongShortExtremeDetector {
    upper: f64,
    lower: f64,
    last_signal: i8,
}

impl LongShortExtremeDetector {
    /// Create a new detector. Typical defaults: `upper = 0.8`, `lower = 0.2`.
    pub fn new(upper: f64, lower: f64) -> Self {
        Self {
            upper,
            lower,
            last_signal: 0,
        }
    }

    /// Passthrough for bar events — returns last signal unchanged.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }
}

impl LongShortRatioConsumer for LongShortExtremeDetector {
    fn update_long_short_ratio(&mut self, lsr: &LongShortRatio) -> IndicatorValue {
        self.last_signal = if lsr.long_ratio > self.upper {
            1
        } else if lsr.long_ratio < self.lower {
            -1
        } else {
            0
        };
        IndicatorValue::Signal(self.last_signal)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    fn reset(&mut self) {
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_lsr(long_ratio: f64) -> LongShortRatio {
        LongShortRatio {
            ratio_type: "global_account".to_string(),
            long_ratio,
            short_ratio: 1.0 - long_ratio,
            ratio: None,
            timestamp: 0,
        }
    }

    #[test]
    fn extreme_long_gives_signal_one() {
        let mut ind = LongShortExtremeDetector::new(0.8, 0.2);
        let v = ind.update_long_short_ratio(&make_lsr(0.85));
        assert_eq!(v, IndicatorValue::Signal(1));
    }

    #[test]
    fn extreme_short_gives_signal_minus_one() {
        let mut ind = LongShortExtremeDetector::new(0.8, 0.2);
        let v = ind.update_long_short_ratio(&make_lsr(0.15));
        assert_eq!(v, IndicatorValue::Signal(-1));
    }

    #[test]
    fn neutral_gives_signal_zero() {
        let mut ind = LongShortExtremeDetector::new(0.8, 0.2);
        let v = ind.update_long_short_ratio(&make_lsr(0.5));
        assert_eq!(v, IndicatorValue::Signal(0));
    }

    #[test]
    fn is_ready_immediately() {
        let ind = LongShortExtremeDetector::new(0.8, 0.2);
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_signal() {
        let mut ind = LongShortExtremeDetector::new(0.8, 0.2);
        ind.update_long_short_ratio(&make_lsr(0.9));
        ind.reset();
        assert_eq!(ind.value(), IndicatorValue::Signal(0));
    }
}
