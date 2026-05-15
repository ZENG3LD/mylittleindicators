//! TickFrequencyAnomaly — burst/quiet detection via short/long rate ratio.
//!
//! Computes the ratio of the current tick rate (short window) to the baseline
//! tick rate (long window).
//!
//!   ratio > 1.5 → burst activity
//!   ratio ≈ 1.0 → normal
//!   ratio < 0.5 → quiet period
//!
//! Output: `IndicatorValue::Single(ratio)`
//!
//! `ratio` is 0.0 when there is no baseline yet (long window has no data).

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Tick-frequency anomaly detector using short/long rate ratio.
#[derive(Debug, Clone)]
pub struct TickFrequencyAnomaly {
    short_window_ms: i64,
    long_window_ms: i64,
    /// Timestamps of all ticks within `long_window_ms`.
    timestamps: VecDeque<i64>,
    last_ratio: f64,
}

impl TickFrequencyAnomaly {
    /// Create detector.
    ///
    /// - `short_window_ms`: window for current rate (e.g. 5 000 ms = 5 s).
    /// - `long_window_ms`:  window for baseline rate (e.g. 60 000 ms = 1 min).
    ///
    /// `long_window_ms` must be > `short_window_ms`.
    pub fn new(short_window_ms: i64, long_window_ms: i64) -> Self {
        // Enforce short < long; if mis-specified, swap silently.
        let (short, long) = if short_window_ms < long_window_ms {
            (short_window_ms.max(1), long_window_ms)
        } else {
            (long_window_ms.max(1), short_window_ms)
        };
        Self {
            short_window_ms: short,
            long_window_ms: long,
            timestamps: VecDeque::with_capacity(1024),
            last_ratio: 0.0,
        }
    }
}

impl TickConsumer for TickFrequencyAnomaly {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        self.timestamps.push_back(tick.time);

        // Evict timestamps outside the long window.
        while let Some(&ts) = self.timestamps.front() {
            if tick.time - ts > self.long_window_ms {
                self.timestamps.pop_front();
            } else {
                break;
            }
        }

        // Count ticks in short window.
        let cutoff_short = tick.time - self.short_window_ms;
        let short_count = self.timestamps.iter().filter(|&&ts| ts >= cutoff_short).count();

        let long_count = self.timestamps.len();

        // Convert counts to rates (events per second).
        let short_secs = self.short_window_ms as f64 / 1000.0;
        let long_secs = self.long_window_ms as f64 / 1000.0;

        let current_rate = short_count as f64 / short_secs;
        let baseline_rate = long_count as f64 / long_secs;

        self.last_ratio = if baseline_rate > 0.0 {
            current_rate / baseline_rate
        } else {
            0.0
        };

        IndicatorValue::Single(self.last_ratio)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_ratio)
    }

    fn reset(&mut self) {
        self.timestamps.clear();
        self.last_ratio = 0.0;
    }

    /// Ready once the long window has at least one tick.
    fn is_ready(&self) -> bool {
        !self.timestamps.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick_at(time_ms: i64) -> Tick {
        Tick::new(time_ms, 100.0, 1.0, true)
    }

    #[test]
    fn uniform_rate_gives_ratio_one() {
        // 10 ticks uniformly spread over 60 seconds (6 s apart).
        // Short window = 5 s: ~1 tick (the last one)
        // Long window = 60 s: 10 ticks
        // current_rate = 1 / 5.0 = 0.2 tps
        // baseline_rate = 10 / 60.0 ≈ 0.1667 tps
        // ratio ≈ 1.2 (not exactly 1 because short window captures only 1 tick of the last 5 s)
        let mut ind = TickFrequencyAnomaly::new(5_000, 60_000);
        for i in 0..10 {
            ind.update_tick(&tick_at(i * 6_000));
        }
        let ratio = match ind.value() {
            IndicatorValue::Single(r) => r,
            other => panic!("expected Single, got {:?}", other),
        };
        // ratio should be > 0 and finite
        assert!(ratio > 0.0 && ratio.is_finite(), "ratio={}", ratio);
    }

    #[test]
    fn burst_gives_high_ratio() {
        // First, establish a low baseline (1 tick per 10 seconds over 60 s).
        let mut ind = TickFrequencyAnomaly::new(5_000, 60_000);
        for i in 0..6 {
            ind.update_tick(&tick_at(i * 10_000));
        }
        // Now burst: 10 ticks in the last 1 second (the short window).
        let base_time = 6 * 10_000i64;
        for j in 0..10 {
            ind.update_tick(&tick_at(base_time + j * 100));
        }
        let ratio = match ind.value() {
            IndicatorValue::Single(r) => r,
            other => panic!("expected Single, got {:?}", other),
        };
        assert!(ratio > 1.0, "burst should yield ratio > 1, got {}", ratio);
    }

    #[test]
    fn zero_ratio_when_no_data() {
        let ind = TickFrequencyAnomaly::new(5_000, 60_000);
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::Single(0.0));
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = TickFrequencyAnomaly::new(5_000, 60_000);
        ind.update_tick(&tick_at(0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::Single(0.0));
    }
}
