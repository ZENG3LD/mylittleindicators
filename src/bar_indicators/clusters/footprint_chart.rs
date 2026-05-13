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
//! Delta and totals are approximate. Prefer `update_tick` when available.
//!
//! Output: `IndicatorValue::Single(net_delta)` where net_delta = total_buy - total_sell.
//! Use the `bar_data()` getter for the full price-level map.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::types::Tick;
use std::collections::BTreeMap;

/// Volume at a single price level for one bar.
#[derive(Debug, Clone, Default)]
pub struct FootprintLevel {
    pub price: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
}

impl FootprintLevel {
    pub fn total(&self) -> f64 { self.buy_volume + self.sell_volume }
    pub fn delta(&self) -> f64 { self.buy_volume - self.sell_volume }
}

/// Footprint Chart indicator.
///
/// Accumulates ticks for the current bar. Call `finish_bar()` to snapshot
/// the completed bar and reset for the next one.
#[derive(Clone)]
pub struct FootprintChart {
    tick_size: f64,
    /// Current bar levels: price_key → FootprintLevel.
    current_bar: BTreeMap<i64, FootprintLevel>,
    /// Completed bar snapshot (cleared on next `finish_bar`).
    last_bar: BTreeMap<i64, FootprintLevel>,

    total_buy: f64,
    total_sell: f64,
    net_delta: f64,
}

impl FootprintChart {
    /// `tick_size`: price rounding granularity (e.g. 0.01 for cents, 1.0 for integers).
    pub fn new(tick_size: f64) -> Self {
        Self {
            tick_size: tick_size.max(1e-12),
            current_bar: BTreeMap::new(),
            last_bar: BTreeMap::new(),
            total_buy: 0.0,
            total_sell: 0.0,
            net_delta: 0.0,
        }
    }

    fn price_key(&self, price: f64) -> i64 {
        (price / self.tick_size).round() as i64
    }

    /// Process a real trade tick. Uses `tick.is_buy` directly.
    pub fn update_tick(&mut self, tick: &Tick) {
        let key = self.price_key(tick.price);
        let level = self.current_bar.entry(key).or_insert_with(|| FootprintLevel {
            price: tick.price,
            ..Default::default()
        });
        if tick.is_buy {
            level.buy_volume += tick.size;
            self.total_buy += tick.size;
        } else {
            level.sell_volume += tick.size;
            self.total_sell += tick.size;
        }
        self.net_delta = self.total_buy - self.total_sell;
    }

    /// SYNTHETIC ESTIMATE: split bar volume across OHLCV price points.
    /// Volume above mid-price → buy, below → sell. Not accurate.
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> IndicatorValue {
        let mid = (h + l) / 2.0;
        let volume_per_level = v / 4.0;

        for (price, is_buy) in [
            (o, o >= mid),
            (h, true),   // high is always above mid
            (l, false),  // low is always below mid
            (c, c >= mid),
        ] {
            let key = self.price_key(price);
            let level = self.current_bar.entry(key).or_insert_with(|| FootprintLevel {
                price,
                ..Default::default()
            });
            if is_buy {
                level.buy_volume += volume_per_level;
                self.total_buy += volume_per_level;
            } else {
                level.sell_volume += volume_per_level;
                self.total_sell += volume_per_level;
            }
        }
        self.net_delta = self.total_buy - self.total_sell;
        IndicatorValue::Single(self.net_delta)
    }

    /// Snapshot current bar into `last_bar` and reset accumulation.
    pub fn finish_bar(&mut self) {
        self.last_bar = std::mem::take(&mut self.current_bar);
        self.total_buy = 0.0;
        self.total_sell = 0.0;
        self.net_delta = 0.0;
    }

    /// Levels accumulated in the current (in-progress) bar.
    pub fn bar_data(&self) -> &BTreeMap<i64, FootprintLevel> {
        &self.current_bar
    }

    /// Levels from the last completed bar (after `finish_bar`).
    pub fn last_bar_data(&self) -> &BTreeMap<i64, FootprintLevel> {
        &self.last_bar
    }

    pub fn total_buy(&self) -> f64 { self.total_buy }
    pub fn total_sell(&self) -> f64 { self.total_sell }
    pub fn net_delta(&self) -> f64 { self.net_delta }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.net_delta)
    }

    pub fn is_ready(&self) -> bool {
        !self.current_bar.is_empty() || !self.last_bar.is_empty()
    }

    pub fn reset(&mut self) {
        self.current_bar.clear();
        self.last_bar.clear();
        self.total_buy = 0.0;
        self.total_sell = 0.0;
        self.net_delta = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_footprint_creation() {
        let fp = FootprintChart::new(0.01);
        assert!(!fp.is_ready());
        assert_eq!(fp.net_delta(), 0.0);
    }

    #[test]
    fn test_footprint_real_tick_buy() {
        let mut fp = FootprintChart::new(1.0);
        let tick = Tick::new(1000, 100.0, 50.0, true);
        fp.update_tick(&tick);
        assert_eq!(fp.total_buy(), 50.0);
        assert_eq!(fp.total_sell(), 0.0);
        assert_eq!(fp.net_delta(), 50.0);
    }

    #[test]
    fn test_footprint_real_tick_sell() {
        let mut fp = FootprintChart::new(1.0);
        let tick = Tick::new(1000, 100.0, 30.0, false);
        fp.update_tick(&tick);
        assert_eq!(fp.total_sell(), 30.0);
        assert_eq!(fp.net_delta(), -30.0);
    }

    #[test]
    fn test_footprint_bar_synthetic() {
        let mut fp = FootprintChart::new(1.0);
        // bullish bar: close > mid so buy > sell
        fp.update_bar(100.0, 110.0, 90.0, 108.0, 400.0);
        assert!(fp.total_buy() > fp.total_sell());
    }

    #[test]
    fn test_footprint_finish_bar() {
        let mut fp = FootprintChart::new(1.0);
        let tick = Tick::new(1000, 100.0, 10.0, true);
        fp.update_tick(&tick);
        assert!(!fp.bar_data().is_empty());
        fp.finish_bar();
        assert!(fp.bar_data().is_empty());
        assert!(!fp.last_bar_data().is_empty());
        assert_eq!(fp.net_delta(), 0.0);
    }

    #[test]
    fn test_footprint_reset() {
        let mut fp = FootprintChart::new(1.0);
        let tick = Tick::new(1000, 100.0, 10.0, true);
        fp.update_tick(&tick);
        fp.reset();
        assert!(!fp.is_ready());
        assert_eq!(fp.net_delta(), 0.0);
    }
}
