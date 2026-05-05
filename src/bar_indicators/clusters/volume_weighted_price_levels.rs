//! Volume Weighted Price Levels - анализатор объемно-взвешенных ценовых уровней
//! Определяет ключевые уровни поддержки/сопротивления на основе объемного анализа

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::types::Bar;
use arrayvec::ArrayVec;

/// Объемно-взвешенный ценовой уровень
#[derive(Debug, Clone)]
pub struct VwapLevel {
    pub price: f64,
    pub volume_weight: f64,
    pub significance: f64, // 0.0 - 1.0
    pub level_type: LevelType,
    pub touch_count: usize,
    pub last_touch_time: i64,
}

/// Тип ценового уровня
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LevelType {
    Support,
    Resistance,
    Pivot,
    VWAP,
    HighVolumeNode,
    LowVolumeNode,
}

/// Анализатор объемно-взвешенных ценовых уровней
#[derive(Clone)]
pub struct VolumeWeightedPriceLevels {
    period: usize,
    price_precision: f64,
    
    // Буферы данных
    volume_bars: ArrayVec<Bar, 512>,
    levels: ArrayVec<VwapLevel, 64>,
    
    // VWAP расчеты
    cumulative_volume: f64,
    cumulative_price_volume: f64,
    current_vwap: f64,
    
    // Статистика уровней
    strongest_support: Option<VwapLevel>,
    strongest_resistance: Option<VwapLevel>,
    active_levels_count: usize,
}

impl VolumeWeightedPriceLevels {
    pub fn new(period: usize, price_precision: f64) -> Self {
        Self {
            period,
            price_precision,
            volume_bars: ArrayVec::new(),
            levels: ArrayVec::new(),
            cumulative_volume: 0.0,
            cumulative_price_volume: 0.0,
            current_vwap: 0.0,
            strongest_support: None,
            strongest_resistance: None,
            active_levels_count: 0,
        }
    }
    
    /// Обновить анализатор новым Bar
    pub fn update_volume_bar(&mut self, volume_bar: &Bar) -> f64 {
        // Добавляем в буфер
        if self.volume_bars.len() >= self.period {
            self.volume_bars.remove(0);
        }
        self.volume_bars.push(*volume_bar);
        
        // Обновляем VWAP
        self.update_vwap(volume_bar);
        
        // Анализируем уровни
        self.analyze_levels();
        
        self.current_vwap
    }
    
    /// Обновить VWAP расчеты
    fn update_vwap(&mut self, volume_bar: &Bar) {
        let typical_price = (volume_bar.high + volume_bar.low + volume_bar.close) / 3.0;
        let price_volume = typical_price * volume_bar.volume;
        
        self.cumulative_volume += volume_bar.volume;
        self.cumulative_price_volume += price_volume;
        
        if self.cumulative_volume > 0.0 {
            self.current_vwap = self.cumulative_price_volume / self.cumulative_volume;
        }
    }
    
    /// Анализировать ценовые уровни
    fn analyze_levels(&mut self) {
        self.levels.clear();
        
        if self.volume_bars.len() < 5 {
            return;
        }
        
        // Добавляем текущий VWAP как уровень
        if self.current_vwap > 0.0 {
            self.add_level(self.current_vwap, self.cumulative_volume, LevelType::VWAP);
        }
        
        // Анализируем high volume nodes
        self.find_high_volume_nodes();
        
        // Определяем уровни поддержки/сопротивления
        self.identify_support_resistance();
        
        // Обновляем статистику
        self.update_level_statistics();
    }
    
    /// Найти узлы высокого объема
    fn find_high_volume_nodes(&mut self) {
        let mut price_volume_map: std::collections::HashMap<i64, f64> = std::collections::HashMap::new();
        
        // Группируем объем по ценовым уровням
        for bar in &self.volume_bars {
            let prices = [bar.open, bar.high, bar.low, bar.close];
            let volume_per_price = bar.volume / 4.0;
            
            for price in &prices {
                let price_key = (*price / self.price_precision).round() as i64;
                *price_volume_map.entry(price_key).or_insert(0.0) += volume_per_price;
            }
        }
        
        // Находим уровни с высоким объемом
        let mut volume_levels: Vec<(f64, f64)> = price_volume_map
            .into_iter()
            .map(|(price_key, volume)| (price_key as f64 * self.price_precision, volume))
            .collect();
        
        volume_levels.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        // Добавляем топ уровни
        for (price, volume) in volume_levels.iter().take(10) {
            if *volume > self.cumulative_volume * 0.05 { // Минимум 5% от общего объема
                self.add_level(*price, *volume, LevelType::HighVolumeNode);
            }
        }
    }
    
    /// Определить уровни поддержки и сопротивления
    fn identify_support_resistance(&mut self) {
        if self.volume_bars.len() < 10 {
            return;
        }
        
        let current_price = self.volume_bars.last().unwrap().close;
        
        // Клонируем данные для избежания проблем с заимствованием
        let bars_clone = self.volume_bars.clone();
        
        // Ищем локальные максимумы и минимумы
        for i in 2..bars_clone.len()-2 {
            let bar = &bars_clone[i];
            
            // Проверяем локальный максимум (сопротивление)
            if bar.high > bars_clone[i-1].high && 
               bar.high > bars_clone[i-2].high &&
               bar.high > bars_clone[i+1].high && 
               bar.high > bars_clone[i+2].high {
                
                let level_type = if bar.high > current_price {
                    LevelType::Resistance
                } else {
                    LevelType::Support
                };
                
                self.add_level(bar.high, bar.volume, level_type);
            }
            
            // Проверяем локальный минимум (поддержка)
            if bar.low < bars_clone[i-1].low && 
               bar.low < bars_clone[i-2].low &&
               bar.low < bars_clone[i+1].low && 
               bar.low < bars_clone[i+2].low {
                
                let level_type = if bar.low < current_price {
                    LevelType::Support
                } else {
                    LevelType::Resistance
                };
                
                self.add_level(bar.low, bar.volume, level_type);
            }
        }
    }
    
    /// Добавить уровень
    fn add_level(&mut self, price: f64, volume: f64, level_type: LevelType) {
        if self.levels.is_full() {
            return;
        }
        
        let significance = (volume / self.cumulative_volume.max(1.0)).min(1.0);
        
        let level = VwapLevel {
            price,
            volume_weight: volume,
            significance,
            level_type,
            touch_count: 0,
            last_touch_time: 0,
        };
        
        self.levels.push(level);
    }
    
    /// Обновить статистику уровней
    fn update_level_statistics(&mut self) {
        self.strongest_support = None;
        self.strongest_resistance = None;
        self.active_levels_count = 0;
        
        let mut max_support_significance = 0.0;
        let mut max_resistance_significance = 0.0;
        
        for level in &self.levels {
            if level.significance > 0.1 {
                self.active_levels_count += 1;
            }
            
            match level.level_type {
                LevelType::Support => {
                    if level.significance > max_support_significance {
                        max_support_significance = level.significance;
                        self.strongest_support = Some(level.clone());
                    }
                }
                LevelType::Resistance => {
                    if level.significance > max_resistance_significance {
                        max_resistance_significance = level.significance;
                        self.strongest_resistance = Some(level.clone());
                    }
                }
                _ => {}
            }
        }
    }
    
    /// Получить текущий VWAP
    pub fn current_vwap(&self) -> f64 {
        self.current_vwap
    }
    
    /// Получить все уровни
    pub fn get_levels(&self) -> &ArrayVec<VwapLevel, 64> {
        &self.levels
    }
    
    /// Получить сильнейший уровень поддержки
    pub fn strongest_support(&self) -> Option<&VwapLevel> {
        self.strongest_support.as_ref()
    }
    
    /// Получить сильнейший уровень сопротивления
    pub fn strongest_resistance(&self) -> Option<&VwapLevel> {
        self.strongest_resistance.as_ref()
    }
    
    /// Получить количество активных уровней
    pub fn active_levels_count(&self) -> usize {
        self.active_levels_count
    }
    
    /// Найти ближайший уровень к цене
    pub fn nearest_level(&self, price: f64) -> Option<&VwapLevel> {
        self.levels
            .iter()
            .min_by(|a, b| {
                let dist_a = (a.price - price).abs();
                let dist_b = (b.price - price).abs();
                dist_a.partial_cmp(&dist_b).unwrap()
            })
    }
    
    /// Получить уровни определенного типа
    pub fn levels_by_type(&self, level_type: LevelType) -> Vec<&VwapLevel> {
        self.levels
            .iter()
            .filter(|level| level.level_type == level_type)
            .collect()
    }
    
    /// Проверить готовность анализатора
    pub fn is_ready(&self) -> bool {
        self.volume_bars.len() >= (self.period / 2).max(5)
    }

    /// Получить значение как IndicatorValue
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_vwap)
    }
    
    /// Сбросить анализатор
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






















