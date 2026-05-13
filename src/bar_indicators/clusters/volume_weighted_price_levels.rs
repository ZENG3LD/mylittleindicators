//! Volume Weighted Price Levels — VWAP and volume-based support/resistance.
//!
//! This indicator is OHLCV-based and does not require L2 data.
//! VWAP is computed from (H+L+C)/3 * volume. Volume nodes and swing
//! support/resistance are derived from OHLCV bars.
//!
//! Output: current VWAP price. Additional data accessible via getters.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::types::Bar;

/// A volume-weighted price level.
#[derive(Debug, Clone)]
pub struct VwapLevel {
    pub price: f64,
    pub volume_weight: f64,
    /// Significance 0.0–1.0 (fraction of total cumulative volume).
    pub significance: f64,
    pub level_type: LevelType,
    pub touch_count: usize,
    pub last_touch_time: i64,
}

/// Classification of a price level.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LevelType {
    Support,
    Resistance,
    Pivot,
    VWAP,
    HighVolumeNode,
    LowVolumeNode,
}

/// Volume Weighted Price Levels analyser.
#[derive(Clone)]
pub struct VolumeWeightedPriceLevels {
    period: usize,
    price_precision: f64,

    volume_bars: Vec<Bar>,
    levels: Vec<VwapLevel>,

    cumulative_volume: f64,
    cumulative_price_volume: f64,
    current_vwap: f64,

    strongest_support: Option<VwapLevel>,
    strongest_resistance: Option<VwapLevel>,
    active_levels_count: usize,
}

impl VolumeWeightedPriceLevels {
    pub fn new(period: usize, price_precision: f64) -> Self {
        Self {
            period,
            price_precision,
            volume_bars: Vec::with_capacity(period),
            levels: Vec::with_capacity(64),
            cumulative_volume: 0.0,
            cumulative_price_volume: 0.0,
            current_vwap: 0.0,
            strongest_support: None,
            strongest_resistance: None,
            active_levels_count: 0,
        }
    }

    /// Update with a `Bar`. Returns current VWAP.
    pub fn update_volume_bar(&mut self, volume_bar: &Bar) -> f64 {
        if self.volume_bars.len() >= self.period {
            self.volume_bars.remove(0);
        }
        self.volume_bars.push(*volume_bar);
        self.update_vwap(volume_bar);
        self.analyze_levels();
        self.current_vwap
    }

    /// Convenience wrapper: OHLCV bar → `update_volume_bar`.
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> IndicatorValue {
        let bar = Bar { time: 0, open: o, high: h, low: l, close: c, volume: v };
        self.update_volume_bar(&bar);
        self.value()
    }

    fn update_vwap(&mut self, volume_bar: &Bar) {
        let typical_price = (volume_bar.high + volume_bar.low + volume_bar.close) / 3.0;
        self.cumulative_volume += volume_bar.volume;
        self.cumulative_price_volume += typical_price * volume_bar.volume;
        if self.cumulative_volume > 0.0 {
            self.current_vwap = self.cumulative_price_volume / self.cumulative_volume;
        }
    }

    fn analyze_levels(&mut self) {
        self.levels.clear();
        if self.volume_bars.len() < 5 {
            return;
        }
        if self.current_vwap > 0.0 {
            self.add_level(self.current_vwap, self.cumulative_volume, LevelType::VWAP);
        }
        self.find_high_volume_nodes();
        self.identify_support_resistance();
        self.update_level_statistics();
    }

    fn find_high_volume_nodes(&mut self) {
        let mut price_volume_map: std::collections::HashMap<i64, f64> =
            std::collections::HashMap::new();

        for bar in &self.volume_bars {
            let prices = [bar.open, bar.high, bar.low, bar.close];
            let volume_per_price = bar.volume / 4.0;
            for price in &prices {
                let price_key = (*price / self.price_precision).round() as i64;
                *price_volume_map.entry(price_key).or_insert(0.0) += volume_per_price;
            }
        }

        let mut volume_levels: Vec<(f64, f64)> = price_volume_map
            .into_iter()
            .map(|(price_key, volume)| (price_key as f64 * self.price_precision, volume))
            .collect();
        volume_levels.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        for (price, volume) in volume_levels.iter().take(10) {
            if *volume > self.cumulative_volume * 0.05 {
                self.add_level(*price, *volume, LevelType::HighVolumeNode);
            }
        }
    }

    fn identify_support_resistance(&mut self) {
        if self.volume_bars.len() < 10 {
            return;
        }
        let current_price = self.volume_bars.last().unwrap().close;
        let bars_clone = self.volume_bars.clone();

        for i in 2..bars_clone.len().saturating_sub(2) {
            let bar = &bars_clone[i];

            if bar.high > bars_clone[i - 1].high
                && bar.high > bars_clone[i - 2].high
                && bar.high > bars_clone[i + 1].high
                && bar.high > bars_clone[i + 2].high
            {
                let level_type = if bar.high > current_price {
                    LevelType::Resistance
                } else {
                    LevelType::Support
                };
                self.add_level(bar.high, bar.volume, level_type);
            }

            if bar.low < bars_clone[i - 1].low
                && bar.low < bars_clone[i - 2].low
                && bar.low < bars_clone[i + 1].low
                && bar.low < bars_clone[i + 2].low
            {
                let level_type = if bar.low < current_price {
                    LevelType::Support
                } else {
                    LevelType::Resistance
                };
                self.add_level(bar.low, bar.volume, level_type);
            }
        }
    }

    fn add_level(&mut self, price: f64, volume: f64, level_type: LevelType) {
        let significance = (volume / self.cumulative_volume.max(1.0)).min(1.0);
        self.levels.push(VwapLevel {
            price,
            volume_weight: volume,
            significance,
            level_type,
            touch_count: 0,
            last_touch_time: 0,
        });
    }

    fn update_level_statistics(&mut self) {
        self.strongest_support = None;
        self.strongest_resistance = None;
        self.active_levels_count = 0;

        let mut max_support_sig = 0.0f64;
        let mut max_resistance_sig = 0.0f64;

        for level in &self.levels {
            if level.significance > 0.1 {
                self.active_levels_count += 1;
            }
            match level.level_type {
                LevelType::Support if level.significance > max_support_sig => {
                    max_support_sig = level.significance;
                    self.strongest_support = Some(level.clone());
                }
                LevelType::Resistance if level.significance > max_resistance_sig => {
                    max_resistance_sig = level.significance;
                    self.strongest_resistance = Some(level.clone());
                }
                _ => {}
            }
        }
    }

    pub fn current_vwap(&self) -> f64 { self.current_vwap }
    pub fn get_levels(&self) -> &[VwapLevel] { &self.levels }
    pub fn strongest_support(&self) -> Option<&VwapLevel> { self.strongest_support.as_ref() }
    pub fn strongest_resistance(&self) -> Option<&VwapLevel> { self.strongest_resistance.as_ref() }
    pub fn active_levels_count(&self) -> usize { self.active_levels_count }

    pub fn nearest_level(&self, price: f64) -> Option<&VwapLevel> {
        self.levels.iter().min_by(|a, b| {
            let da = (a.price - price).abs();
            let db = (b.price - price).abs();
            da.partial_cmp(&db).unwrap()
        })
    }

    pub fn levels_by_type(&self, level_type: LevelType) -> Vec<&VwapLevel> {
        self.levels.iter().filter(|l| l.level_type == level_type).collect()
    }

    pub fn is_ready(&self) -> bool {
        self.volume_bars.len() >= (self.period / 2).max(5)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_vwap)
    }

    pub fn reset(&mut self) {
        self.volume_bars.clear();
        self.levels.clear();
        self.cumulative_volume = 0.0;
        self.cumulative_price_volume = 0.0;
        self.current_vwap = 0.0;
        self.strongest_support = None;
        self.strongest_resistance = None;
        self.active_levels_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vwpl_creation() {
        let ind = VolumeWeightedPriceLevels::new(20, 0.01);
        assert!(!ind.is_ready());
        assert_eq!(ind.current_vwap(), 0.0);
    }

    #[test]
    fn test_vwpl_warmup() {
        let mut ind = VolumeWeightedPriceLevels::new(10, 0.01);
        for i in 0..15 {
            let bar = Bar {
                time: i as i64,
                open: 100.0 + (i as f64 * 0.1).sin(),
                high: 101.0 + (i as f64 * 0.1).sin(),
                low: 99.0 + (i as f64 * 0.1).sin(),
                close: 100.5 + (i as f64 * 0.1).sin(),
                volume: 1000.0 + i as f64 * 10.0,
            };
            ind.update_volume_bar(&bar);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_vwpl_vwap_calculation() {
        let mut ind = VolumeWeightedPriceLevels::new(10, 0.01);
        for i in 0..10 {
            let bar = Bar {
                time: i as i64,
                open: 100.0,
                high: 102.0,
                low: 98.0,
                close: 101.0,
                volume: 1000.0,
            };
            ind.update_volume_bar(&bar);
        }
        assert!(ind.current_vwap() > 0.0);
        assert!(ind.current_vwap().is_finite());
    }

    #[test]
    fn test_vwpl_update_bar_wrapper() {
        let mut ind = VolumeWeightedPriceLevels::new(10, 0.01);
        let val = ind.update_bar(100.0, 102.0, 98.0, 101.0, 1000.0);
        assert!(val.main() > 0.0);
    }

    #[test]
    fn test_vwpl_reset() {
        let mut ind = VolumeWeightedPriceLevels::new(10, 0.01);
        for i in 0..15 {
            let bar = Bar {
                time: i as i64,
                open: 100.0,
                high: 102.0,
                low: 98.0,
                close: 101.0,
                volume: 1000.0,
            };
            ind.update_volume_bar(&bar);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.current_vwap(), 0.0);
    }
}
