//! Footprint POC — Point of Control for the current bar.
//!
//! Tracks total volume per price bucket and outputs the price level with the
//! maximum accumulated volume after `close_bar()`.
//!
//! Output: `IndicatorValue::Single(poc_price)` where `poc_price` is the bucket
//! mid-point (bucket_index * price_bucket) of the highest-volume level.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::types::Tick;
use std::collections::HashMap;

/// Footprint POC — reports the price level with maximum volume per bar.
#[derive(Clone)]
pub struct FootprintPoc {
    price_bucket: f64,
    /// In-progress bar: bucket_index → total volume.
    levels: HashMap<i64, f64>,
    /// POC price from the last closed bar.
    last_poc: f64,
}

impl FootprintPoc {
    /// `price_bucket`: price-level quantization step (e.g. 0.01, 1.0).
    pub fn new(price_bucket: f64) -> Self {
        Self {
            price_bucket: price_bucket.max(1e-9),
            levels: HashMap::new(),
            last_poc: 0.0,
        }
    }

    /// Accumulate one tick into the in-progress bar.
    ///
    /// Eagerly updates `last_poc` from the live in-bar levels so that
    /// `is_ready()` returns true after the first tick without waiting for
    /// `close_bar()`. `close_bar()` still finalises and resets the bar.
    pub fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        let bucket = (tick.price / self.price_bucket).floor() as i64;
        *self.levels.entry(bucket).or_insert(0.0) += tick.size;
        // Update live POC so is_ready() flips true mid-bar
        if let Some((&poc_bucket, _)) = self
            .levels
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
            self.last_poc = poc_bucket as f64 * self.price_bucket;
        }
        IndicatorValue::Single(self.last_poc)
    }

    /// Finalize bar: compute POC and reset accumulation.
    pub fn close_bar(&mut self) {
        if let Some((&poc_bucket, _)) = self
            .levels
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
            self.last_poc = poc_bucket as f64 * self.price_bucket;
        }
        self.levels.clear();
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_poc)
    }

    pub fn reset(&mut self) {
        self.levels.clear();
        self.last_poc = 0.0;
    }

    pub fn is_ready(&self) -> bool {
        self.last_poc != 0.0
    }

    /// POC price from the last closed bar.
    pub fn poc_price(&self) -> f64 { self.last_poc }
}

impl TickConsumer for FootprintPoc {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        FootprintPoc::update_tick(self, tick)
    }
    fn value(&self) -> IndicatorValue { FootprintPoc::value(self) }
    fn reset(&mut self) { FootprintPoc::reset(self) }
    fn is_ready(&self) -> bool { FootprintPoc::is_ready(self) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(price: f64, qty: f64) -> Tick {
        Tick::new(0, price, qty, true)
    }

    #[test]
    fn test_poc_selects_highest_volume_bucket() {
        let mut poc = FootprintPoc::new(1.0);
        // 5 ticks @ 100 (total 5) vs 10 ticks @ 101 (total 10)
        for _ in 0..5 {
            poc.update_tick(&tick(100.0, 1.0));
        }
        for _ in 0..10 {
            poc.update_tick(&tick(101.0, 1.0));
        }
        poc.close_bar();
        assert_eq!(poc.poc_price(), 101.0, "bucket 101 has more volume");
    }

    #[test]
    fn test_poc_single_bucket() {
        let mut poc = FootprintPoc::new(1.0);
        poc.update_tick(&tick(50.0, 100.0));
        poc.close_bar();
        assert_eq!(poc.poc_price(), 50.0);
    }

    #[test]
    fn test_poc_reset() {
        let mut poc = FootprintPoc::new(1.0);
        poc.update_tick(&tick(100.0, 10.0));
        poc.close_bar();
        poc.reset();
        assert_eq!(poc.poc_price(), 0.0);
        assert!(!poc.is_ready());
    }

    #[test]
    fn test_poc_not_ready_before_close() {
        let poc = FootprintPoc::new(1.0);
        assert!(!poc.is_ready());
    }
}
