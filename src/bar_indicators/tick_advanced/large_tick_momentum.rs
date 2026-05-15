//! LargeTickMomentum — directional momentum of large-size ticks only.
//!
//! Filters ticks by a fixed `size_threshold`. Only ticks whose size exceeds
//! the threshold contribute to momentum. Momentum is the volume-weighted
//! directional bias of those large ticks over a rolling time window.
//!
//! momentum = Σ(size × side) / Σ(size)  for large ticks in window
//!   side = +1 for buy, -1 for sell
//!   range: [-1.0, +1.0]
//!
//! Output: `IndicatorValue::Single(momentum)`

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Large-tick-only directional momentum over a rolling time window.
#[derive(Debug, Clone)]
pub struct LargeTickMomentum {
    size_threshold: f64,
    window_ms: i64,
    /// (timestamp_ms, size, is_buy) — only large ticks stored.
    events: VecDeque<(i64, f64, bool)>,
    last_momentum: f64,
}

impl LargeTickMomentum {
    /// Create indicator.
    ///
    /// - `size_threshold`: minimum tick size to include (e.g. 1.0 BTC).
    /// - `window_ms`: rolling time window in milliseconds.
    pub fn new(size_threshold: f64, window_ms: i64) -> Self {
        Self {
            size_threshold: size_threshold.max(0.0),
            window_ms: window_ms.max(1),
            events: VecDeque::with_capacity(256),
            last_momentum: 0.0,
        }
    }
}

impl TickConsumer for LargeTickMomentum {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        // Only record large ticks.
        if tick.size > self.size_threshold {
            self.events.push_back((tick.time, tick.size, tick.is_buy));
        }

        // Evict old large ticks outside the window.
        while let Some(&(ts, _, _)) = self.events.front() {
            if tick.time - ts > self.window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }

        let sum_signed: f64 = self.events.iter().map(|&(_, sz, buy)| {
            if buy { sz } else { -sz }
        }).sum();
        let sum_abs: f64 = self.events.iter().map(|&(_, sz, _)| sz).sum();

        self.last_momentum = if sum_abs > 0.0 {
            (sum_signed / sum_abs).clamp(-1.0, 1.0)
        } else {
            0.0
        };

        IndicatorValue::Single(self.last_momentum)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_momentum)
    }

    fn reset(&mut self) {
        self.events.clear();
        self.last_momentum = 0.0;
    }

    /// Ready once at least one large tick is in the window.
    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(time_ms: i64, size: f64, is_buy: bool) -> Tick {
        Tick::new(time_ms, 100.0, size, is_buy)
    }

    #[test]
    fn small_ticks_ignored() {
        let mut ind = LargeTickMomentum::new(5.0, 60_000);
        // All ticks below threshold.
        ind.update_tick(&tick(0, 1.0, true));
        ind.update_tick(&tick(1, 2.0, false));
        ind.update_tick(&tick(2, 4.9, true));
        assert!(!ind.is_ready(), "no large ticks → not ready");
        assert_eq!(ind.value(), IndicatorValue::Single(0.0));
    }

    #[test]
    fn large_buy_ticks_give_plus_one() {
        let mut ind = LargeTickMomentum::new(5.0, 60_000);
        ind.update_tick(&tick(0, 10.0, true));
        ind.update_tick(&tick(1, 20.0, true));
        match ind.value() {
            IndicatorValue::Single(m) => assert!((m - 1.0).abs() < 1e-9),
            other => panic!("expected Single, got {:?}", other),
        }
    }

    #[test]
    fn balanced_large_ticks_give_zero() {
        let mut ind = LargeTickMomentum::new(5.0, 60_000);
        ind.update_tick(&tick(0, 10.0, true));
        ind.update_tick(&tick(1, 10.0, false));
        match ind.value() {
            IndicatorValue::Single(m) => assert!(m.abs() < 1e-9),
            other => panic!("expected Single, got {:?}", other),
        }
    }

    #[test]
    fn old_large_ticks_evicted() {
        let mut ind = LargeTickMomentum::new(5.0, 1_000); // 1 second window
        ind.update_tick(&tick(0, 100.0, false)); // large sell, old
        // 2 seconds later — sell evicted, large buy arrives.
        let v = ind.update_tick(&tick(2_000, 10.0, true));
        match v {
            IndicatorValue::Single(m) => assert!((m - 1.0).abs() < 1e-9),
            other => panic!("expected Single, got {:?}", other),
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = LargeTickMomentum::new(5.0, 60_000);
        ind.update_tick(&tick(0, 10.0, true));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::Single(0.0));
    }
}
