//! AbsorptionDetector — detects large volume at minimal price movement.
//!
//! Absorption score = total_volume / (price_range + eps).
//! High score = someone absorbing order flow (price stays flat despite volume).
//!
//! Direction signal:
//!   +1.0 = buy absorption (buy_vol > sell_vol * 1.5, price flat)
//!   -1.0 = sell absorption (sell_vol > buy_vol * 1.5, price flat)
//!    0.0 = neutral / warming up
//!
//! Output: `IndicatorValue::Double(score, signal)`

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Detects absorption: high volume with minimal price movement.
#[derive(Debug, Clone)]
pub struct AbsorptionDetector {
    rolling_window_ticks: usize,
    /// Ring buffer of (price, size, is_buy) per tick.
    tick_buffer: VecDeque<(f64, f64, bool)>,
    last_absorption_score: f64,
    last_signal: f64,
}

impl AbsorptionDetector {
    /// Create with `window` ticks lookback.
    pub fn new(window: usize) -> Self {
        let cap = window.max(2);
        Self {
            rolling_window_ticks: cap,
            tick_buffer: VecDeque::with_capacity(cap),
            last_absorption_score: 0.0,
            last_signal: 0.0,
        }
    }
}

impl TickConsumer for AbsorptionDetector {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        self.tick_buffer.push_back((tick.price, tick.size, tick.is_buy));
        if self.tick_buffer.len() > self.rolling_window_ticks {
            self.tick_buffer.pop_front();
        }
        if self.tick_buffer.len() < self.rolling_window_ticks {
            return IndicatorValue::Double(0.0, 0.0);
        }

        let total_volume: f64 = self.tick_buffer.iter().map(|&(_, s, _)| s).sum();
        let first_price = self.tick_buffer.front().map(|&(p, _, _)| p).unwrap_or(0.0);
        let last_price = self.tick_buffer.back().map(|&(p, _, _)| p).unwrap_or(0.0);
        let price_range = (last_price - first_price).abs();

        let score = if price_range > 1e-9 {
            total_volume / price_range
        } else if total_volume > 0.0 {
            total_volume * 1000.0
        } else {
            0.0
        };

        let buy_vol: f64 = self.tick_buffer.iter()
            .filter(|&&(_, _, is_buy)| is_buy)
            .map(|&(_, s, _)| s)
            .sum();
        let sell_vol = total_volume - buy_vol;

        let signal = if buy_vol > sell_vol * 1.5 {
            1.0
        } else if sell_vol > buy_vol * 1.5 {
            -1.0
        } else {
            0.0
        };

        self.last_absorption_score = score;
        self.last_signal = signal;

        IndicatorValue::Double(score, signal)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.last_absorption_score, self.last_signal)
    }

    fn reset(&mut self) {
        self.tick_buffer.clear();
        self.last_absorption_score = 0.0;
        self.last_signal = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.tick_buffer.len() >= self.rolling_window_ticks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Tick;

    fn tick(price: f64, size: f64, is_buy: bool) -> Tick {
        Tick::new(0, price, size, is_buy)
    }

    #[test]
    fn not_ready_until_window_full() {
        let mut det = AbsorptionDetector::new(5);
        for _ in 0..4 {
            let v = det.update_tick(&tick(100.0, 10.0, true));
            assert!(!det.is_ready());
            assert_eq!(v, IndicatorValue::Double(0.0, 0.0));
        }
        det.update_tick(&tick(100.0, 10.0, true));
        assert!(det.is_ready());
    }

    #[test]
    fn flat_price_all_buy_gives_buy_absorption() {
        let mut det = AbsorptionDetector::new(4);
        // all ticks at same price, all buy → score high, signal +1
        for _ in 0..4 {
            det.update_tick(&tick(100.0, 10.0, true));
        }
        match det.value() {
            IndicatorValue::Double(score, signal) => {
                assert!(score > 100.0, "score should be large when price flat: {}", score);
                assert!((signal - 1.0).abs() < 1e-9, "signal should be +1: {}", signal);
            }
            other => panic!("expected Double, got {:?}", other),
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut det = AbsorptionDetector::new(3);
        for _ in 0..3 {
            det.update_tick(&tick(100.0, 10.0, true));
        }
        assert!(det.is_ready());
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.value(), IndicatorValue::Double(0.0, 0.0));
    }
}
