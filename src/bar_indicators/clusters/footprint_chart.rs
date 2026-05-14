//! Footprint Chart — volume-by-price breakdown with buy/sell split.
//!
//! Aggregates trade volume at each price level over a bar, splitting into
//! buy-initiated (taker lifted the ask) and sell-initiated (taker hit the bid).
//!
//! Primary path: `update_tick(&Tick)` — uses real `tick.is_buy` flag. Accurate
//! footprint requires a live tick feed with aggressor-side information.
//!
//! Fallback path: `update_bar(o,h,l,c,v)` — SYNTHETIC ESTIMATE only.
//! Volume is split proportionally: upper half of bar range → buy, lower half → sell.
//! Delta and totals are approximate; no per-level breakdown is available. Prefer
//! `update_tick` when available.
//!
//! Output: `IndicatorValue::Triple(net_delta, poc_price, total_volume)` from last
//! closed bar. Call `close_bar()` to finalize a bar and populate cached metrics.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::types::Tick;
use std::collections::HashMap;

/// Footprint Chart indicator.
///
/// Accumulates ticks for the current bar by price bucket. Call `close_bar()` to
/// snapshot the completed bar (POC, max imbalance, totals) and reset for the next.
#[derive(Clone)]
pub struct FootprintChart {
    /// Price quantization step. E.g. 0.01 for 1-cent buckets.
    price_bucket: f64,

    /// Current bar levels: bucket_index → (buy_vol, sell_vol).
    levels: HashMap<i64, (f64, f64)>,

    /// Cumulative totals for the in-progress bar.
    total_buy: f64,
    total_sell: f64,

    // ── Cached metrics from the last closed bar ──────────────────────────────
    /// Price level with maximum total volume (Point of Control).
    last_poc_price: f64,
    /// Maximum |buy - sell| / total across all levels, in percent.
    last_max_imbalance_pct: f64,
    /// Price level that had the maximum imbalance.
    last_max_imbalance_price: f64,
    /// total_buy − total_sell for the last closed bar.
    last_net_delta: f64,
    /// total_buy + total_sell for the last closed bar.
    last_total_volume: f64,
}

impl FootprintChart {
    /// `price_bucket`: price-level quantization step (e.g. 0.01 for cents, 1.0 for integer ticks).
    pub fn new(price_bucket: f64) -> Self {
        Self {
            price_bucket: price_bucket.max(1e-9),
            levels: HashMap::new(),
            total_buy: 0.0,
            total_sell: 0.0,
            last_poc_price: 0.0,
            last_max_imbalance_pct: 0.0,
            last_max_imbalance_price: 0.0,
            last_net_delta: 0.0,
            last_total_volume: 0.0,
        }
    }

    /// Add a real trade tick to the current bar accumulation.
    pub fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        let bucket = (tick.price / self.price_bucket).floor() as i64;
        let entry = self.levels.entry(bucket).or_insert((0.0, 0.0));
        if tick.is_buy {
            entry.0 += tick.size;
            self.total_buy += tick.size;
        } else {
            entry.1 += tick.size;
            self.total_sell += tick.size;
        }
        IndicatorValue::Single(self.total_buy - self.total_sell)
    }

    /// SYNTHETIC ESTIMATE: split bar volume across OHLCV price points.
    ///
    /// Volume above mid-price → buy, below → sell. No per-level breakdown is
    /// computed; only aggregate totals are updated. Prefer `update_tick`.
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, v: f64) -> IndicatorValue {
        let mid = (h + l) / 2.0;
        let buy_frac = if h > l { (c - l) / (h - l) } else { 0.5 };
        let buy_vol = v * buy_frac;
        let sell_vol = v * (1.0 - buy_frac);
        let _ = mid; // unused but kept for readability

        // Synthetic single bucket at close price
        let bucket = (c / self.price_bucket).floor() as i64;
        let entry = self.levels.entry(bucket).or_insert((0.0, 0.0));
        entry.0 += buy_vol;
        entry.1 += sell_vol;
        self.total_buy += buy_vol;
        self.total_sell += sell_vol;

        IndicatorValue::Single(self.total_buy - self.total_sell)
    }

    /// Finalize current bar: compute POC and max imbalance, then reset accumulators.
    pub fn close_bar(&mut self) {
        if self.levels.is_empty() {
            return;
        }

        let mut poc_bucket = 0i64;
        let mut poc_vol = 0.0f64;
        let mut max_imb_pct = 0.0f64;
        let mut max_imb_bucket = 0i64;

        for (&bucket, &(buy, sell)) in &self.levels {
            let total = buy + sell;
            if total > poc_vol {
                poc_vol = total;
                poc_bucket = bucket;
            }
            if total > 0.0 {
                let imb_pct = ((buy - sell).abs() / total) * 100.0;
                if imb_pct > max_imb_pct {
                    max_imb_pct = imb_pct;
                    max_imb_bucket = bucket;
                }
            }
        }

        self.last_poc_price = poc_bucket as f64 * self.price_bucket;
        self.last_max_imbalance_pct = max_imb_pct;
        self.last_max_imbalance_price = max_imb_bucket as f64 * self.price_bucket;
        self.last_net_delta = self.total_buy - self.total_sell;
        self.last_total_volume = self.total_buy + self.total_sell;

        self.levels.clear();
        self.total_buy = 0.0;
        self.total_sell = 0.0;
    }

    // ── Accessors ────────────────────────────────────────────────────────────

    /// Price level with the most volume in the last closed bar.
    pub fn poc_price(&self) -> f64 { self.last_poc_price }

    /// Maximum imbalance percent across all levels in the last closed bar.
    pub fn max_imbalance_pct(&self) -> f64 { self.last_max_imbalance_pct }

    /// Price level that had the maximum imbalance in the last closed bar.
    pub fn max_imbalance_price(&self) -> f64 { self.last_max_imbalance_price }

    /// Net delta (total_buy − total_sell) from the last closed bar.
    pub fn net_delta(&self) -> f64 { self.last_net_delta }

    /// Total volume (total_buy + total_sell) from the last closed bar.
    pub fn total_volume(&self) -> f64 { self.last_total_volume }

    /// In-progress levels map: bucket_index → (buy_vol, sell_vol).
    pub fn current_levels(&self) -> &HashMap<i64, (f64, f64)> { &self.levels }

    /// In-progress buy accumulator.
    pub fn total_buy(&self) -> f64 { self.total_buy }

    /// In-progress sell accumulator.
    pub fn total_sell(&self) -> f64 { self.total_sell }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_net_delta, self.last_poc_price, self.last_total_volume)
    }

    pub fn is_ready(&self) -> bool {
        self.last_total_volume > 0.0
    }

    pub fn reset(&mut self) {
        self.levels.clear();
        self.total_buy = 0.0;
        self.total_sell = 0.0;
        self.last_poc_price = 0.0;
        self.last_max_imbalance_pct = 0.0;
        self.last_max_imbalance_price = 0.0;
        self.last_net_delta = 0.0;
        self.last_total_volume = 0.0;
    }
}

impl TickConsumer for FootprintChart {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        FootprintChart::update_tick(self, tick)
    }
    fn value(&self) -> IndicatorValue { FootprintChart::value(self) }
    fn reset(&mut self) { FootprintChart::reset(self) }
    fn is_ready(&self) -> bool { FootprintChart::is_ready(self) }
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
    fn test_footprint_creation() {
        let fp = FootprintChart::new(0.01);
        assert!(!fp.is_ready());
        assert_eq!(fp.net_delta(), 0.0);
    }

    #[test]
    fn test_accumulate_and_close_bar() {
        let mut fp = FootprintChart::new(1.0);

        // 5 buy ticks @ price 100, qty 10 each → 50 buy at bucket 100
        for _ in 0..5 {
            fp.update_tick(&buy_tick(100.0, 10.0));
        }
        // 3 sell ticks @ price 101, qty 5 each → 15 sell at bucket 101
        for _ in 0..3 {
            fp.update_tick(&sell_tick(101.0, 5.0));
        }

        fp.close_bar();

        assert!((fp.net_delta() - 35.0).abs() < 1e-9, "net delta should be 50-15=35");
        assert_eq!(fp.poc_price(), 100.0, "POC should be at price 100 (50 vol > 15 vol)");
        assert!((fp.total_volume() - 65.0).abs() < 1e-9);
    }

    #[test]
    fn test_max_imbalance_is_100_pct_for_pure_buy() {
        let mut fp = FootprintChart::new(1.0);
        fp.update_tick(&buy_tick(100.0, 20.0));
        fp.close_bar();
        assert!((fp.max_imbalance_pct() - 100.0).abs() < 1e-9);
        assert_eq!(fp.max_imbalance_price(), 100.0);
    }

    #[test]
    fn test_footprint_bar_synthetic() {
        let mut fp = FootprintChart::new(1.0);
        // bullish bar: close above mid → buy > sell
        fp.update_bar(100.0, 110.0, 90.0, 108.0, 400.0);
        assert!(fp.total_buy() > fp.total_sell());
    }

    #[test]
    fn test_footprint_reset() {
        let mut fp = FootprintChart::new(1.0);
        fp.update_tick(&buy_tick(100.0, 10.0));
        fp.close_bar();
        fp.reset();
        assert!(!fp.is_ready());
        assert_eq!(fp.net_delta(), 0.0);
    }

    #[test]
    fn test_close_bar_clears_in_progress() {
        let mut fp = FootprintChart::new(1.0);
        fp.update_tick(&buy_tick(100.0, 10.0));
        assert!(!fp.current_levels().is_empty());
        fp.close_bar();
        assert!(fp.current_levels().is_empty());
        assert!(fp.is_ready()); // last_total_volume is populated
    }
}
