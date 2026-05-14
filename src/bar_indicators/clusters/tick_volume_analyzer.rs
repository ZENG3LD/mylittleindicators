//! Tick Volume Analyzer — buy/sell volume split from trade stream.
//!
//! Primary path: `update(&Tick)` — uses real `tick.is_buy` flag set by exchange.
//! Accurate buy/sell delta requires a live tick feed.
//!
//! Fallback path: `update_bar(o,h,l,c,v)` — SYNTHETIC ESTIMATE only.
//! close > open → all volume counted as buy; close < open → all as sell.
//! This heuristic is only an approximation. Prefer `update` when ticks available.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::types::Tick;

/// Tick Volume Analyzer.
#[derive(Clone)]
pub struct TickVolumeAnalyzer {
    period: usize,
    ticks: Vec<Tick>,

    total_volume: f64,
    buy_volume: f64,
    sell_volume: f64,

    volume_delta: f64,
    volume_ratio: f64,
    tick_count: usize,
    avg_tick_size: f64,

    avg_spread: f64,
    spread_samples: usize,
}

impl TickVolumeAnalyzer {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            ticks: Vec::with_capacity(period),
            total_volume: 0.0,
            buy_volume: 0.0,
            sell_volume: 0.0,
            volume_delta: 0.0,
            volume_ratio: 1.0,
            tick_count: 0,
            avg_tick_size: 0.0,
            avg_spread: 0.0,
            spread_samples: 0,
        }
    }

    /// Update with a real trade tick. Uses `tick.is_buy` directly.
    pub fn update(&mut self, tick: &Tick) {
        self.total_volume += tick.size;
        if tick.is_buy {
            self.buy_volume += tick.size;
        } else {
            self.sell_volume += tick.size;
        }

        if let Some(spread) = tick.spread() {
            self.avg_spread = (self.avg_spread * self.spread_samples as f64 + spread)
                / (self.spread_samples + 1) as f64;
            self.spread_samples += 1;
        }

        self.tick_count += 1;
        self.recalculate_derived();

        if self.ticks.len() >= self.period {
            self.ticks.remove(0);
        }
        self.ticks.push(*tick);
    }

    /// SYNTHETIC ESTIMATE: close > open → buy volume; close < open → sell volume;
    /// close == open (doji) → split 50/50. Not accurate — use `update` with real ticks.
    pub fn update_bar(&mut self, o: f64, _h: f64, _l: f64, c: f64, v: f64) -> f64 {
        self.total_volume += v;

        if c > o {
            self.buy_volume += v;
        } else if c < o {
            self.sell_volume += v;
        } else {
            // Doji: split evenly
            self.buy_volume += v * 0.5;
            self.sell_volume += v * 0.5;
        }

        self.tick_count += 1;
        self.recalculate_derived();
        self.volume_delta
    }

    fn recalculate_derived(&mut self) {
        self.volume_delta = self.buy_volume - self.sell_volume;
        self.volume_ratio = if self.sell_volume > 0.0 {
            self.buy_volume / self.sell_volume
        } else {
            1.0
        };
        self.avg_tick_size = self.total_volume / self.tick_count as f64;
    }

    pub fn volume_delta(&self) -> f64 { self.volume_delta }
    pub fn volume_ratio(&self) -> f64 { self.volume_ratio }
    pub fn buy_volume(&self) -> f64 { self.buy_volume }
    pub fn sell_volume(&self) -> f64 { self.sell_volume }
    pub fn avg_spread(&self) -> f64 { self.avg_spread }
    pub fn tick_count(&self) -> usize { self.tick_count }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.volume_delta)
    }

    pub fn is_ready(&self) -> bool { self.tick_count > 0 }

    pub fn reset(&mut self) {
        self.ticks.clear();
        self.total_volume = 0.0;
        self.buy_volume = 0.0;
        self.sell_volume = 0.0;
        self.volume_delta = 0.0;
        self.volume_ratio = 1.0;
        self.tick_count = 0;
        self.avg_tick_size = 0.0;
        self.avg_spread = 0.0;
        self.spread_samples = 0;
    }
}

impl TickConsumer for TickVolumeAnalyzer {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        self.update(tick);
        self.value()
    }

    fn value(&self) -> IndicatorValue {
        self.value()
    }

    fn reset(&mut self) {
        self.reset();
    }

    fn is_ready(&self) -> bool {
        self.is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tick_volume_analyzer_creation() {
        let ind = TickVolumeAnalyzer::new(100);
        assert!(!ind.is_ready());
        assert_eq!(ind.volume_delta(), 0.0);
    }

    #[test]
    fn test_tick_volume_analyzer_update_bar() {
        let mut ind = TickVolumeAnalyzer::new(100);
        // SYNTHETIC: bullish bar
        ind.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0);
        assert!(ind.is_ready());
        assert_eq!(ind.tick_count(), 1);
        assert!(ind.buy_volume() > ind.sell_volume());
    }

    #[test]
    fn test_tick_volume_analyzer_real_tick() {
        let mut ind = TickVolumeAnalyzer::new(100);
        let tick = Tick::new(1000, 100.0, 50.0, true);
        ind.update(&tick);
        assert!(ind.is_ready());
        assert_eq!(ind.buy_volume(), 50.0);
        assert_eq!(ind.sell_volume(), 0.0);
    }

    #[test]
    fn test_tick_volume_analyzer_multiple_updates() {
        let mut ind = TickVolumeAnalyzer::new(100);
        for i in 0..10 {
            ind.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0 + i as f64);
        }
        assert_eq!(ind.tick_count(), 10);
    }

    #[test]
    fn test_tick_volume_analyzer_reset() {
        let mut ind = TickVolumeAnalyzer::new(100);
        for _ in 0..5 {
            ind.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.volume_delta(), 0.0);
        assert_eq!(ind.tick_count(), 0);
    }
}
