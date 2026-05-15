//! SizeWeightedDirectionalMomentum — rolling volume-weighted directional bias.
//!
//! Momentum = Σ(size × side) / Σ(size) over a time window, where
//!   side = +1 for buy ticks, -1 for sell ticks.
//!
//! Range: [-1.0, +1.0].
//!   +1.0 = all buying pressure
//!   -1.0 = all selling pressure
//!   0.0  = balanced
//!
//! Output: `IndicatorValue::Single(momentum)`

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Rolling size-weighted directional momentum.
#[derive(Debug, Clone)]
pub struct SizeWeightedDirectionalMomentum {
    window_ms: i64,
    /// (timestamp_ms, signed_size)  signed_size = +size for buys, -size for sells
    events: VecDeque<(i64, f64)>,
    last_momentum: f64,
}

impl SizeWeightedDirectionalMomentum {
    /// Create with `window_ms` millisecond rolling window (default 60 000 ms).
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            events: VecDeque::with_capacity(512),
            last_momentum: 0.0,
        }
    }
}

impl TickConsumer for SizeWeightedDirectionalMomentum {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        let signed = if tick.is_buy { tick.size } else { -tick.size };
        self.events.push_back((tick.time, signed));

        while let Some(&(ts, _)) = self.events.front() {
            if tick.time - ts > self.window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }

        let sum_signed: f64 = self.events.iter().map(|&(_, s)| s).sum();
        let sum_abs: f64 = self.events.iter().map(|&(_, s)| s.abs()).sum();

        self.last_momentum = if sum_abs > 0.0 {
            // Clamp to [-1, 1] to guard against floating-point edge cases.
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
    fn all_buys_give_plus_one() {
        let mut ind = SizeWeightedDirectionalMomentum::new(60_000);
        ind.update_tick(&tick(0, 5.0, true));
        ind.update_tick(&tick(1, 3.0, true));
        let v = ind.update_tick(&tick(2, 2.0, true));
        match v {
            IndicatorValue::Single(m) => assert!((m - 1.0).abs() < 1e-9),
            other => panic!("expected Single, got {:?}", other),
        }
    }

    #[test]
    fn balanced_gives_zero() {
        let mut ind = SizeWeightedDirectionalMomentum::new(60_000);
        ind.update_tick(&tick(0, 5.0, true));
        ind.update_tick(&tick(1, 5.0, false));
        match ind.value() {
            IndicatorValue::Single(m) => assert!(m.abs() < 1e-9),
            other => panic!("expected Single, got {:?}", other),
        }
    }

    #[test]
    fn weighted_not_equal_count() {
        // 1 buy of size 9, 9 sells of size 1 each → buy volume = 9, sell volume = 9
        // → momentum = 0
        let mut ind = SizeWeightedDirectionalMomentum::new(60_000);
        ind.update_tick(&tick(0, 9.0, true));
        for i in 1..=9 {
            ind.update_tick(&tick(i, 1.0, false));
        }
        match ind.value() {
            IndicatorValue::Single(m) => assert!(m.abs() < 1e-9, "balanced by volume: {}", m),
            other => panic!("expected Single, got {:?}", other),
        }
    }

    #[test]
    fn old_events_evicted() {
        let mut ind = SizeWeightedDirectionalMomentum::new(1_000);
        ind.update_tick(&tick(0, 100.0, false)); // large sell, old
        // 2 seconds later — sell evicted, only buy survives
        let v = ind.update_tick(&tick(2_000, 1.0, true));
        match v {
            IndicatorValue::Single(m) => assert!((m - 1.0).abs() < 1e-9, "only buy left: {}", m),
            other => panic!("expected Single, got {:?}", other),
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = SizeWeightedDirectionalMomentum::new(60_000);
        ind.update_tick(&tick(0, 5.0, true));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::Single(0.0));
    }
}
