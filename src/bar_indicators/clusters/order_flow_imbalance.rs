//! Order Flow Imbalance - анализатор дисбаланса ордер флоу
//! Анализирует дисбаланс между покупками и продажами на разных ценовых уровнях

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::types::Bar;
use arrayvec::ArrayVec;
use std::collections::HashMap;

/// Ценовой уровень с объемной информацией
#[derive(Debug, Clone)]
pub struct PriceLevel {
    pub price: f64,
    pub buy_volume: f64,
    pub sell_volume: f64,
    pub total_volume: f64,
    pub imbalance: f64, // (buy - sell) / total
    pub imbalance_ratio: f64, // buy / sell
}

impl PriceLevel {
    pub fn new(price: f64) -> Self {
        Self {
            price,
            buy_volume: 0.0,
            sell_volume: 0.0,
            total_volume: 0.0,
            imbalance: 0.0,
            imbalance_ratio: 1.0,
        }
    }
    
    pub fn add_volume(&mut self, volume: f64, is_buy: bool) {
        if is_buy {
            self.buy_volume += volume;
        } else {
            self.sell_volume += volume;
        }
        self.total_volume += volume;
        self.update_metrics();
    }
    
    fn update_metrics(&mut self) {
        if self.total_volume > 0.0 {
            self.imbalance = (self.buy_volume - self.sell_volume) / self.total_volume;
        }
        
        if self.sell_volume > 0.0 {
            self.imbalance_ratio = self.buy_volume / self.sell_volume;
        } else {
            self.imbalance_ratio = if self.buy_volume > 0.0 { f64::INFINITY } else { 1.0 };
        }
    }
}

/// Анализатор дисбаланса order flow
#[derive(Clone)]
pub struct OrderFlowImbalance {
    period: usize,
    tick_size: f64, // Минимальный размер тика для группировки цен
    
    // Буферы данных
    volume_bars: ArrayVec<Bar, 512>,
    price_levels: HashMap<i64, PriceLevel>, // price_key -> level
    
    // Метрики дисбаланса
    total_imbalance: f64,
    avg_imbalance: f64,
    max_imbalance: f64,
    min_imbalance: f64,
    
    // Статистика
    dominant_side: i8, // 1 = buy, -1 = sell, 0 = neutral
    imbalance_strength: f64,
    flow_acceleration: f64,
    prev_imbalance: f64,
    
    // Ключевые уровни
    max_buy_level: Option<PriceLevel>,
    max_sell_level: Option<PriceLevel>,
    strongest_imbalance_level: Option<PriceLevel>,
}

impl OrderFlowImbalance {
    pub fn new(period: usize, tick_size: f64) -> Self {
        Self {
            period,
            tick_size,
            volume_bars: ArrayVec::new(),
            price_levels: HashMap::new(),
            total_imbalance: 0.0,
            avg_imbalance: 0.0,
            max_imbalance: 0.0,
            min_imbalance: 0.0,
            dominant_side: 0,
            imbalance_strength: 0.0,
            flow_acceleration: 0.0,
            prev_imbalance: 0.0,
            max_buy_level: None,
            max_sell_level: None,
            strongest_imbalance_level: None,
        }
    }
    
    /// Обновить анализатор новым Bar
    pub fn update_volume_bar(&mut self, volume_bar: &Bar) -> f64 {
        // Добавляем в буфер
        if self.volume_bars.len() >= self.period {
            self.volume_bars.remove(0);
        }
        self.volume_bars.push(*volume_bar);
        
        // Анализируем ценовые уровни
        self.analyze_price_levels(volume_bar);
        
        // Пересчитываем метрики
        self.recalculate_metrics();
        
        self.total_imbalance
    }
    
    /// Анализировать ценовые уровни в баре
    fn analyze_price_levels(&mut self, volume_bar: &Bar) {
        // Очищаем старые уровни если буфер полный
        if self.volume_bars.len() >= self.period {
            self.price_levels.clear();
            
            // Клонируем бары для избежания проблем с заимствованием
            let bars_to_process = self.volume_bars.clone();
            for bar in &bars_to_process {
                self.process_bar_levels(bar);
            }
        } else {
            self.process_bar_levels(volume_bar);
        }
    }
    
    /// Обработать ценовые уровни одного бара
    fn process_bar_levels(&mut self, volume_bar: &Bar) {
        // Определяем ключевые цены в баре
        let prices = [volume_bar.open, volume_bar.high, volume_bar.low, volume_bar.close];
        let volume_per_price = volume_bar.volume / 4.0;
        
        for price in &prices {
            let price_key = self.price_to_key(*price);
            
            // Определяем направление для каждой цены
            let is_buy = if let (Some(buy_vol), Some(sell_vol)) = (None::<f64>, None::<f64>) {
                // Если есть готовые данные о buy/sell volume
                buy_vol > sell_vol
            } else {
                // Иначе используем price action
                *price >= (volume_bar.open + volume_bar.close) / 2.0
            };
            
            // Обновляем ценовой уровень
            let level = self.price_levels.entry(price_key).or_insert_with(|| PriceLevel::new(*price));
            level.add_volume(volume_per_price, is_buy);
        }
    }
    
    /// Конвертировать цену в ключ для группировки
    fn price_to_key(&self, price: f64) -> i64 {
        (price / self.tick_size).round() as i64
    }
    
    /// Пересчитать все метрики
    fn recalculate_metrics(&mut self) {
        if self.price_levels.is_empty() {
            return;
        }
        
        let mut total_imbalance = 0.0;
        let mut max_imbalance = f64::NEG_INFINITY;
        let mut min_imbalance = f64::INFINITY;
        let mut max_buy_volume = 0.0;
        let mut max_sell_volume = 0.0;
        let mut strongest_imbalance = 0.0;
        
        self.max_buy_level = None;
        self.max_sell_level = None;
        self.strongest_imbalance_level = None;
        
        for level in self.price_levels.values() {
            total_imbalance += level.imbalance;
            
            if level.imbalance > max_imbalance {
                max_imbalance = level.imbalance;
            }
            if level.imbalance < min_imbalance {
                min_imbalance = level.imbalance;
            }
            
            // Находим уровни с максимальными объемами
            if level.buy_volume > max_buy_volume {
                max_buy_volume = level.buy_volume;
                self.max_buy_level = Some(level.clone());
            }
            
            if level.sell_volume > max_sell_volume {
                max_sell_volume = level.sell_volume;
                self.max_sell_level = Some(level.clone());
            }
            
            // Находим уровень с сильнейшим дисбалансом
            if level.imbalance.abs() > strongest_imbalance {
                strongest_imbalance = level.imbalance.abs();
                self.strongest_imbalance_level = Some(level.clone());
            }
        }
        
        self.total_imbalance = total_imbalance;
        self.avg_imbalance = total_imbalance / self.price_levels.len() as f64;
        self.max_imbalance = max_imbalance;
        self.min_imbalance = min_imbalance;
        
        // Определяем доминирующую сторону
        self.dominant_side = if self.avg_imbalance > 0.1 {
            1 // Buy dominant
        } else if self.avg_imbalance < -0.1 {
            -1 // Sell dominant
        } else {
            0 // Balanced
        };
        
        // Сила дисбаланса
        self.imbalance_strength = self.avg_imbalance.abs();
        
        // Ускорение flow
        self.flow_acceleration = self.avg_imbalance - self.prev_imbalance;
        self.prev_imbalance = self.avg_imbalance;
    }
    
    /// Получить общий дисбаланс
    pub fn total_imbalance(&self) -> f64 {
        self.total_imbalance
    }
    
    /// Получить средний дисбаланс
    pub fn avg_imbalance(&self) -> f64 {
        self.avg_imbalance
    }
    
    /// Получить доминирующую сторону
    pub fn dominant_side(&self) -> i8 {
        self.dominant_side
    }
    
    /// Получить силу дисбаланса
    pub fn imbalance_strength(&self) -> f64 {
        self.imbalance_strength
    }
    
    /// Получить ускорение flow
    pub fn flow_acceleration(&self) -> f64 {
        self.flow_acceleration
    }
    
    /// Получить уровень с максимальным buy volume
    pub fn max_buy_level(&self) -> Option<&PriceLevel> {
        self.max_buy_level.as_ref()
    }
    
    /// Получить уровень с максимальным sell volume
    pub fn max_sell_level(&self) -> Option<&PriceLevel> {
        self.max_sell_level.as_ref()
    }
    
    /// Получить уровень с сильнейшим дисбалансом
    pub fn strongest_imbalance_level(&self) -> Option<&PriceLevel> {
        self.strongest_imbalance_level.as_ref()
    }
    
    /// Получить количество ценовых уровней
    pub fn price_levels_count(&self) -> usize {
        self.price_levels.len()
    }
    
    /// Определить состояние order flow
    pub fn flow_state(&self) -> &'static str {
        match (self.dominant_side, self.imbalance_strength) {
            (1, s) if s > 0.5 => "Strong Buy Flow",
            (1, s) if s > 0.2 => "Moderate Buy Flow",
            (-1, s) if s > 0.5 => "Strong Sell Flow",
            (-1, s) if s > 0.2 => "Moderate Sell Flow",
            (0, s) if s < 0.1 => "Balanced Flow",
            _ => "Weak Flow",
        }
    }
    
    /// Получить качество анализа
    pub fn analysis_quality(&self) -> &'static str {
        let level_count = self.price_levels.len();
        match level_count {
            n if n >= 20 => "Excellent",
            n if n >= 10 => "Good",
            n if n >= 5 => "Fair",
            _ => "Poor",
        }
    }
    
    /// Проверить готовность анализатора
    pub fn is_ready(&self) -> bool {
        self.volume_bars.len() >= (self.period / 2).max(1) && !self.price_levels.is_empty()
    }

    /// Update with OHLCV bar
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> IndicatorValue {
        let bar = Bar {
            time: 0,
            open,
            high,
            low,
            close,
            volume,
        };
        self.update_volume_bar(&bar);
        self.value()
    }

    /// Получить значение как IndicatorValue
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.total_imbalance)
    }
    
    /// Сбросить анализатор
    pub fn reset(&mut self) {
        self.volume_bars.clear();
        self.price_levels.clear();
        self.total_imbalance = 0.0;
        self.avg_imbalance = 0.0;
        self.max_imbalance = 0.0;
        self.min_imbalance = 0.0;
        self.dominant_side = 0;
        self.imbalance_strength = 0.0;
        self.flow_acceleration = 0.0;
        self.prev_imbalance = 0.0;
        self.max_buy_level = None;
        self.max_sell_level = None;
        self.strongest_imbalance_level = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_flow_imbalance_creation() {
        let ind = OrderFlowImbalance::new(20, 0.01);
        assert!(!ind.is_ready());
        assert_eq!(ind.total_imbalance(), 0.0);
    }

    #[test]
    fn test_order_flow_imbalance_warmup() {
        let mut ind = OrderFlowImbalance::new(10, 0.01);
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
    fn test_order_flow_imbalance_values() {
        let mut ind = OrderFlowImbalance::new(10, 0.01);
        for i in 0..20 {
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
        assert!(ind.total_imbalance().is_finite());
        let side = ind.dominant_side();
        assert!(side >= -1 && side <= 1);
    }

    #[test]
    fn test_order_flow_imbalance_reset() {
        let mut ind = OrderFlowImbalance::new(10, 0.01);
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
        assert_eq!(ind.total_imbalance(), 0.0);
    }
} 






















