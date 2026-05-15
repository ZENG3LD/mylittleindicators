//! VwapDeviation — rolling VWAP with deviation from current price.
//!
//! Maintains a time-windowed VWAP and reports the percent deviation of the
//! current price from that VWAP.
//!
//! Output: `IndicatorValue::Triple(current_price, vwap, deviation_pct)`
//!   deviation_pct = (price - vwap) / vwap, e.g. 0.01 = 1% above VWAP.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Rolling VWAP deviation indicator.
///
/// Computes volume-weighted average price over a rolling `window_ms` millisecond
/// window and returns the percent deviation of the current price from that VWAP.
#[derive(Debug, Clone)]
pub struct VwapDeviation {
    window_ms: i64,
    /// (timestamp_ms, price, qty)
    events: VecDeque<(i64, f64, f64)>,
    last_price: f64,
    last_vwap: f64,
    last_deviation: f64,
}

impl VwapDeviation {
    /// Create with rolling `window_ms` millisecond window (default 60 000 ms = 1 min).
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            events: VecDeque::with_capacity(512),
            last_price: 0.0,
            last_vwap: 0.0,
            last_deviation: 0.0,
        }
    }
}

impl TickConsumer for VwapDeviation {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        self.events.push_back((tick.time, tick.price, tick.size));

        // Evict ticks outside the time window.
        while let Some(&(ts, _, _)) = self.events.front() {
            if tick.time - ts > self.window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }

        let total_qty: f64 = self.events.iter().map(|&(_, _, q)| q).sum();
        let vwap = if total_qty > 0.0 {
            self.events.iter().map(|&(_, p, q)| p * q).sum::<f64>() / total_qty
        } else {
            tick.price
        };

        self.last_price = tick.price;
        self.last_vwap = vwap;
        self.last_deviation = if vwap > 0.0 {
            (tick.price - vwap) / vwap
        } else {
            0.0
        };

        IndicatorValue::Triple(self.last_price, self.last_vwap, self.last_deviation)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_price, self.last_vwap, self.last_deviation)
    }

    fn reset(&mut self) {
        self.events.clear();
        self.last_price = 0.0;
        self.last_vwap = 0.0;
        self.last_deviation = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick_at(time_ms: i64, price: f64, size: f64) -> Tick {
        Tick::new(time_ms, price, size, true)
    }

    #[test]
    fn single_tick_deviation_is_zero() {
        let mut ind = VwapDeviation::new(60_000);
        // With only one tick, VWAP == price → deviation == 0.
        let v = ind.update_tick(&tick_at(0, 100.0, 1.0));
        match v {
            IndicatorValue::Triple(price, vwap, dev) => {
                assert!((price - 100.0).abs() < 1e-9);
                assert!((vwap - 100.0).abs() < 1e-9);
                assert!(dev.abs() < 1e-9);
            }
            other => panic!("expected Triple, got {:?}", other),
        }
    }

    #[test]
    fn deviation_above_vwap() {
        // Two ticks: large volume at 100, small volume at 110.
        // VWAP ≈ (100*10 + 110*1) / 11 ≈ 101.0
        // deviation = (110 - ~101) / ~101 > 0
        let mut ind = VwapDeviation::new(60_000);
        ind.update_tick(&tick_at(0, 100.0, 10.0));
        let v = ind.update_tick(&tick_at(1, 110.0, 1.0));
        match v {
            IndicatorValue::Triple(price, vwap, dev) => {
                assert!((price - 110.0).abs() < 1e-9);
                let expected_vwap = (100.0 * 10.0 + 110.0 * 1.0) / 11.0;
                assert!((vwap - expected_vwap).abs() < 1e-9);
                assert!(dev > 0.0, "price above vwap → positive deviation");
            }
            other => panic!("expected Triple, got {:?}", other),
        }
    }

    #[test]
    fn old_ticks_evicted_by_window() {
        let mut ind = VwapDeviation::new(1_000); // 1 second window
        ind.update_tick(&tick_at(0, 100.0, 10.0));
        // New tick 2 seconds later — old tick evicted, VWAP resets to new price.
        let v = ind.update_tick(&tick_at(2_100, 200.0, 1.0));
        match v {
            IndicatorValue::Triple(price, vwap, dev) => {
                assert!((price - 200.0).abs() < 1e-9);
                assert!((vwap - 200.0).abs() < 1e-9, "only one tick → vwap == price");
                assert!(dev.abs() < 1e-9);
            }
            other => panic!("expected Triple, got {:?}", other),
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = VwapDeviation::new(60_000);
        ind.update_tick(&tick_at(0, 100.0, 1.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }
}
