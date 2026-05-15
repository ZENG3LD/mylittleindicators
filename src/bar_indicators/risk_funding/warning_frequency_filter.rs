//! WarningFrequencyFilter — debounce filter for market warning events.
//!
//! Emits a signal only when:
//! - warning_kind differs from the previous emitted warning, OR
//! - enough time has passed since last emission (> min_interval_ms)

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::market_warning_consumer::MarketWarningConsumer;
use crate::core::types::MarketWarning;

/// Debounce filter that suppresses repeated identical market warnings.
///
/// Returns `Signal(+1)` when a warning passes the filter,
/// `Signal(0)` when it is suppressed.
///
/// A warning passes if:
/// - Its `warning_kind` differs from the previous emitted warning, OR
/// - The elapsed time since last emission exceeds `min_interval_ms`.
///
/// Output: `Signal(i8)`: `+1` = emitted, `0` = filtered.
#[derive(Clone)]
pub struct WarningFrequencyFilter {
    min_interval_ms: i64,
    last_kind: Option<String>,
    last_emission_ts: Option<i64>,
    last_signal: i8,
}

impl WarningFrequencyFilter {
    /// Create a new filter.
    ///
    /// - `min_interval_ms`: minimum milliseconds between emissions of the same warning kind.
    pub fn new(min_interval_ms: i64) -> Self {
        Self {
            min_interval_ms: min_interval_ms.max(0),
            last_kind: None,
            last_emission_ts: None,
            last_signal: 0,
        }
    }

    /// Called by `update_bar` passthrough — returns last signal.
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, _close: f64, _volume: f64) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    fn should_emit(&self, warning: &MarketWarning) -> bool {
        match (&self.last_kind, self.last_emission_ts) {
            (None, _) => true,
            (Some(prev_kind), _) if prev_kind != &warning.warning_kind => true,
            (Some(_), Some(last_ts)) => {
                warning.timestamp - last_ts > self.min_interval_ms
            }
            (Some(_), None) => true,
        }
    }
}

impl Default for WarningFrequencyFilter {
    fn default() -> Self {
        Self::new(60_000)
    }
}

impl MarketWarningConsumer for WarningFrequencyFilter {
    fn update_market_warning(&mut self, w: &MarketWarning) -> IndicatorValue {
        if self.should_emit(w) {
            self.last_kind = Some(w.warning_kind.clone());
            self.last_emission_ts = Some(w.timestamp);
            self.last_signal = 1;
        } else {
            self.last_signal = 0;
        }
        IndicatorValue::Signal(self.last_signal)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    fn reset(&mut self) {
        self.last_kind = None;
        self.last_emission_ts = None;
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_warning(kind: &str, timestamp: i64) -> MarketWarning {
        MarketWarning {
            symbol: "BTCUSDT".to_string(),
            warning_kind: kind.to_string(),
            message: "test".to_string(),
            timestamp,
        }
    }

    #[test]
    fn first_warning_always_emits() {
        let mut f = WarningFrequencyFilter::new(60_000);
        let val = f.update_market_warning(&make_warning("volatility", 0));
        assert_eq!(val, IndicatorValue::Signal(1));
    }

    #[test]
    fn same_kind_within_interval_suppressed() {
        let mut f = WarningFrequencyFilter::new(60_000);
        f.update_market_warning(&make_warning("volatility", 0));
        let val = f.update_market_warning(&make_warning("volatility", 1_000));
        assert_eq!(val, IndicatorValue::Signal(0), "same kind too soon should be filtered");
    }

    #[test]
    fn different_kind_passes_immediately() {
        let mut f = WarningFrequencyFilter::new(60_000);
        f.update_market_warning(&make_warning("volatility", 0));
        let val = f.update_market_warning(&make_warning("margin_call", 1_000));
        assert_eq!(val, IndicatorValue::Signal(1), "different kind should pass");
    }

    #[test]
    fn same_kind_after_interval_passes() {
        let mut f = WarningFrequencyFilter::new(60_000);
        f.update_market_warning(&make_warning("volatility", 0));
        let val = f.update_market_warning(&make_warning("volatility", 60_001));
        assert_eq!(val, IndicatorValue::Signal(1), "same kind after interval should pass");
    }

    #[test]
    fn reset_clears_state() {
        let mut f = WarningFrequencyFilter::new(60_000);
        f.update_market_warning(&make_warning("volatility", 0));
        f.reset();
        let val = f.update_market_warning(&make_warning("volatility", 1_000));
        assert_eq!(val, IndicatorValue::Signal(1), "after reset, first update should emit");
    }
}
