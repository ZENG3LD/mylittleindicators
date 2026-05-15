//! FundingSentimentAlignment — detects alignment between funding rate sign and long/short positioning.
//!
//! Dual consumer: `FundingRateConsumer` + `LongShortRatioConsumer`.
//!
//! Logic:
//! - `funding > 0` (longs pay shorts) AND `long_ratio > 0.5` (crowd is long) → `+1`
//!   (longs over-positioned — bearish reversal signal)
//! - `funding < 0` (shorts pay longs) AND `long_ratio < 0.5` (crowd is short) → `-1`
//!   (shorts over-positioned — bullish reversal signal)
//! - Otherwise → `0`
//!
//! Output: `Signal(i8)`.

use crate::bar_indicators::funding_rate_consumer::FundingRateConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::long_short_ratio_consumer::LongShortRatioConsumer;
use crate::core::types::FundingRate;
use crate::core::types::LongShortRatio;

/// Alignment detector between funding rate and market positioning.
///
/// Implements both `FundingRateConsumer` and `LongShortRatioConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Clone)]
pub struct FundingSentimentAlignment {
    last_funding: f64,
    last_long_ratio: f64,
    last_signal: i8,
    funding_seen: bool,
    ratio_seen: bool,
}

impl FundingSentimentAlignment {
    /// Create a new indicator.
    pub fn new() -> Self {
        Self {
            last_funding: 0.0,
            last_long_ratio: 0.5,
            last_signal: 0,
            funding_seen: false,
            ratio_seen: false,
        }
    }

    fn recompute(&mut self) {
        self.last_signal = if self.last_funding > 0.0 && self.last_long_ratio > 0.5 {
            1
        } else if self.last_funding < 0.0 && self.last_long_ratio < 0.5 {
            -1
        } else {
            0
        };
    }

    /// Passthrough for bar pipeline — returns current value.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    /// True when both streams have delivered at least one update.
    pub fn indicator_is_ready(&self) -> bool {
        self.funding_seen && self.ratio_seen
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.last_funding = 0.0;
        self.last_long_ratio = 0.5;
        self.last_signal = 0;
        self.funding_seen = false;
        self.ratio_seen = false;
    }
}

impl Default for FundingSentimentAlignment {
    fn default() -> Self {
        Self::new()
    }
}

impl FundingRateConsumer for FundingSentimentAlignment {
    fn update_funding(&mut self, fr: &FundingRate) -> IndicatorValue {
        self.last_funding = fr.rate;
        self.funding_seen = true;
        self.recompute();
        self.indicator_value()
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
    }

    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl LongShortRatioConsumer for FundingSentimentAlignment {
    fn update_long_short_ratio(&mut self, lsr: &LongShortRatio) -> IndicatorValue {
        self.last_long_ratio = lsr.long_ratio;
        self.ratio_seen = true;
        self.recompute();
        self.indicator_value()
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
    }

    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate { symbol: "BTCUSDT".to_string(), rate, next_funding_time: None, timestamp: 1000 }
    }

    fn make_lsr(long_ratio: f64) -> LongShortRatio {
        LongShortRatio {
            symbol: String::new(),
            ratio_type: "global_account".to_string(),
            long_ratio,
            short_ratio: 1.0 - long_ratio,
            ratio: Some(long_ratio / (1.0 - long_ratio + 1e-9)),
            timestamp: 1000,
        }
    }

    #[test]
    fn positive_funding_majority_long_gives_plus_one() {
        let mut ind = FundingSentimentAlignment::new();
        ind.update_funding(&make_fr(0.001));
        ind.update_long_short_ratio(&make_lsr(0.65));
        if let IndicatorValue::Signal(s) = ind.indicator_value() {
            assert_eq!(s, 1);
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn negative_funding_majority_short_gives_minus_one() {
        let mut ind = FundingSentimentAlignment::new();
        ind.update_funding(&make_fr(-0.001));
        ind.update_long_short_ratio(&make_lsr(0.35));
        if let IndicatorValue::Signal(s) = ind.indicator_value() {
            assert_eq!(s, -1);
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn misaligned_gives_zero() {
        let mut ind = FundingSentimentAlignment::new();
        ind.update_funding(&make_fr(0.001));
        ind.update_long_short_ratio(&make_lsr(0.45)); // positive funding, fewer longs
        if let IndicatorValue::Signal(s) = ind.indicator_value() {
            assert_eq!(s, 0);
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn not_ready_before_both_streams() {
        let mut ind = FundingSentimentAlignment::new();
        ind.update_funding(&make_fr(0.001));
        assert!(!ind.indicator_is_ready());
        ind.update_long_short_ratio(&make_lsr(0.65));
        assert!(ind.indicator_is_ready());
    }
}
