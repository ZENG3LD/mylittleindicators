//! Woodie Pivots - уровни Вуди
//! Модификация классических пивотов, разработанная Ken Wood
//! Основное отличие: при расчете PP используется больший вес цены открытия
//! PP = (High + Low + 2 * Open) / 4  (вместо стандартного (H+L+C)/3)
//! R1 = 2 * PP - Low
//! R2 = PP + High - Low
//! R3 = High + 2 * (PP - Low)
//! R4 = R3 + (High - Low)
//! S1 = 2 * PP - High
//! S2 = PP - High + Low
//! S3 = Low - 2 * (High - PP)
//! S4 = S3 - (High - Low)

use arrayvec::ArrayVec;
use std::collections::HashMap;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Уровни Вуди
#[derive(Debug, Clone)]
pub struct WoodiePivotLevels {
    pub pivot_point: f64,
    pub resistance_1: f64,
    pub resistance_2: f64,
    pub resistance_3: f64,
    pub resistance_4: f64,
    pub support_1: f64,
    pub support_2: f64,
    pub support_3: f64,
    pub support_4: f64,
}

impl WoodiePivotLevels {
    /// Создать новые уровни Вуди
    pub fn new(open: f64, high: f64, low: f64, _close: f64) -> Self {
        // Вуди формула: PP = (High + Low + 2 * Open) / 4
        let pivot_point = (high + low + 2.0 * open) / 4.0;
        let range = high - low;
        
        Self {
            pivot_point,
            resistance_1: 2.0 * pivot_point - low,
            resistance_2: pivot_point + range,
            resistance_3: high + 2.0 * (pivot_point - low),
            resistance_4: high + 2.0 * (pivot_point - low) + range,
            support_1: 2.0 * pivot_point - high,
            support_2: pivot_point - range,
            support_3: low - 2.0 * (high - pivot_point),
            support_4: low - 2.0 * (high - pivot_point) - range,
        }
    }
    
    /// Получить все уровни как вектор (от самого низкого к самому высокому)
    pub fn all_levels(&self) -> Vec<f64> {
        vec![
            self.support_4,
            self.support_3,
            self.support_2,
            self.support_1,
            self.pivot_point,
            self.resistance_1,
            self.resistance_2,
            self.resistance_3,
            self.resistance_4,
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
            p if p >= self.resistance_4 => "Above R4",
            p if p >= self.resistance_3 => "R3-R4",
            p if p >= self.resistance_2 => "R2-R3",
            p if p >= self.resistance_1 => "R1-R2",
            p if p >= self.pivot_point => "PP-R1",
            p if p >= self.support_1 => "S1-PP",
            p if p >= self.support_2 => "S2-S1",
            p if p >= self.support_3 => "S3-S2",
            p if p >= self.support_4 => "S4-S3",
            _ => "Below S4",
        }
    }
    
    /// Получить уровень по имени
    pub fn level_by_name(&self, name: &str) -> Option<f64> {
        match name {
            "PP" => Some(self.pivot_point),
            "R1" => Some(self.resistance_1),
            "R2" => Some(self.resistance_2),
            "R3" => Some(self.resistance_3),
            "R4" => Some(self.resistance_4),
            "S1" => Some(self.support_1),
            "S2" => Some(self.support_2),
            "S3" => Some(self.support_3),
            "S4" => Some(self.support_4),
            _ => None,
        }
    }
    
    /// Получить все имена уровней
    pub fn level_names(&self) -> Vec<&'static str> {
        vec!["S4", "S3", "S2", "S1", "PP", "R1", "R2", "R3", "R4"]
    }
    
    /// Получить процентное расстояние от пивот поинта
    pub fn distance_from_pivot_pct(&self, price: f64) -> f64 {
        if self.pivot_point.abs() < 1e-12 {
            0.0
        } else {
            (price - self.pivot_point) / self.pivot_point * 100.0
        }
    }
}

/// Woodie Pivots индикатор
#[derive(Clone)]
pub struct WoodiePivots {
    // Текущие уровни пивот
    current_levels: Option<WoodiePivotLevels>,
    
    // Период для обновления (количество баров)
    update_period: usize,
    
    // Буферы для расчетов
    period_open: f64,
    period_high: f64,
    period_low: f64,
    period_close: f64,
    
    // Счетчики
    bars_in_period: usize,
    
    // История уровней
    levels_history: ArrayVec<WoodiePivotLevels, 100>,
    
    // Статистика взаимодействий с уровнями
    level_touches: HashMap<String, u32>,
    level_breaks: HashMap<String, u32>,
    level_reversals: HashMap<String, u32>,
    
    // Предыдущая цена для отслеживания разворотов
    prev_price: f64,
    
    // Состояние
    is_ready: bool,
}

impl WoodiePivots {
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
            period_open: 0.0,
            period_high: f64::NEG_INFINITY,
            period_low: f64::INFINITY,
            period_close: 0.0,
            bars_in_period: 0,
            levels_history: ArrayVec::new(),
            level_touches: HashMap::new(),
            level_breaks: HashMap::new(),
            level_reversals: HashMap::new(),
            prev_price: 0.0,
            is_ready: false,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, _volume: f64) -> Option<WoodiePivotLevels> {
        // Первый бар в периоде сохраняем цену открытия
        if self.bars_in_period == 0 {
            self.period_open = open;
        }
        
        // Обновляем данные для текущего периода
        self.period_high = self.period_high.max(high);
        self.period_low = self.period_low.min(low);
        self.period_close = close;
        self.bars_in_period += 1;
        
        // Проверяем взаимодействия с уровнями
        if let Some(levels) = self.current_levels.clone() {
            self.check_level_interactions(high, low, close, &levels);
        }
        
        // Обновляем предыдущую цену
        self.prev_price = close;
        
        // Проверяем, нужно ли обновить пивот уровни
        if self.bars_in_period >= self.update_period {
            // Рассчитываем новые уровни
            let new_levels = WoodiePivotLevels::new(self.period_open, self.period_high, self.period_low, self.period_close);
            
            // Сохраняем в истории
            if self.levels_history.len() >= 100 {
                self.levels_history.remove(0);
            }
            self.levels_history.push(new_levels.clone());
            
            // Обновляем текущие уровни
            self.current_levels = Some(new_levels);
            
            // Сбрасываем данные периода
            self.period_open = 0.0;
            self.period_high = f64::NEG_INFINITY;
            self.period_low = f64::INFINITY;
            self.period_close = 0.0;
            self.bars_in_period = 0;
            
            self.is_ready = true;
        }
        
        self.current_levels.clone()
    }
    
    /// Проверить взаимодействия с уровнями
    fn check_level_interactions(&mut self, high: f64, low: f64, close: f64, levels: &WoodiePivotLevels) {
        let tolerance = 0.0008; // 0.08% допуск для касания уровня
        
        let level_names_values = vec![
            ("S4", levels.support_4),
            ("S3", levels.support_3),
            ("S2", levels.support_2),
            ("S1", levels.support_1),
            ("PP", levels.pivot_point),
            ("R1", levels.resistance_1),
            ("R2", levels.resistance_2),
            ("R3", levels.resistance_3),
            ("R4", levels.resistance_4),
        ];
        
        for (name, level) in level_names_values {
            // Проверяем касания
            if low <= level * (1.0 + tolerance) && high >= level * (1.0 - tolerance) {
                *self.level_touches.entry(name.to_string()).or_insert(0) += 1;
            }
            
            // Проверяем пробои
            if close > level * (1.0 + tolerance) && self.prev_price <= level * (1.0 + tolerance) {
                *self.level_breaks.entry(format!("{}_break_up", name)).or_insert(0) += 1;
            } else if close < level * (1.0 - tolerance) && self.prev_price >= level * (1.0 - tolerance) {
                *self.level_breaks.entry(format!("{}_break_down", name)).or_insert(0) += 1;
            }
            
            // Проверяем развороты (касание уровня и откат)
            if (low <= level * (1.0 + tolerance) && close > level * (1.0 + tolerance)) ||
               (high >= level * (1.0 - tolerance) && close < level * (1.0 - tolerance)) {
                *self.level_reversals.entry(name.to_string()).or_insert(0) += 1;
            }
        }
    }
    
    /// Получить текущие уровни пивот
    pub fn current_levels(&self) -> Option<&WoodiePivotLevels> {
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
        self.period_open = 0.0;
        self.period_high = f64::NEG_INFINITY;
        self.period_low = f64::INFINITY;
        self.period_close = 0.0;
        self.bars_in_period = 0;
        self.levels_history.clear();
        self.level_touches.clear();
        self.level_breaks.clear();
        self.level_reversals.clear();
        self.prev_price = 0.0;
        self.is_ready = false;
    }
    
    /// Получить торговый сигнал на основе стратегии Вуди
    pub fn trading_signal(&self, current_price: f64) -> i8 {
        if let Some(levels) = &self.current_levels {
            let tolerance = 0.001; // 0.1% допуск
            
            // Стратегия Вуди: фокус на PP и первых уровнях
            // Покупка при отскоке от поддержки
            if (current_price - levels.support_1).abs() <= levels.support_1 * tolerance {
                return 1;
            }
            
            // Продажа при отскоке от сопротивления
            if (current_price - levels.resistance_1).abs() <= levels.resistance_1 * tolerance {
                return -1;
            }
            
            // Покупка при касании S2 (более сильная поддержка)
            if (current_price - levels.support_2).abs() <= levels.support_2 * tolerance {
                return 1;
            }
            
            // Продажа при касании R2 (более сильное сопротивление)
            if (current_price - levels.resistance_2).abs() <= levels.resistance_2 * tolerance {
                return -1;
            }
            
            // Пробойные сигналы
            if current_price > levels.resistance_1 * (1.0 + tolerance) {
                return 1;
            }
            
            if current_price < levels.support_1 * (1.0 - tolerance) {
                return -1;
            }
        }
        
        0
    }
    
    /// Получить продвинутый сигнал с учетом статистики уровней
    pub fn advanced_signal(&self, current_price: f64) -> i8 {
        if let Some(levels) = &self.current_levels {
            let zone = levels.price_zone(current_price);
            
            // Анализ силы уровней на основе истории
            match zone {
                "S1-PP" => {
                    // Зона между S1 и PP - обычно бычья
                    if self.level_strength("S1") > 2 {
                        return 1;
                    }
                }
                "PP-R1" => {
                    // Зона между PP и R1 - обычно медвежья
                    if self.level_strength("R1") > 2 {
                        return -1;
                    }
                }
                "S2-S1" => {
                    // Сильная поддержка
                    if self.level_reversals("S2") > 1 {
                        return 1;
                    }
                }
                "R1-R2" => {
                    // Сильное сопротивление
                    if self.level_reversals("R2") > 1 {
                        return -1;
                    }
                }
                "Below S4" => {
                    // Сверхпродажа - вероятен отскок
                    return 1;
                }
                "Above R4" => {
                    // Перекупленность - вероятен откат
                    return -1;
                }
                _ => {}
            }
        }
        
        0
    }
    
    /// Получить силу уровня (количество касаний)
    pub fn level_strength(&self, level_name: &str) -> u32 {
        self.level_touches.get(level_name).copied().unwrap_or(0)
    }
    
    /// Получить количество разворотов от уровня
    pub fn level_reversals(&self, level_name: &str) -> u32 {
        self.level_reversals.get(level_name).copied().unwrap_or(0)
    }
    
    /// Получить все касания уровней
    pub fn all_level_touches(&self) -> &HashMap<String, u32> {
        &self.level_touches
    }
    
    /// Получить все пробои уровней
    pub fn all_level_breaks(&self) -> &HashMap<String, u32> {
        &self.level_breaks
    }
    
    /// Получить все развороты от уровней
    pub fn all_level_reversals(&self) -> &HashMap<String, u32> {
        &self.level_reversals
    }
    
    /// Получить самый сильный уровень поддержки
    pub fn strongest_support(&self) -> Option<String> {
        let support_levels = ["S1", "S2", "S3", "S4"];
        support_levels
            .iter()
            .max_by_key(|&level| self.level_strength(level))
            .map(|&level| level.to_string())
    }
    
    /// Получить самый сильный уровень сопротивления
    pub fn strongest_resistance(&self) -> Option<String> {
        let resistance_levels = ["R1", "R2", "R3", "R4"];
        resistance_levels
            .iter()
            .max_by_key(|&level| self.level_strength(level))
            .map(|&level| level.to_string())
    }
    
    /// Получить историю уровней
    pub fn levels_history(&self) -> &[WoodiePivotLevels] {
        &self.levels_history
    }
    
    /// Получить рыночную волатильность на основе уровней
    pub fn market_volatility(&self) -> f64 {
        if let Some(levels) = &self.current_levels {
            levels.resistance_4 - levels.support_4
        } else {
            0.0
        }
    }
    
    /// Получить эффективность уровней (отношение разворотов к касаниям)
    pub fn level_effectiveness(&self, level_name: &str) -> f64 {
        let touches = self.level_strength(level_name) as f64;
        let reversals = self.level_reversals(level_name) as f64;
        
        if touches > 0.0 {
            reversals / touches
        } else {
            0.0
        }
    }
    
    /// Получить информацию о текущем состоянии
    pub fn info(&self, current_price: f64) -> String {
        if let Some(levels) = &self.current_levels {
            let distance_pct = levels.distance_from_pivot_pct(current_price);
            format!(
                "Woodie - PP: {:.2}, Zone: {}, Distance from PP: {:.2}%, Volatility: {:.2}",
                levels.pivot_point,
                levels.price_zone(current_price),
                distance_pct,
                self.market_volatility()
            )
        } else {
            "Woodie - Not Ready".to_string()
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
    fn test_woodie_pivots_creation() {
        let wp = WoodiePivots::new();
        assert!(!wp.is_ready());
        assert!(wp.current_levels().is_none());
    }

    #[test]
    fn test_woodie_pivot_levels() {
        // PP = (High + Low + 2 * Open) / 4 = (110 + 90 + 200) / 4 = 100
        let levels = WoodiePivotLevels::new(100.0, 110.0, 90.0, 105.0);
        assert!((levels.pivot_point - 100.0).abs() < 0.001);
    }

    #[test]
    fn test_woodie_pivots_warmup() {
        let mut wp = WoodiePivots::with_period(5);
        for i in 0..6 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            wp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(wp.is_ready());
    }

    #[test]
    fn test_woodie_pivots_levels_order() {
        let mut wp = WoodiePivots::with_period(1);
        let levels = wp.update_bar(100.0, 110.0, 90.0, 100.0, 1000.0).unwrap();
        // Verify levels are in correct order
        assert!(levels.support_4 < levels.support_3);
        assert!(levels.support_3 < levels.support_2);
        assert!(levels.support_2 < levels.support_1);
        assert!(levels.support_1 < levels.pivot_point);
        assert!(levels.pivot_point < levels.resistance_1);
        assert!(levels.resistance_1 < levels.resistance_2);
        assert!(levels.resistance_2 < levels.resistance_3);
        assert!(levels.resistance_3 < levels.resistance_4);
    }

    #[test]
    fn test_woodie_pivots_reset() {
        let mut wp = WoodiePivots::with_period(5);
        for i in 0..6 {
            wp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0, 1000.0);
        }
        wp.reset();
        assert!(!wp.is_ready());
        assert!(wp.current_levels().is_none());
    }
}






















