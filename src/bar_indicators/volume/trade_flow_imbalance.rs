//! Trade Flow Imbalance — rolling signed volume imbalance from tick stream.
//!
//! Computes `(buy_vol - sell_vol) / (buy_vol + sell_vol)` over the last N ticks.
//! Output range: [-1.0, 1.0].
//!   +1.0 = all volume is buy-side (maximum buying pressure)
//!   -1.0 = all volume is sell-side (maximum selling pressure)
//!    0.0 = perfectly balanced
//!
//! Output: `IndicatorValue::Double(imbalance, total_volume)`

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Trade Flow Imbalance — amount-weighted rolling buy/sell imbalance.
#[derive(Debug, Clone)]
pub struct TradeFlowImbalance {
    rolling_window_ticks: usize,
    /// Ring buffer of (size, is_buy) per tick.
    tick_history: VecDeque<(f64, bool)>,
    last_imbalance: f64,
    last_total_volume: f64,
}

impl TradeFlowImbalance {
    /// Create with `window` ticks lookback.
    pub fn new(window: usize) -> Self {
        let cap = window.max(1);
        Self {
            rolling_window_ticks: cap,
            tick_history: VecDeque::with_capacity(cap),
            last_imbalance: 0.0,
            last_total_volume: 0.0,
        }
    }
}

impl TickConsumer for TradeFlowImbalance {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        self.tick_history.push_back((tick.size, tick.is_buy));
        if self.tick_history.len() > self.rolling_window_ticks {
            self.tick_history.pop_front();
        }

        let (mut buy, mut sell) = (0.0_f64, 0.0_f64);
        for &(sz, is_buy) in &self.tick_history {
            if is_buy {
                buy += sz;
            } else {
                sell += sz;
            }
        }

        self.last_total_volume = buy + sell;
        self.last_imbalance = if self.last_total_volume > 0.0 {
            (buy - sell) / self.last_total_volume
        } else {
            0.0
        };

        IndicatorValue::Double(self.last_imbalance, self.last_total_volume)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.last_imbalance, self.last_total_volume)
    }

    fn reset(&mut self) {
        self.tick_history.clear();
        self.last_imbalance = 0.0;
        self.last_total_volume = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.tick_history.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Tick;

    fn tick(size: f64, is_buy: bool) -> Tick {
        Tick::new(0, 100.0, size, is_buy)
    }

    #[test]
    fn test_all_buy_imbalance_plus_one() {
        let mut tfi = TradeFlowImbalance::new(10);
        for _ in 0..5 {
            tfi.update_tick(&tick(10.0, true));
        }
        assert!(tfi.is_ready());
        assert!((tfi.last_imbalance - 1.0).abs() < 1e-9);
        assert!((tfi.last_total_volume - 50.0).abs() < 1e-9);
    }

    #[test]
    fn test_balanced_imbalance_zero() {
        let mut tfi = TradeFlowImbalance::new(10);
        for _ in 0..5 {
            tfi.update_tick(&tick(10.0, true));
            tfi.update_tick(&tick(10.0, false));
        }
        assert!((tfi.last_imbalance - 0.0).abs() < 1e-9);
        assert!((tfi.last_total_volume - 100.0).abs() < 1e-9);
    }

    #[test]
    fn test_all_sell_imbalance_minus_one() {
        let mut tfi = TradeFlowImbalance::new(10);
        for _ in 0..5 {
            tfi.update_tick(&tick(10.0, false));
        }
        assert!((tfi.last_imbalance - (-1.0)).abs() < 1e-9);
    }

    #[test]
    fn test_rolling_window_evicts_old() {
        // window=2: only last 2 ticks matter
        let mut tfi = TradeFlowImbalance::new(2);
        // push 3 sell ticks — window keeps only last 2
        tfi.update_tick(&tick(10.0, false));
        tfi.update_tick(&tick(10.0, false));
        // now push 2 buy ticks — they fill the window
        tfi.update_tick(&tick(10.0, true));
        tfi.update_tick(&tick(10.0, true));
        // window = [buy, buy] → imbalance = +1.0
        assert!((tfi.last_imbalance - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_value_returns_same_as_last() {
        let mut tfi = TradeFlowImbalance::new(5);
        tfi.update_tick(&tick(20.0, true));
        let v = tfi.value();
        assert_eq!(v, IndicatorValue::Double(tfi.last_imbalance, tfi.last_total_volume));
    }

    #[test]
    fn test_reset() {
        let mut tfi = TradeFlowImbalance::new(5);
        tfi.update_tick(&tick(10.0, true));
        tfi.reset();
        assert!(!tfi.is_ready());
        assert_eq!(tfi.value(), IndicatorValue::Double(0.0, 0.0));
    }
}
