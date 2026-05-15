//! AggressorBurstDetector — detects one-sided buy/sell bursts in a short window.
//!
//! Within `burst_window_ms`, if buy_ratio >= `directional_threshold` AND total
//! count >= `min_count` → buy burst (+1).
//! If sell_ratio >= `directional_threshold` AND total count >= `min_count` → sell burst (-1).
//! Otherwise no burst (0).
//!
//! Output: `IndicatorValue::Signal(i8)`: +1 buy burst, -1 sell burst, 0 none.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Short-window aggressor burst detector.
#[derive(Debug, Clone)]
pub struct AggressorBurstDetector {
    burst_window_ms: i64,
    min_count: usize,
    /// Minimum fraction of one-sided ticks to declare a burst (e.g. 0.8).
    directional_threshold: f64,
    /// (timestamp_ms, is_buy)
    events: VecDeque<(i64, bool)>,
    last_signal: i8,
}

impl AggressorBurstDetector {
    /// Create detector.
    ///
    /// - `burst_window_ms`: rolling window to scan (e.g. 5 000 ms).
    /// - `min_count`: minimum ticks in window to evaluate (default 5).
    /// - `directional_threshold`: fraction of one side needed (e.g. 0.8 = 80%).
    pub fn new(burst_window_ms: i64, min_count: usize, directional_threshold: f64) -> Self {
        Self {
            burst_window_ms: burst_window_ms.max(1),
            min_count: min_count.max(1),
            directional_threshold: directional_threshold.clamp(0.5, 1.0),
            events: VecDeque::with_capacity(256),
            last_signal: 0,
        }
    }
}

impl TickConsumer for AggressorBurstDetector {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        self.events.push_back((tick.time, tick.is_buy));

        // Evict events outside the burst window.
        while let Some(&(ts, _)) = self.events.front() {
            if tick.time - ts > self.burst_window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }

        let total = self.events.len();
        if total >= self.min_count {
            let buy_count = self.events.iter().filter(|&&(_, is_buy)| is_buy).count();
            let sell_count = total - buy_count;
            let buy_ratio = buy_count as f64 / total as f64;
            let sell_ratio = sell_count as f64 / total as f64;

            self.last_signal = if buy_ratio >= self.directional_threshold {
                1
            } else if sell_ratio >= self.directional_threshold {
                -1
            } else {
                0
            };
        } else {
            self.last_signal = 0;
        }

        IndicatorValue::Signal(self.last_signal)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    fn reset(&mut self) {
        self.events.clear();
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        self.events.len() >= self.min_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick_buy(time_ms: i64) -> Tick {
        Tick::new(time_ms, 100.0, 1.0, true)
    }

    fn tick_sell(time_ms: i64) -> Tick {
        Tick::new(time_ms, 100.0, 1.0, false)
    }

    #[test]
    fn buy_burst_detected() {
        // min_count=5, threshold=0.8 → need 4/5 buys
        let mut det = AggressorBurstDetector::new(10_000, 5, 0.8);
        for i in 0..5 {
            det.update_tick(&tick_buy(i as i64 * 100));
        }
        assert_eq!(det.value(), IndicatorValue::Signal(1), "5 buys → buy burst");
    }

    #[test]
    fn sell_burst_detected() {
        let mut det = AggressorBurstDetector::new(10_000, 5, 0.8);
        for i in 0..5 {
            det.update_tick(&tick_sell(i as i64 * 100));
        }
        assert_eq!(det.value(), IndicatorValue::Signal(-1), "5 sells → sell burst");
    }

    #[test]
    fn mixed_gives_no_burst() {
        let mut det = AggressorBurstDetector::new(10_000, 4, 0.8);
        det.update_tick(&tick_buy(0));
        det.update_tick(&tick_sell(100));
        det.update_tick(&tick_buy(200));
        det.update_tick(&tick_sell(300));
        // 50/50 — neither threshold met
        assert_eq!(det.value(), IndicatorValue::Signal(0));
    }

    #[test]
    fn old_ticks_evicted_breaks_burst() {
        // window = 1 second; fill 5 buys, then add sell 2 s later
        let mut det = AggressorBurstDetector::new(1_000, 2, 0.8);
        for i in 0..5 {
            det.update_tick(&tick_buy(i * 100));
        }
        // 2 seconds later — all old buys evicted, only sell remains.
        det.update_tick(&tick_sell(2_100));
        // Only 1 tick in window; below min_count=2 → no signal.
        assert_eq!(det.value(), IndicatorValue::Signal(0));
    }

    #[test]
    fn reset_clears_state() {
        let mut det = AggressorBurstDetector::new(10_000, 3, 0.8);
        for i in 0..3 {
            det.update_tick(&tick_buy(i as i64 * 100));
        }
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.value(), IndicatorValue::Signal(0));
    }
}
