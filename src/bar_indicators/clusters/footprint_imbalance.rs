//! Footprint Imbalance — detects price levels with extreme buy/sell skew.
//!
//! Scans all price buckets accumulated in the current bar and finds the level
//! with the largest signed imbalance `(buy - sell) / total`. Reports a signal
//! when any level's absolute imbalance exceeds `threshold_pct`.
//!
//! Output: `IndicatorValue::Triple(signal, imb_price, imb_pct)` where:
//! - `signal`: `+1` if buy-dominated above threshold, `-1` if sell-dominated, `0` otherwise.
//! - `imb_price`: price of the level with maximum absolute imbalance.
//! - `imb_pct`: absolute imbalance percent at that level (0..100).
//!
//! Call `close_bar()` to finalize each bar. In-bar `update_tick` returns the
//! current cached value unchanged until the next `close_bar`.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::types::Tick;
use std::collections::HashMap;

/// Footprint Imbalance — signals price levels with extreme buy/sell skew.
#[derive(Clone)]
pub struct FootprintImbalance {
    price_bucket: f64,
    /// Trigger threshold in percent. E.g. 75.0 means signal when ≥ 75% one-sided.
    threshold_pct: f64,

    /// In-progress bar: bucket_index → (buy_vol, sell_vol).
    levels: HashMap<i64, (f64, f64)>,

    // ── Cached results from last closed bar ──────────────────────────────────
    /// -1 = sell-dominated extreme, +1 = buy-dominated extreme, 0 = none.
    last_signal: i8,
    /// Price of the level with maximum absolute imbalance.
    last_imb_price: f64,
    /// Absolute imbalance percent at that level.
    last_imb_pct: f64,
}

impl FootprintImbalance {
    /// `price_bucket`: price-level quantization step.
    /// `threshold_pct`: minimum imbalance percent to fire a signal (0..100).
    pub fn new(price_bucket: f64, threshold_pct: f64) -> Self {
        Self {
            price_bucket: price_bucket.max(1e-9),
            threshold_pct: threshold_pct.clamp(0.0, 100.0),
            levels: HashMap::new(),
            last_signal: 0,
            last_imb_price: 0.0,
            last_imb_pct: 0.0,
        }
    }

    /// Accumulate one tick into the in-progress bar.
    ///
    /// Eagerly recomputes `last_signal`, `last_imb_price`, and `last_imb_pct`
    /// from the live in-bar levels so that live consumers see non-zero values
    /// without waiting for `close_bar()`. `close_bar()` still finalises the bar.
    pub fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        let bucket = (tick.price / self.price_bucket).floor() as i64;
        let entry = self.levels.entry(bucket).or_insert((0.0, 0.0));
        if tick.is_buy {
            entry.0 += tick.size;
        } else {
            entry.1 += tick.size;
        }
        // Eagerly update cached fields from live in-bar state
        let mut max_signed_pct = 0.0f64;
        let mut max_price = 0.0f64;
        for (&bkt, &(buy, sell)) in &self.levels {
            let total = buy + sell;
            if total <= 0.0 {
                continue;
            }
            let signed_pct = ((buy - sell) / total) * 100.0;
            if signed_pct.abs() > max_signed_pct.abs() {
                max_signed_pct = signed_pct;
                max_price = bkt as f64 * self.price_bucket;
            }
        }
        self.last_signal = if max_signed_pct >= self.threshold_pct {
            1
        } else if max_signed_pct <= -self.threshold_pct {
            -1
        } else {
            0
        };
        self.last_imb_price = max_price;
        self.last_imb_pct = max_signed_pct.abs();
        self.value()
    }

    /// Finalize bar: find the level with maximum signed imbalance and compare to threshold.
    pub fn close_bar(&mut self) {
        let mut max_signed_pct = 0.0f64;
        let mut max_price = 0.0f64;

        for (&bucket, &(buy, sell)) in &self.levels {
            let total = buy + sell;
            if total <= 0.0 {
                continue;
            }
            let signed_pct = ((buy - sell) / total) * 100.0; // −100..+100
            if signed_pct.abs() > max_signed_pct.abs() {
                max_signed_pct = signed_pct;
                max_price = bucket as f64 * self.price_bucket;
            }
        }

        self.last_signal = if max_signed_pct >= self.threshold_pct {
            1
        } else if max_signed_pct <= -self.threshold_pct {
            -1
        } else {
            0
        };
        self.last_imb_price = max_price;
        self.last_imb_pct = max_signed_pct.abs();

        self.levels.clear();
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(
            self.last_signal as f64,
            self.last_imb_price,
            self.last_imb_pct,
        )
    }

    pub fn reset(&mut self) {
        self.levels.clear();
        self.last_signal = 0;
        self.last_imb_price = 0.0;
        self.last_imb_pct = 0.0;
    }

    pub fn is_ready(&self) -> bool {
        !self.levels.is_empty() || self.last_imb_pct > 0.0
    }

    /// Last signal: +1 buy extreme, -1 sell extreme, 0 neutral.
    pub fn signal(&self) -> i8 { self.last_signal }
    /// Price level with the maximum imbalance from the last closed bar.
    pub fn imbalance_price(&self) -> f64 { self.last_imb_price }
    /// Absolute imbalance percent from the last closed bar.
    pub fn imbalance_pct(&self) -> f64 { self.last_imb_pct }
}

impl TickConsumer for FootprintImbalance {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        FootprintImbalance::update_tick(self, tick)
    }
    fn value(&self) -> IndicatorValue { FootprintImbalance::value(self) }
    fn reset(&mut self) { FootprintImbalance::reset(self) }
    fn is_ready(&self) -> bool { FootprintImbalance::is_ready(self) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buy_tick(price: f64, qty: f64) -> Tick {
        Tick::new(0, price, qty, true)
    }

    fn sell_tick(price: f64, qty: f64) -> Tick {
        Tick::new(0, price, qty, false)
    }

    #[test]
    fn test_buy_extreme_signals_plus_one() {
        let mut fi = FootprintImbalance::new(1.0, 75.0);
        // 10 buy, 0 sell at price 100 → 100% buy
        for _ in 0..10 {
            fi.update_tick(&buy_tick(100.0, 1.0));
        }
        fi.close_bar();
        assert_eq!(fi.signal(), 1);
        assert!((fi.imbalance_pct() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn test_sell_extreme_signals_minus_one() {
        let mut fi = FootprintImbalance::new(1.0, 75.0);
        for _ in 0..10 {
            fi.update_tick(&sell_tick(100.0, 1.0));
        }
        fi.close_bar();
        assert_eq!(fi.signal(), -1);
    }

    #[test]
    fn test_balanced_no_signal() {
        let mut fi = FootprintImbalance::new(1.0, 75.0);
        // exactly 50/50
        for _ in 0..5 {
            fi.update_tick(&buy_tick(100.0, 1.0));
            fi.update_tick(&sell_tick(100.0, 1.0));
        }
        fi.close_bar();
        assert_eq!(fi.signal(), 0);
    }

    #[test]
    fn test_reset_clears_state() {
        let mut fi = FootprintImbalance::new(1.0, 75.0);
        fi.update_tick(&buy_tick(100.0, 10.0));
        fi.close_bar();
        fi.reset();
        assert_eq!(fi.signal(), 0);
        assert_eq!(fi.imbalance_pct(), 0.0);
        assert!(!fi.is_ready());
    }
}
