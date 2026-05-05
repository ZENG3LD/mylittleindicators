//! Camarilla Pivots - уровни Камарилла
//! Разработаны Nick Stott, основаны на внутридневной торговле
//! Используют коэффициенты для более точного определения уровней:
//! PP = (High + Low + Close) / 3
//! R1 = Close + (High - Low) * 1.1/12
//! R2 = Close + (High - Low) * 1.1/6
//! R3 = Close + (High - Low) * 1.1/4
//! R4 = Close + (High - Low) * 1.1/2
//! S1 = Close - (High - Low) * 1.1/12
//! S2 = Close - (High - Low) * 1.1/6
//! S3 = Close - (High - Low) * 1.1/4
//! S4 = Close - (High - Low) * 1.1/2

use arrayvec::ArrayVec;
use std::collections::HashMap;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Уровни Камарилла
#[derive(Debug, Clone)]
pub struct CamarillaPivotLevels {
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

impl CamarillaPivotLevels {
    /// Создать новые уровни Камарилла
    pub fn new(high: f64, low: f64, close: f64) -> Self {
        let pivot_point = (high + low + close) / 3.0;
        let range = high - low;
        let multiplier = 1.1;
        
        Self {
            pivot_point,
            resistance_1: close + range * multiplier / 12.0,
            resistance_2: close + range * multiplier / 6.0,
            resistance_3: close + range * multiplier / 4.0,
            resistance_4: close + range * multiplier / 2.0,
            support_1: close - range * multiplier / 12.0,
            support_2: close - range * multiplier / 6.0,
            support_3: close - range * multiplier / 4.0,
            support_4: close - range * multiplier / 2.0,
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
    
    /// Получить силу уровня (обратное расстояние от цены)
    pub fn level_strength(&self, level_name: &str, price: f64) -> f64 {
        if let Some(level) = self.level_by_name(level_name) {
            let distance = (price - level).abs();
            if distance < 1e-6 {
                f64::MAX
            } else {
                1.0 / distance
            }
        } else {
            0.0
        }
    }
}

/// Camarilla Pivots индикатор
#[derive(Clone)]
pub struct CamarillaPivots {
    // Текущие уровни пивот
    current_levels: Option<CamarillaPivotLevels>,
    
    // Период для обновления (количество баров)
    update_period: usize,
    
    // Буферы для расчетов
    period_high: f64,
    period_low: f64,
    period_close: f64,
    
    // Счетчики
    bars_in_period: usize,
    
    // История уровней
    levels_history: ArrayVec<CamarillaPivotLevels, 100>,
    
    // Статистика взаимодействий с уровнями
    level_touches: HashMap<String, u32>,
    level_breaks: HashMap<String, u32>,
    
    // Состояние
    is_ready: bool,
}

impl CamarillaPivots {
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
            level_breaks: HashMap::new(),
            is_ready: false,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> Option<CamarillaPivotLevels> {
        // Обновляем данные для текущего периода
        self.period_high = self.period_high.max(high);
        self.period_low = self.period_low.min(low);
        self.period_close = close;
        self.bars_in_period += 1;
        
        // Проверяем касания и пробои уровней
        if let Some(levels) = self.current_levels.clone() {
            self.check_level_interactions(high, low, close, &levels);
        }
        
        // Проверяем, нужно ли обновить пивот уровни
        if self.bars_in_period >= self.update_period {
            // Рассчитываем новые уровни
            let new_levels = CamarillaPivotLevels::new(self.period_high, self.period_low, self.period_close);
            
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
    
    /// Проверить касания и пробои уровней
    fn check_level_interactions(&mut self, high: f64, low: f64, close: f64, levels: &CamarillaPivotLevels) {
        let tolerance = 0.0005; // 0.05% допуск для касания уровня
        
        let level_names = vec![
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
        
        for (name, level) in level_names {
            // Проверяем касания
            if low <= level * (1.0 + tolerance) && high >= level * (1.0 - tolerance) {
                *self.level_touches.entry(name.to_string()).or_insert(0) += 1;
            }
            
            // Проверяем пробои
            if close > level * (1.0 + tolerance) || close < level * (1.0 - tolerance) {
                *self.level_breaks.entry(name.to_string()).or_insert(0) += 1;
            }
        }
    }
    
    /// Получить текущие уровни пивот
    pub fn current_levels(&self) -> Option<&CamarillaPivotLevels> {
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
        self.level_breaks.clear();
        self.is_ready = false;
    }
    
    /// Получить торговый сигнал на основе стратегии Камарилла
    pub fn trading_signal(&self, current_price: f64) -> i8 {
        if let Some(levels) = &self.current_levels {
            let tolerance = 0.001; // 0.1% допуск
            
            // Стратегия Камарилла: торговля на возврат к центру
            // Покупка при касании S3 или S4 (ожидаем возврат к центру)
            if (current_price - levels.support_3).abs() <= levels.support_3 * tolerance ||
               (current_price - levels.support_4).abs() <= levels.support_4 * tolerance {
                return 1;
            }
            
            // Продажа при касании R3 или R4 (ожидаем возврат к центру)
            if (current_price - levels.resistance_3).abs() <= levels.resistance_3 * tolerance ||
               (current_price - levels.resistance_4).abs() <= levels.resistance_4 * tolerance {
                return -1;
            }
            
            // Пробойная стратегия: покупка при пробое R3, продажа при пробое S3
            if current_price > levels.resistance_3 * (1.0 + tolerance) {
                return 1;
            }
            
            if current_price < levels.support_3 * (1.0 - tolerance) {
                return -1;
            }
        }
        
        0
    }
    
    /// Получить продвинутый сигнал с учетом истории взаимодействий
    pub fn advanced_signal(&self, current_price: f64) -> i8 {
        if let Some(levels) = &self.current_levels {
            let zone = levels.price_zone(current_price);
            
            // Анализ зон для более точных сигналов
            match zone {
                "S4-S3" | "Below S4" => {
                    // Глубокая поддержка - покупка с высокой вероятностью
                    if self.level_strength("S4") > 2 || self.level_strength("S3") > 2 {
                        return 1;
                    }
                }
                "R3-R4" | "Above R4" => {
                    // Сильное сопротивление - продажа с высокой вероятностью
                    if self.level_strength("R4") > 2 || self.level_strength("R3") > 2 {
                        return -1;
                    }
                }
                "S2-S1" => {
                    // Умеренная поддержка - покупка при подтверждении
                    if self.level_strength("S2") > 1 {
                        return 1;
                    }
                }
                "R1-R2" => {
                    // Умеренное сопротивление - продажа при подтверждении
                    if self.level_strength("R2") > 1 {
                        return -1;
                    }
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
    
    /// Получить количество пробоев уровня
    pub fn level_breaks(&self, level_name: &str) -> u32 {
        self.level_breaks.get(level_name).copied().unwrap_or(0)
    }
    
    /// Получить все касания уровней
    pub fn all_level_touches(&self) -> &HashMap<String, u32> {
        &self.level_touches
    }
    
    /// Получить все пробои уровней
    pub fn all_level_breaks(&self) -> &HashMap<String, u32> {
        &self.level_breaks
    }
    
    /// Получить самый сильный уровень
    pub fn strongest_level(&self) -> Option<String> {
        self.level_touches
            .iter()
            .max_by_key(|(_, &count)| count)
            .map(|(name, _)| name.clone())
    }
    
    /// Получить историю уровней
    pub fn levels_history(&self) -> &[CamarillaPivotLevels] {
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
    
    /// Получить информацию о текущем состоянии
    pub fn info(&self, current_price: f64) -> String {
        if let Some(levels) = &self.current_levels {
            format!(
                "Camarilla - PP: {:.2}, Zone: {}, Nearest: {:.2}, Distance: {:.4}, Volatility: {:.2}",
                levels.pivot_point,
                levels.price_zone(current_price),
                levels.nearest_level(current_price),
                levels.distance_to_nearest(current_price),
                self.market_volatility()
            )
        } else {
            "Camarilla - Not Ready".to_string()
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






















