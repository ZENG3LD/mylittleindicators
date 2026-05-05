//! Classic Pivot Points - классические пивот уровни для определения поддержки и сопротивления
//! Pivot Point (PP) = (High + Low + Close) / 3
//! Support 1 (S1) = (2 × PP) - High
//! Support 2 (S2) = PP - (High - Low)
//! Support 3 (S3) = Low - 2 × (High - PP)
//! Resistance 1 (R1) = (2 × PP) - Low
//! Resistance 2 (R2) = PP + (High - Low)
//! Resistance 3 (R3) = High + 2 × (PP - Low)

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Уровни Classic Pivot Points
#[derive(Debug, Clone, Copy)]
pub struct ClassicPivotLevels {
    pub pivot: f64,       // Основной пивот уровень
    pub resistance_1: f64, // Первое сопротивление (R1)
    pub resistance_2: f64, // Второе сопротивление (R2) 
    pub resistance_3: f64, // Третье сопротивление (R3)
    pub support_1: f64,    // Первая поддержка (S1)
    pub support_2: f64,    // Вторая поддержка (S2)
    pub support_3: f64,    // Третья поддержка (S3)
}

impl ClassicPivotLevels {
    /// Создать пустые уровни
    pub fn empty() -> Self {
        Self {
            pivot: 0.0,
            resistance_1: 0.0,
            resistance_2: 0.0,
            resistance_3: 0.0,
            support_1: 0.0,
            support_2: 0.0,
            support_3: 0.0,
        }
    }
    
    /// Получить все уровни сопротивления отсортированные по возрастанию
    pub fn resistance_levels(&self) -> [f64; 3] {
        [self.resistance_1, self.resistance_2, self.resistance_3]
    }
    
    /// Получить все уровни поддержки отсортированные по убыванию
    pub fn support_levels(&self) -> [f64; 3] {
        [self.support_1, self.support_2, self.support_3]
    }
    
    /// Получить все уровни включая пивот
    pub fn all_levels(&self) -> [f64; 7] {
        [
            self.support_3,
            self.support_2, 
            self.support_1,
            self.pivot,
            self.resistance_1,
            self.resistance_2,
            self.resistance_3,
        ]
    }
}

/// Classic Pivot Points индикатор
#[derive(Clone)]
pub struct PivotPoints {
    // Текущие уровни
    current_levels: ClassicPivotLevels,
    
    // История уровней
    levels_history: ArrayVec<ClassicPivotLevels, 100>,
    
    // Буферы для расчета (для периодических обновлений)
    highs: ArrayVec<f64, 32>,
    lows: ArrayVec<f64, 32>,
    closes: ArrayVec<f64, 32>,
    
    // Настройки
    calculation_period: usize, // Период для расчета (1 = ежедневно, 7 = еженедельно и т.д.)
    bars_since_update: usize,
    
    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl PivotPoints {
    /// Создать новый индикатор Pivot Points (ежедневное обновление)
    pub fn new() -> Self {
        Self::with_period(1)
    }
    
    /// Создать новый индикатор с настраиваемым периодом обновления
    /// period = 1 (ежедневно), 7 (еженедельно), и т.д.
    pub fn with_period(calculation_period: usize) -> Self {
        assert!(calculation_period > 0, "Period must be greater than 0");
        
        Self {
            current_levels: ClassicPivotLevels::empty(),
            levels_history: ArrayVec::new(),
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            closes: ArrayVec::new(),
            calculation_period,
            bars_since_update: 0,
            is_ready: false,
            update_count: 0,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> ClassicPivotLevels {
        // Добавляем данные в буферы
        self.highs.push(high);
        self.lows.push(low);
        self.closes.push(close);
        
        self.bars_since_update += 1;
        
        // Проверяем, нужно ли пересчитать уровни
        if self.bars_since_update >= self.calculation_period {
            self.calculate_levels();
            self.bars_since_update = 0;
        }
        
        self.current_levels
    }
    
    /// Принудительно пересчитать уровни на основе накопленных данных
    pub fn calculate_levels(&mut self) {
        if self.highs.is_empty() {
            return;
        }
        
        // Находим максимум, минимум и последнее закрытие за период
        let period_high = self.highs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let period_low = self.lows.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let period_close = self.closes[self.closes.len() - 1];
        
        // Рассчитываем пивот точку
        let pivot = (period_high + period_low + period_close) / 3.0;
        
        // Рассчитываем уровни сопротивления
        let resistance_1 = (2.0 * pivot) - period_low;
        let resistance_2 = pivot + (period_high - period_low);
        let resistance_3 = period_high + 2.0 * (pivot - period_low);
        
        // Рассчитываем уровни поддержки
        let support_1 = (2.0 * pivot) - period_high;
        let support_2 = pivot - (period_high - period_low);
        let support_3 = period_low - 2.0 * (period_high - pivot);
        
        // Создаем новые уровни
        let new_levels = ClassicPivotLevels {
            pivot,
            resistance_1,
            resistance_2,
            resistance_3,
            support_1,
            support_2,
            support_3,
        };
        
        // Сохраняем в историю
        if self.levels_history.len() >= 100 {
            self.levels_history.remove(0);
        }
        self.levels_history.push(self.current_levels);
        
        // Обновляем текущие уровни
        self.current_levels = new_levels;
        
        // Очищаем буферы для следующего периода
        self.highs.clear();
        self.lows.clear();
        self.closes.clear();
        
        self.update_count += 1;
        self.is_ready = true;
    }
    
    /// Получить текущие уровни Pivot Points
    pub fn levels(&self) -> ClassicPivotLevels {
        self.current_levels
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.current_levels = ClassicPivotLevels::empty();
        self.levels_history.clear();
        self.highs.clear();
        self.lows.clear();
        self.closes.clear();
        self.bars_since_update = 0;
        self.is_ready = false;
        self.update_count = 0;
    }
    
    /// Определить ближайший уровень поддержки для текущей цены
    pub fn nearest_support(&self, current_price: f64) -> f64 {
        let supports = [
            self.current_levels.support_1,
            self.current_levels.support_2,
            self.current_levels.support_3,
        ];
        
        // Находим ближайший уровень поддержки ниже текущей цены
        supports
            .iter()
            .filter(|&&level| level < current_price)
            .fold(f64::NEG_INFINITY, |acc, &level| acc.max(level))
    }
    
    /// Определить ближайший уровень сопротивления для текущей цены
    pub fn nearest_resistance(&self, current_price: f64) -> f64 {
        let resistances = [
            self.current_levels.resistance_1,
            self.current_levels.resistance_2,
            self.current_levels.resistance_3,
        ];
        
        // Находим ближайший уровень сопротивления выше текущей цены
        resistances
            .iter()
            .filter(|&&level| level > current_price)
            .fold(f64::INFINITY, |acc, &level| acc.min(level))
    }
    
    /// Определить текущую позицию цены относительно пивота
    /// 1 = выше пивота (бычий), -1 = ниже пивота (медвежий), 0 = на пивоте
    pub fn price_position(&self, current_price: f64) -> i8 {
        let pivot = self.current_levels.pivot;
        let threshold = pivot * 0.001; // 0.1% порог
        
        if current_price > pivot + threshold {
            1  // Выше пивота
        } else if current_price < pivot - threshold {
            -1 // Ниже пивота
        } else {
            0  // На пивоте
        }
    }
    
    /// Получить торговый сигнал на основе пересечения уровней
    /// 1 = покупка, -1 = продажа, 0 = нейтрально
    pub fn trading_signal(&self, current_price: f64, prev_price: f64) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        // Проверяем пересечение уровней снизу вверх (покупка)
        let levels = self.current_levels.all_levels();
        for &level in &levels {
            if prev_price <= level && current_price > level {
                // Пересечение снизу вверх - потенциальная покупка
                if level >= self.current_levels.pivot {
                    return 1; // Пересечение пивота или сопротивления вверх
                }
            }
        }
        
        // Проверяем пересечение уровней сверху вниз (продажа)
        for &level in &levels {
            if prev_price >= level && current_price < level {
                // Пересечение сверху вниз - потенциальная продажа
                if level <= self.current_levels.pivot {
                    return -1; // Пересечение пивота или поддержки вниз
                }
            }
        }
        
        0
    }
    
    /// Рассчитать расстояние до ближайшего уровня в процентах
    pub fn distance_to_nearest_level(&self, current_price: f64) -> f64 {
        if current_price.abs() < 1e-12 {
            return 0.0;
        }
        
        let levels = self.current_levels.all_levels();
        let nearest_distance = levels
            .iter()
            .map(|&level| (level - current_price).abs())
            .fold(f64::INFINITY, |acc, dist| acc.min(dist));
        
        (nearest_distance / current_price) * 100.0
    }
    
    /// Определить силу уровня на основе истории
    /// Возвращает количество раз, когда цена отскакивала от уровня
    pub fn level_strength(&self, level: f64, price_history: &[f64], tolerance_pct: f64) -> u32 {
        if price_history.len() < 2 {
            return 0;
        }
        
        let tolerance = level * (tolerance_pct / 100.0);
        let mut touches = 0;
        
        for i in 1..price_history.len() {
            let prev_price = price_history[i - 1];
            let current_price = price_history[i];
            
            // Проверяем касание уровня (цена приближается и отскакивает)
            let near_level = (current_price - level).abs() <= tolerance;
            let moving_away = (current_price - level).abs() > (prev_price - level).abs();
            
            if near_level && moving_away {
                touches += 1;
            }
        }
        
        touches
    }
    
    /// Получить историю уровней для анализа
    pub fn levels_history(&self) -> Vec<ClassicPivotLevels> {
        self.levels_history.iter().cloned().collect()
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self, current_price: f64) -> String {
        format!(
            "PP: {:.4}, Position: {}, Nearest S: {:.4}, Nearest R: {:.4}, Distance: {:.2}%",
            self.current_levels.pivot,
            match self.price_position(current_price) {
                1 => "Above",
                -1 => "Below",
                _ => "At Pivot"
            },
            self.nearest_support(current_price),
            self.nearest_resistance(current_price),
            self.distance_to_nearest_level(current_price)
        )
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        // Return pivot, R1, S1 as Triple
        IndicatorValue::Triple(
            self.current_levels.resistance_1,
            self.current_levels.support_1,
            self.current_levels.pivot,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pivot_points_creation() {
        let pp = PivotPoints::new();
        assert!(!pp.is_ready());
    }

    #[test]
    fn test_classic_pivot_levels() {
        let levels = ClassicPivotLevels::empty();
        assert_eq!(levels.pivot, 0.0);
        assert_eq!(levels.resistance_1, 0.0);
        assert_eq!(levels.support_1, 0.0);
    }

    #[test]
    fn test_pivot_points_update() {
        let mut pp = PivotPoints::new();
        let levels = pp.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0);
        assert!(pp.is_ready());
        // PP = (105 + 95 + 102) / 3 = 100.67
        assert!((levels.pivot - 100.67).abs() < 0.1);
    }

    #[test]
    fn test_pivot_points_levels_order() {
        let mut pp = PivotPoints::new();
        let levels = pp.update_bar(100.0, 110.0, 90.0, 100.0, 1000.0);
        // Verify levels are in correct order
        assert!(levels.support_3 < levels.support_2);
        assert!(levels.support_2 < levels.support_1);
        assert!(levels.support_1 < levels.pivot);
        assert!(levels.pivot < levels.resistance_1);
        assert!(levels.resistance_1 < levels.resistance_2);
        assert!(levels.resistance_2 < levels.resistance_3);
    }

    #[test]
    fn test_pivot_points_reset() {
        let mut pp = PivotPoints::new();
        pp.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0);
        pp.reset();
        assert!(!pp.is_ready());
    }
}






















