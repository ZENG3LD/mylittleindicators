//! FundingTimeDecay — pressure score approaching the next funding settlement.
//!
//! Receives `PredictedFunding` updates to learn the next funding time.
//! On each `update_bar_with_ts` call the bar timestamp is used to compute
//! how close the current time is to the next funding settlement.
//!
//! ## Honesty Check
//!
//! `update_bar` in `IndicatorInstance` exposes `timestamp: Option<i64>`.
//! When the timestamp is zero or None, a tick-based proxy is used:
//! counts `update_bar` calls since the last `PredictedFunding` event and
//! normalizes against `max_window_ticks`.
//!
//! Output: `Single(decay_pressure)` ∈ [0, 1].
//! - `0.0` = far from funding (or no funding event seen)
//! - `1.0` = at or past funding time

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::predicted_funding_consumer::PredictedFundingConsumer;
use crate::core::types::PredictedFunding;

/// Funding time decay pressure score.
///
/// Pressure grows from 0 to 1 as the next funding settlement approaches.
#[derive(Clone)]
pub struct FundingTimeDecay {
    /// Maximum lookahead window in milliseconds.
    max_window_ms: i64,
    /// Maximum lookahead in ticks for the fallback.
    max_window_ticks: usize,
    /// Next funding time in milliseconds (from last PredictedFunding event).
    next_funding_ms: Option<i64>,
    /// Ticks since the last PredictedFunding event (tick-based fallback).
    ticks_since_funding: usize,
    last_pressure: f64,
}

impl FundingTimeDecay {
    /// Create a new indicator.
    ///
    /// - `max_window_ms`: pressure is 0 when `countdown >= max_window_ms` and 1 at 0.
    /// - `max_window_ticks`: fallback window in ticks when timestamps unavailable.
    pub fn new(max_window_ms: i64, max_window_ticks: usize) -> Self {
        Self {
            max_window_ms: max_window_ms.max(1),
            max_window_ticks: max_window_ticks.max(1),
            next_funding_ms: None,
            ticks_since_funding: 0,
            last_pressure: 0.0,
        }
    }

    /// Update pressure given current bar timestamp (ms, 0 if unknown).
    pub fn update_bar_with_ts(&mut self, ts_ms: i64) -> IndicatorValue {
        self.ticks_since_funding += 1;
        match self.next_funding_ms {
            Some(funding_ts) if ts_ms > 0 => {
                let countdown = (funding_ts - ts_ms).max(0);
                let normalized = countdown as f64 / self.max_window_ms as f64;
                self.last_pressure = 1.0 - normalized.clamp(0.0, 1.0);
            }
            _ => {
                // Tick-based fallback
                let ratio = self.ticks_since_funding as f64 / self.max_window_ticks as f64;
                self.last_pressure = ratio.clamp(0.0, 1.0);
            }
        }
        IndicatorValue::Single(self.last_pressure)
    }
}

impl Default for FundingTimeDecay {
    fn default() -> Self {
        // 8 hours default (Binance/Bybit cycle); 480 ticks fallback
        Self::new(8 * 3600 * 1000, 480)
    }
}

impl PredictedFundingConsumer for FundingTimeDecay {
    fn update_predicted_funding(&mut self, pf: &PredictedFunding) -> IndicatorValue {
        self.next_funding_ms = Some(pf.next_funding_time);
        self.ticks_since_funding = 0;
        // Recompute if event timestamp is valid
        if pf.timestamp > 0 {
            let countdown = (pf.next_funding_time - pf.timestamp).max(0);
            let normalized = countdown as f64 / self.max_window_ms as f64;
            self.last_pressure = 1.0 - normalized.clamp(0.0, 1.0);
        }
        IndicatorValue::Single(self.last_pressure)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_pressure)
    }

    fn reset(&mut self) {
        self.next_funding_ms = None;
        self.ticks_since_funding = 0;
        self.last_pressure = 0.0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_predicted(next_funding_time: i64, ts: i64) -> PredictedFunding {
        PredictedFunding {
            predicted_rate: 0.0001,
            next_funding_time,
            timestamp: ts,
        }
    }

    #[test]
    fn far_from_funding_gives_low_pressure() {
        let mut ind = FundingTimeDecay::new(8 * 3600 * 1000, 480);
        let next_ts = 8 * 3600 * 1000_i64;
        ind.update_predicted_funding(&make_predicted(next_ts, 0));
        let p = ind.update_bar_with_ts(0).main();
        // tick-based: 1 tick / 480 → very low
        assert!(p < 0.1, "pressure should be low far from funding, got {p}");
    }

    #[test]
    fn near_funding_gives_high_pressure() {
        let mut ind = FundingTimeDecay::new(8 * 3600 * 1000, 480);
        let next_ts = 50_i64; // 50ms ahead
        ind.update_predicted_funding(&make_predicted(next_ts, 0));
        let p = ind.update_bar_with_ts(next_ts - 10).main();
        assert!(p > 0.9, "pressure should be high near funding, got {p}");
    }

    #[test]
    fn tick_fallback_saturates() {
        let max_ticks = 5;
        let mut ind = FundingTimeDecay::new(8 * 3600 * 1000, max_ticks);
        for _ in 0..max_ticks {
            ind.update_bar_with_ts(0);
        }
        let p = ind.value().main();
        assert!(p >= 1.0, "tick fallback should saturate at 1.0, got {p}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = FundingTimeDecay::new(8 * 3600 * 1000, 480);
        ind.update_predicted_funding(&make_predicted(1_000_000, 0));
        ind.reset();
        assert_eq!(ind.value().main(), 0.0);
    }
}
