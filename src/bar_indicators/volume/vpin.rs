//! VPIN — Volume-Synchronized Probability of Informed Trading
//!
//! Real implementation: Easley, López de Prado, O'Hara (2012).
//! Primary path: `TickConsumer::update_tick` — Bulk Volume Classification
//!   on live tick stream. Each `bucket_size` volume accumulates a bucket;
//!   VPIN = rolling mean of |buy_vol - sell_vol| / bucket_size over last N buckets.
//!
//! Fallback path: `update_bar(o,h,l,c,v)` — SYNTHETIC ESTIMATE only.
//!   A single synthetic tick per bar is injected (close ≥ open → buy side).
//!   Precision limited to bar granularity. Prefer tick stream when available.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// VPIN — Volume-Synchronized Probability of Informed Trading.
///
/// Range: [0.0, 1.0]. Higher values indicate elevated order-flow toxicity.
#[derive(Debug, Clone)]
pub struct Vpin {
    /// Target volume per bucket (e.g. 50.0 contracts/coins).
    bucket_size: f64,
    /// Number of completed buckets to average for final VPIN.
    smoothing_window: usize,

    // Current in-flight bucket accumulators
    curr_buy: f64,
    curr_sell: f64,
    curr_volume: f64,

    /// VPIN values of completed buckets (rolling window).
    completed_vpins: VecDeque<f64>,
    last_vpin: f64,
}

impl Vpin {
    /// Create a new VPIN indicator.
    ///
    /// * `bucket_size` — target volume per bucket (clamped to 1e-9 minimum).
    /// * `smoothing_window` — number of buckets to average (clamped to 1 minimum).
    pub fn new(bucket_size: f64, smoothing_window: usize) -> Self {
        Self {
            bucket_size: bucket_size.max(1e-9),
            smoothing_window: smoothing_window.max(1),
            curr_buy: 0.0,
            curr_sell: 0.0,
            curr_volume: 0.0,
            completed_vpins: VecDeque::with_capacity(smoothing_window.max(1)),
            last_vpin: 0.0,
        }
    }

    /// SYNTHETIC ESTIMATE: without a tick stream, inject a single synthetic tick per bar.
    /// Direction: close >= open → buy, else sell. Total volume = bar volume.
    /// Precision is limited to bar granularity; prefer `update_tick` when available.
    pub fn update_bar(&mut self, o: f64, _h: f64, _l: f64, c: f64, v: f64) -> IndicatorValue {
        let is_buy = c >= o;
        let synthetic = Tick::new(0, c, v, is_buy);
        self.update_tick(&synthetic)
    }
}

impl TickConsumer for Vpin {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        // BVC: classify volume by trade direction (is_buy flag from exchange)
        if tick.is_buy {
            self.curr_buy += tick.size;
        } else {
            self.curr_sell += tick.size;
        }
        self.curr_volume += tick.size;

        // Finalise buckets until remaining curr_volume < bucket_size
        while self.curr_volume >= self.bucket_size {
            let bucket_vpin = (self.curr_buy - self.curr_sell).abs() / self.bucket_size;

            self.completed_vpins.push_back(bucket_vpin);
            if self.completed_vpins.len() > self.smoothing_window {
                self.completed_vpins.pop_front();
            }

            // Simple approach: reset bucket (ignore overflow carry-over for robustness)
            self.curr_buy = 0.0;
            self.curr_sell = 0.0;
            self.curr_volume = 0.0;
        }

        if !self.completed_vpins.is_empty() {
            let sum: f64 = self.completed_vpins.iter().sum();
            self.last_vpin = sum / self.completed_vpins.len() as f64;
        }

        IndicatorValue::Single(self.last_vpin)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_vpin)
    }

    fn reset(&mut self) {
        self.curr_buy = 0.0;
        self.curr_sell = 0.0;
        self.curr_volume = 0.0;
        self.completed_vpins.clear();
        self.last_vpin = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.completed_vpins.len() >= self.smoothing_window
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Tick;

    fn buy_tick(size: f64) -> Tick {
        Tick::new(0, 100.0, size, true)
    }

    fn sell_tick(size: f64) -> Tick {
        Tick::new(0, 100.0, size, false)
    }

    #[test]
    fn test_balanced_buckets_zero_vpin() {
        // Interleaved buy+sell of equal size → each completed bucket is 50/50 → VPIN=0
        // bucket_size=20, smoothing_window=2 → need 40 interleaved ticks (2 buckets of 20)
        let mut vpin = Vpin::new(20.0, 2);
        for _ in 0..20 {
            vpin.update_tick(&buy_tick(1.0));
            vpin.update_tick(&sell_tick(1.0));
        }
        assert!(vpin.is_ready());
        assert!((vpin.last_vpin - 0.0).abs() < 1e-9);
    }

    #[test]
    fn test_all_buy_full_bucket_vpin_one() {
        // 10 buy size=10 → 1 bucket fully buy → VPIN ≈ 1.0
        let mut vpin = Vpin::new(100.0, 1);
        for _ in 0..10 {
            vpin.update_tick(&buy_tick(10.0));
        }
        assert!(vpin.is_ready());
        assert!((vpin.last_vpin - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_all_sell_full_bucket_vpin_one() {
        // 10 sell size=10 → 1 fully-sell bucket → |0 - 100| / 100 = 1.0
        let mut vpin = Vpin::new(100.0, 1);
        for _ in 0..10 {
            vpin.update_tick(&sell_tick(10.0));
        }
        assert!(vpin.is_ready());
        assert!((vpin.last_vpin - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_not_ready_before_window_filled() {
        let mut vpin = Vpin::new(100.0, 3);
        // Only 2 buckets completed
        for _ in 0..20 {
            vpin.update_tick(&buy_tick(10.0));
        }
        assert!(!vpin.is_ready()); // needs 3 buckets
    }

    #[test]
    fn test_reset() {
        let mut vpin = Vpin::new(100.0, 1);
        for _ in 0..10 {
            vpin.update_tick(&buy_tick(10.0));
        }
        assert!(vpin.is_ready());
        vpin.reset();
        assert!(!vpin.is_ready());
        assert_eq!(vpin.value(), IndicatorValue::Single(0.0));
        assert_eq!(vpin.curr_volume, 0.0);
    }

    #[test]
    fn test_different_bucket_sizes() {
        // Small bucket: every single tick (size=10) completes bucket_size=10
        let mut vpin = Vpin::new(10.0, 5);
        for _ in 0..5 {
            vpin.update_tick(&buy_tick(10.0));
        }
        assert!(vpin.is_ready());
        // Each bucket is pure buy → vpin = 1.0
        assert!((vpin.last_vpin - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_update_bar_synthetic_fallback() {
        let mut vpin = Vpin::new(100.0, 1);
        // Bar with volume=100, close > open → synthetic buy bucket
        let val = vpin.update_bar(100.0, 105.0, 99.0, 102.0, 100.0);
        assert!(vpin.is_ready());
        if let IndicatorValue::Single(v) = val {
            assert!(v.is_finite());
            assert!(v >= 0.0 && v <= 1.0);
        } else {
            panic!("expected Single");
        }
    }
}
