//! LargeTradeFilter — filters trades that are N× above rolling median size.
//!
//! Useful for detecting institutional activity, iceberg orders, or block trades.
//!
//! Output: `IndicatorValue::Double(signal, size_ratio)`
//!   signal:     +1.0 = large buy, -1.0 = large sell, 0.0 = normal trade
//!   size_ratio: current_size / rolling_median (how many × above median)

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Detects ticks whose size exceeds `multiplier × rolling_median_size`.
#[derive(Debug, Clone)]
pub struct LargeTradeFilter {
    window: usize,
    multiplier: f64,
    size_history: VecDeque<f64>,
    last_signal: f64,
    last_ratio: f64,
}

impl LargeTradeFilter {
    /// Create detector.
    ///
    /// - `window`: rolling window size for median computation.
    /// - `multiplier`: trades larger than `multiplier × median` are "large".
    pub fn new(window: usize, multiplier: f64) -> Self {
        let cap = window.max(2);
        Self {
            window: cap,
            multiplier: if multiplier > 0.0 { multiplier } else { 2.0 },
            size_history: VecDeque::with_capacity(cap),
            last_signal: 0.0,
            last_ratio: 0.0,
        }
    }

    fn rolling_median(&self) -> f64 {
        if self.size_history.is_empty() {
            return 0.0;
        }
        let mut sorted: Vec<f64> = self.size_history.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = sorted.len() / 2;
        if sorted.len() % 2 == 0 {
            (sorted[mid.saturating_sub(1)] + sorted[mid]) / 2.0
        } else {
            sorted[mid]
        }
    }
}

impl TickConsumer for LargeTradeFilter {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        self.size_history.push_back(tick.size);
        if self.size_history.len() > self.window {
            self.size_history.pop_front();
        }

        if self.size_history.len() < self.window {
            return IndicatorValue::Double(0.0, 0.0);
        }

        let median = self.rolling_median();
        let ratio = if median > 1e-12 {
            tick.size / median
        } else {
            0.0
        };

        let is_large = ratio >= self.multiplier;
        self.last_ratio = ratio;
        self.last_signal = if is_large {
            if tick.is_buy { 1.0 } else { -1.0 }
        } else {
            0.0
        };

        IndicatorValue::Double(self.last_signal, ratio)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.last_signal, self.last_ratio)
    }

    fn reset(&mut self) {
        self.size_history.clear();
        self.last_signal = 0.0;
        self.last_ratio = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.size_history.len() >= self.window
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
    fn not_ready_until_window_full() {
        let mut f = LargeTradeFilter::new(5, 3.0);
        for _ in 0..4 {
            let v = f.update_tick(&tick(1.0, true));
            assert!(!f.is_ready());
            assert_eq!(v, IndicatorValue::Double(0.0, 0.0));
        }
        f.update_tick(&tick(1.0, true));
        assert!(f.is_ready());
    }

    #[test]
    fn large_buy_detected() {
        let mut f = LargeTradeFilter::new(5, 3.0);
        // Fill window with small ticks
        for _ in 0..4 {
            f.update_tick(&tick(1.0, true));
        }
        // 5th tick = 10× larger than median 1.0 → large buy
        let v = f.update_tick(&tick(10.0, true));
        match v {
            IndicatorValue::Double(sig, ratio) => {
                assert!((sig - 1.0).abs() < 1e-9, "expected large buy signal: {}", sig);
                assert!(ratio >= 3.0, "ratio should be ≥ 3.0: {}", ratio);
            }
            other => panic!("expected Double, got {:?}", other),
        }
    }

    #[test]
    fn normal_tick_gives_zero_signal() {
        let mut f = LargeTradeFilter::new(5, 3.0);
        for _ in 0..5 {
            f.update_tick(&tick(1.0, true));
        }
        // Another normal-size tick
        let v = f.update_tick(&tick(1.2, false));
        match v {
            IndicatorValue::Double(sig, _) => {
                assert!((sig - 0.0).abs() < 1e-9, "expected no signal: {}", sig);
            }
            other => panic!("expected Double, got {:?}", other),
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut f = LargeTradeFilter::new(3, 2.0);
        for _ in 0..3 {
            f.update_tick(&tick(1.0, true));
        }
        f.reset();
        assert!(!f.is_ready());
        assert_eq!(f.value(), IndicatorValue::Double(0.0, 0.0));
    }
}
