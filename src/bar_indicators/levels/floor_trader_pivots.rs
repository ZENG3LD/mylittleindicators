//! Floor Trader Pivots - классические уровни пивот трейдеров
//! Основаны на данных предыдущего периода (обычно дневных данных)
//! PP = (High + Low + Close) / 3
//! R1 = 2 * PP - Low
//! R2 = PP + (High - Low)
//! R3 = High + 2 * (PP - Low)
//! S1 = 2 * PP - High
//! S2 = PP - (High - Low)
//! S3 = Low - 2 * (High - PP)

use arrayvec::ArrayVec;
use std::collections::HashMap;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Уровни Floor Trader пивот трейдеров
#[derive(Debug, Clone)]
pub struct FloorTraderPivotLevels {
    pub pivot_point: f64,
    pub resistance_1: f64,
    pub resistance_2: f64,
    pub resistance_3: f64,
    pub support_1: f64,
    pub support_2: f64,
    pub support_3: f64,
}

impl FloorTraderPivotLevels {
    /// Создать новые уровни пивот
    pub fn new(high: f64, low: f64, close: f64) -> Self {
        let pivot_point = (high + low + close) / 3.0;
        let range = high - low;
        
        Self {
            pivot_point,
            resistance_1: 2.0 * pivot_point - low,
            resistance_2: pivot_point + range,
            resistance_3: high + 2.0 * (pivot_point - low),
            support_1: 2.0 * pivot_point - high,
            support_2: pivot_point - range,
            support_3: low - 2.0 * (high - pivot_point),
        }
    }
    
    /// Получить все уровни как вектор
    pub fn all_levels(&self) -> Vec<f64> {
        vec![
            self.support_3,
            self.support_2,
            self.support_1,
            self.pivot_point,
            self.resistance_1,
            self.resistance_2,
            self.resistance_3,
        ]
    }
    
    /// Получить ближайший уровень к заданной цене
    pub fn nearest_level(&self, price: f64) -> f64 {
        self.all_levels()
            .into_iter()
            .min_by(|a, b| (a - price).abs().partial_cmp(&(b - price).abs()).unwrap())
            .unwrap_or(self.pivot_point)
    }
    
    /// Получить расстояние до ближайшего уровня
    pub fn distance_to_nearest(&self, price: f64) -> f64 {
        let nearest = self.nearest_level(price);
        (price - nearest).abs()
    }
    
    /// Определить, между какими уровнями находится цена
    pub fn price_zone(&self, price: f64) -> &'static str {
        match price {
            p if p >= self.resistance_3 => "Above R3",
            p if p >= self.resistance_2 => "R2-R3",
            p if p >= self.resistance_1 => "R1-R2",
            p if p >= self.pivot_point => "PP-R1",
            p if p >= self.support_1 => "S1-PP",
            p if p >= self.support_2 => "S2-S1",
            p if p >= self.support_3 => "S3-S2",
            _ => "Below S3",
        }
    }
}

/// Floor Trader Pivots индикатор
#[derive(Clone)]
pub struct FloorTraderPivots {
    // Текущие уровни пивот
    current_levels: Option<FloorTraderPivotLevels>,
    
    // Период для обновления (количество баров)
    update_period: usize,
    
    // Буферы для расчетов
    period_high: f64,
    period_low: f64,
    period_close: f64,
    
    // Счетчики
    bars_in_period: usize,
    
    // История уровней
    levels_history: ArrayVec<FloorTraderPivotLevels, 100>,
    
    // Статистика взаимодействий с уровнями
    level_touches: HashMap<String, u32>,
    
    // Состояние
    is_ready: bool,
}

impl FloorTraderPivots {
    /// Создать новый индикатор с дневными пивотами (по умолчанию)
    pub fn new() -> Self {
        Self::with_period(24) // 24 часа для дневных пивотов
    }
    
    /// Создать новый индикатор с заданным периодом
    pub fn with_period(period: usize) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        
        Self {
            current_levels: None,
            update_period: period,
            period_high: f64::NEG_INFINITY,
            period_low: f64::INFINITY,
            period_close: 0.0,
            bars_in_period: 0,
            levels_history: ArrayVec::new(),
            level_touches: HashMap::new(),
            is_ready: false,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> Option<FloorTraderPivotLevels> {
        // Обновляем данные для текущего периода
        self.period_high = self.period_high.max(high);
        self.period_low = self.period_low.min(low);
        self.period_close = close;
        self.bars_in_period += 1;
        
        // Проверяем касания уровней
        if let Some(levels) = self.current_levels.clone() {
            self.check_level_touches(high, low, &levels);
        }
        
        // Проверяем, нужно ли обновить пивот уровни
        if self.bars_in_period >= self.update_period {
            // Рассчитываем новые уровни
            let new_levels = FloorTraderPivotLevels::new(self.period_high, self.period_low, self.period_close);
            
            // Сохраняем в истории
            if self.levels_history.len() >= 100 {
                self.levels_history.remove(0);
            }
            self.levels_history.push(new_levels.clone());
            
            // Обновляем текущие уровни
            self.current_levels = Some(new_levels);
            
            // Сбрасываем данные периода
            self.period_high = f64::NEG_INFINITY;
            self.period_low = f64::INFINITY;
            self.period_close = 0.0;
            self.bars_in_period = 0;
            
            self.is_ready = true;
        }
        
        self.current_levels.clone()
    }
    
    /// Проверить касания уровней
    fn check_level_touches(&mut self, high: f64, low: f64, levels: &FloorTraderPivotLevels) {
        let tolerance = 0.001; // Допуск для касания уровня
        
        let level_names = vec![
            ("S3", levels.support_3),
            ("S2", levels.support_2),
            ("S1", levels.support_1),
            ("PP", levels.pivot_point),
            ("R1", levels.resistance_1),
            ("R2", levels.resistance_2),
            ("R3", levels.resistance_3),
        ];
        
        for (name, level) in level_names {
            if low <= level * (1.0 + tolerance) && high >= level * (1.0 - tolerance) {
                *self.level_touches.entry(name.to_string()).or_insert(0) += 1;
            }
        }
    }
    
    /// Получить текущие уровни пивот
    pub fn current_levels(&self) -> Option<&FloorTraderPivotLevels> {
        self.current_levels.as_ref()
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить период обновления
    pub fn period(&self) -> usize {
        self.update_period
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.current_levels = None;
        self.period_high = f64::NEG_INFINITY;
        self.period_low = f64::INFINITY;
        self.period_close = 0.0;
        self.bars_in_period = 0;
        self.levels_history.clear();
        self.level_touches.clear();
        self.is_ready = false;
    }
    
    /// Получить торговый сигнал на основе взаимодействия с уровнями
    pub fn trading_signal(&self, current_price: f64) -> i8 {
        if let Some(levels) = &self.current_levels {
            let tolerance = 0.002; // 0.2% допуск
            
            // Покупка при отскоке от поддержки
            if (current_price - levels.support_1).abs() <= levels.support_1 * tolerance ||
               (current_price - levels.support_2).abs() <= levels.support_2 * tolerance ||
               (current_price - levels.support_3).abs() <= levels.support_3 * tolerance {
                return 1;
            }
            
            // Продажа при отскоке от сопротивления
            if (current_price - levels.resistance_1).abs() <= levels.resistance_1 * tolerance ||
               (current_price - levels.resistance_2).abs() <= levels.resistance_2 * tolerance ||
               (current_price - levels.resistance_3).abs() <= levels.resistance_3 * tolerance {
                return -1;
            }
            
            // Покупка при пробое сопротивления
            if current_price > levels.resistance_1 * 1.001 {
                return 1;
            }
            
            // Продажа при пробое поддержки
            if current_price < levels.support_1 * 0.999 {
                return -1;
            }
        }
        
        0
    }
    
    /// Получить силу уровня (количество касаний)
    pub fn level_strength(&self, level_name: &str) -> u32 {
        self.level_touches.get(level_name).copied().unwrap_or(0)
    }
    
    /// Получить все касания уровней
    pub fn all_level_touches(&self) -> &HashMap<String, u32> {
        &self.level_touches
    }
    
    /// Получить самый сильный уровень
    pub fn strongest_level(&self) -> Option<String> {
        self.level_touches
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(name, _)| name.clone())
    }
    
    /// Получить историю уровней
    pub fn levels_history(&self) -> &[FloorTraderPivotLevels] {
        &self.levels_history
    }
    
    /// Получить информацию о текущем состоянии
    pub fn info(&self, current_price: f64) -> String {
        if let Some(levels) = &self.current_levels {
            format!(
                "FTP - PP: {:.2}, Zone: {}, Nearest: {:.2}, Distance: {:.2}",
                levels.pivot_point,
                levels.price_zone(current_price),
                levels.nearest_level(current_price),
                levels.distance_to_nearest(current_price)
            )
        } else {
            "FTP - Not Ready".to_string()
        }
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(
            self.current_levels
                .as_ref()
                .map(|l| l.pivot_point)
                .unwrap_or(0.0)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_floor_trader_pivots_creation() {
        let ftp = FloorTraderPivots::new();
        assert!(!ftp.is_ready());
        assert!(ftp.current_levels().is_none());
    }

    #[test]
    fn test_floor_trader_pivot_levels() {
        let levels = FloorTraderPivotLevels::new(105.0, 95.0, 100.0);
        // PP = (105 + 95 + 100) / 3 = 100
        assert!((levels.pivot_point - 100.0).abs() < 0.001);
        // R1 = 2 * 100 - 95 = 105
        assert!((levels.resistance_1 - 105.0).abs() < 0.001);
        // S1 = 2 * 100 - 105 = 95
        assert!((levels.support_1 - 95.0).abs() < 0.001);
    }

    #[test]
    fn test_floor_trader_pivots_warmup() {
        let mut ftp = FloorTraderPivots::with_period(5);
        for i in 0..6 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ftp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ftp.is_ready());
    }

    #[test]
    fn test_floor_trader_pivots_reset() {
        let mut ftp = FloorTraderPivots::with_period(5);
        for i in 0..6 {
            ftp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0, 1000.0);
        }
        ftp.reset();
        assert!(!ftp.is_ready());
        assert!(ftp.current_levels().is_none());
    }
}






















