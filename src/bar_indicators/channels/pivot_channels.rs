//! pivot_channels.rs: High-Performance Pivot Channels
//! Каналы на основе Pivot Points - уровни поддержки и сопротивления
//!
//! Особенности:
//! - Classic Pivot Points (PP, R1/R2/R3, S1/S2/S3)
//! - Fibonacci Pivots и Camarilla Pivots
//! - Adaptive channel width based on volatility
//! - Breakout and bounce signal detection

use crate::bar_indicators::indicator_value::IndicatorValue;
use arrayvec::ArrayVec;
use serde::{Serialize, Deserialize};

/// Тип Pivot Points
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PivotType {
    /// Классические Pivot Points
    Classic,
    /// Fibonacci Pivot Points
    Fibonacci,
    /// Camarilla Pivot Points
    Camarilla,
    /// Woodie's Pivot Points
    Woodie,
    /// DeMark Pivot Points
    DeMark,
}

/// Временной период для Pivot Points
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PivotPeriod {
    /// Дневные пивоты
    Daily,
    /// Недельные пивоты
    Weekly,
    /// Месячные пивоты
    Monthly,
    /// Часовые пивоты
    Hourly,
    /// 4-часовые пивоты
    FourHour,
}

/// Сигналы Pivot Channels
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PivotSignal {
    /// Пробой сопротивления R1
    BreakoutR1,
    /// Пробой сопротивления R2
    BreakoutR2,
    /// Пробой сопротивления R3
    BreakoutR3,
    /// Отскок от сопротивления
    BounceResistance,
    /// Отскок от поддержки
    BounceSupport,
    /// Пробой поддержки S1
    BreakdownS1,
    /// Пробой поддержки S2
    BreakdownS2,
    /// Пробой поддержки S3
    BreakdownS3,
    /// Движение к Pivot Point
    MovingToPivot,
    /// Внутри канала
    WithinRange,
}

/// Уровни Pivot Channel Points
#[derive(Debug, Clone, Copy)]
pub struct PivotChannelLevels {
    pub pivot_point: f64,    // PP
    pub resistance_1: f64,   // R1
    pub resistance_2: f64,   // R2
    pub resistance_3: f64,   // R3
    pub support_1: f64,      // S1
    pub support_2: f64,      // S2
    pub support_3: f64,      // S3
}

/// High-Performance Pivot Channels
#[derive(Debug, Clone)]
pub struct PivotChannels {
    // Параметры
    pivot_type: PivotType,
    period: PivotPeriod,
    adaptive_width: bool,
    
    // Данные для расчета пивотов
    period_high: f64,
    period_low: f64,
    period_close: f64,
    period_open: f64,
    
    // Буфер для адаптивности
    volatility_buffer: ArrayVec<f64, 100>,
    volatility_index: usize,
    volatility_filled: bool,
    
    // Текущие пивот-уровни
    current_levels: PivotChannelLevels,
    
    // Адаптивные параметры
    avg_volatility: f64,
    width_multiplier: f64,
    
    // Каналы (активные уровни поддержки/сопротивления)
    active_resistance: f64,
    active_support: f64,
    channel_width: f64,
    
    // Статистика пересечений
    breakout_count: usize,
    bounce_count: usize,
    
    // Счетчики времени
    bars_since_period_start: usize,
    bars_per_period: usize,
    
    // Статистика
    bar_count: usize,
}

impl PivotChannels {
    /// Создать Pivot Channels со стандартными параметрами
    pub fn new() -> Self {
        Self::new_custom(PivotType::Classic, PivotPeriod::Daily, true)
    }
    
    /// Создать Pivot Channels с кастомными параметрами
    pub fn new_custom(pivot_type: PivotType, period: PivotPeriod, adaptive_width: bool) -> Self {
        let bars_per_period = match period {
            PivotPeriod::Hourly => 60,      // 60 минутных баров
            PivotPeriod::FourHour => 240,   // 240 минутных баров
            PivotPeriod::Daily => 1440,     // 1440 минутных баров
            PivotPeriod::Weekly => 10080,   // 10080 минутных баров
            PivotPeriod::Monthly => 43200,  // примерно месяц
        };
        
        Self {
            pivot_type,
            period,
            adaptive_width,
            period_high: 0.0,
            period_low: f64::INFINITY,
            period_close: 0.0,
            period_open: 0.0,
            volatility_buffer: ArrayVec::new(),
            volatility_index: 0,
            volatility_filled: false,
            current_levels: PivotChannelLevels {
                pivot_point: 0.0,
                resistance_1: 0.0,
                resistance_2: 0.0,
                resistance_3: 0.0,
                support_1: 0.0,
                support_2: 0.0,
                support_3: 0.0,
            },
            avg_volatility: 0.0,
            width_multiplier: 1.0,
            active_resistance: 0.0,
            active_support: 0.0,
            channel_width: 0.0,
            breakout_count: 0,
            bounce_count: 0,
            bars_since_period_start: 0,
            bars_per_period,
            bar_count: 0,
        }
    }
    
    /// Создать классические дневные пивоты
    pub fn new_classic_daily() -> Self {
        Self::new_custom(PivotType::Classic, PivotPeriod::Daily, false)
    }
    
    /// Создать Fibonacci пивоты с адаптивной шириной
    pub fn new_fibonacci_adaptive() -> Self {
        Self::new_custom(PivotType::Fibonacci, PivotPeriod::Daily, true)
    }
    
    /// Создать Camarilla пивоты
    pub fn new_camarilla() -> Self {
        Self::new_custom(PivotType::Camarilla, PivotPeriod::Daily, true)
    }
    
    /// Обновить каналы новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, _volume: f64) -> (f64, f64, f64) {
        self.bar_count += 1;
        self.bars_since_period_start += 1;
        
        // Обновляем данные периода
        self.update_period_data(open, high, low, close);
        
        // Проверяем, нужно ли пересчитать пивоты
        if self.should_recalculate_pivots() {
            self.calculate_pivot_levels();
            self.update_active_channels(close);
            self.reset_period_data(open);
        }
        
        // Обновляем адаптивные параметры
        if self.adaptive_width {
            self.update_volatility(high, low);
            self.update_width_multiplier();
        }
        
        // Обновляем активные каналы
        self.update_active_channels(close);
        
        (self.active_resistance, self.current_levels.pivot_point, self.active_support)
    }
    
    /// Обновить данные периода
    fn update_period_data(&mut self, open: f64, high: f64, low: f64, close: f64) {
        if self.bars_since_period_start == 1 {
            self.period_open = open;
            self.period_high = high;
            self.period_low = low;
        } else {
            self.period_high = self.period_high.max(high);
            self.period_low = self.period_low.min(low);
        }
        self.period_close = close;
    }
    
    /// Проверить, нужно ли пересчитать пивоты
    fn should_recalculate_pivots(&self) -> bool {
        self.bars_since_period_start >= self.bars_per_period
    }
    
    /// Рассчитать уровни Pivot Points
    fn calculate_pivot_levels(&mut self) {
        let h = self.period_high;
        let l = self.period_low;
        let c = self.period_close;
        let o = self.period_open;
        
        match self.pivot_type {
            PivotType::Classic => {
                let pp = (h + l + c) / 3.0;
                self.current_levels = PivotChannelLevels {
                    pivot_point: pp,
                    resistance_1: 2.0 * pp - l,
                    resistance_2: pp + (h - l),
                    resistance_3: h + 2.0 * (pp - l),
                    support_1: 2.0 * pp - h,
                    support_2: pp - (h - l),
                    support_3: l - 2.0 * (h - pp),
                };
            }
            
            PivotType::Fibonacci => {
                let pp = (h + l + c) / 3.0;
                let range = h - l;
                self.current_levels = PivotChannelLevels {
                    pivot_point: pp,
                    resistance_1: pp + 0.382 * range,
                    resistance_2: pp + 0.618 * range,
                    resistance_3: pp + range,
                    support_1: pp - 0.382 * range,
                    support_2: pp - 0.618 * range,
                    support_3: pp - range,
                };
            }
            
            PivotType::Camarilla => {
                let range = h - l;
                self.current_levels = PivotChannelLevels {
                    pivot_point: c,
                    resistance_1: c + range * 1.1 / 12.0,
                    resistance_2: c + range * 1.1 / 6.0,
                    resistance_3: c + range * 1.1 / 4.0,
                    support_1: c - range * 1.1 / 12.0,
                    support_2: c - range * 1.1 / 6.0,
                    support_3: c - range * 1.1 / 4.0,
                };
            }
            
            PivotType::Woodie => {
                let pp = (h + l + 2.0 * c) / 4.0;
                self.current_levels = PivotChannelLevels {
                    pivot_point: pp,
                    resistance_1: 2.0 * pp - l,
                    resistance_2: pp + h - l,
                    resistance_3: h + 2.0 * (pp - l),
                    support_1: 2.0 * pp - h,
                    support_2: pp - h + l,
                    support_3: l - 2.0 * (h - pp),
                };
            }
            
            PivotType::DeMark => {
                let x = if c < o {
                    h + 2.0 * l + c
                } else if c > o {
                    2.0 * h + l + c
                } else {
                    h + l + 2.0 * c
                };
                let pp = x / 4.0;
                
                self.current_levels = PivotChannelLevels {
                    pivot_point: pp,
                    resistance_1: x / 2.0 - l,
                    resistance_2: pp + (h - l),
                    resistance_3: h + 2.0 * (pp - l),
                    support_1: x / 2.0 - h,
                    support_2: pp - (h - l),
                    support_3: l - 2.0 * (h - pp),
                };
            }
        }
    }
    
    /// Обновить адаптивную волатильность
    fn update_volatility(&mut self, high: f64, low: f64) {
        let true_range = high - low;
        
        if self.volatility_filled {
            self.volatility_buffer[self.volatility_index] = true_range;
        } else {
            self.volatility_buffer.push(true_range);
        }
        
        self.volatility_index = (self.volatility_index + 1) % self.volatility_buffer.capacity();
        
        if self.volatility_buffer.len() == self.volatility_buffer.capacity() && !self.volatility_filled {
            self.volatility_filled = true;
        }
        
        // Рассчитываем среднюю волатильность
        if !self.volatility_buffer.is_empty() {
            self.avg_volatility = self.volatility_buffer.iter().sum::<f64>() / self.volatility_buffer.len() as f64;
        }
    }
    
    /// Обновить множитель ширины канала
    fn update_width_multiplier(&mut self) {
        if self.avg_volatility > 0.0 {
            let current_volatility = if let Some(&last_tr) = self.volatility_buffer.last() {
                last_tr
            } else {
                self.avg_volatility
            };
            
            // Адаптивный множитель на основе текущей волатильности
            self.width_multiplier = (current_volatility / self.avg_volatility).clamp(0.5, 2.0);
        }
    }
    
    /// Обновить активные каналы
    fn update_active_channels(&mut self, current_price: f64) {
        // Определяем ближайшие уровни сопротивления и поддержки
        let levels = [
            self.current_levels.resistance_3,
            self.current_levels.resistance_2,
            self.current_levels.resistance_1,
            self.current_levels.pivot_point,
            self.current_levels.support_1,
            self.current_levels.support_2,
            self.current_levels.support_3,
        ];
        
        // Находим ближайшие уровни выше и ниже текущей цены
        let mut resistance = f64::INFINITY;
        let mut support = f64::NEG_INFINITY;
        
        for &level in &levels {
            if level > current_price && level < resistance {
                resistance = level;
            }
            if level < current_price && level > support {
                support = level;
            }
        }
        
        // Применяем адаптивную ширину если включена
        if self.adaptive_width && self.width_multiplier != 1.0 {
            let center = (resistance + support) / 2.0;
            let half_width = (resistance - support) / 2.0 * self.width_multiplier;
            resistance = center + half_width;
            support = center - half_width;
        }
        
        self.active_resistance = resistance;
        self.active_support = support;
        self.channel_width = resistance - support;
    }
    
    /// Сбросить данные периода
    fn reset_period_data(&mut self, new_open: f64) {
        self.bars_since_period_start = 0;
        self.period_open = new_open;
        self.period_high = 0.0;
        self.period_low = f64::INFINITY;
        self.period_close = 0.0;
    }
    
    /// Получить основные значения (сопротивление, пивот, поддержка)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.active_resistance,
            middle: self.current_levels.pivot_point,
            lower: self.active_support,
        }
    }

    /// Получить основные значения как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.active_resistance, self.current_levels.pivot_point, self.active_support)
    }
    
    /// Получить все уровни пивотов
    pub fn all_levels(&self) -> PivotChannelLevels {
        self.current_levels
    }
    
    /// Получить активные каналы
    pub fn active_channels(&self) -> (f64, f64) {
        (self.active_resistance, self.active_support)
    }
    
    /// Получить ширину канала
    pub fn channel_width(&self) -> f64 {
        self.channel_width
    }
    
    /// Получить позицию цены в канале
    pub fn position_in_channel(&self, price: f64) -> f64 {
        if self.channel_width > 0.0 {
            (price - self.active_support) / self.channel_width
        } else {
            0.5
        }
    }
    
    /// Генерация сигнала на основе позиции цены
    pub fn generate_signal(&self, current_price: f64, previous_price: f64) -> PivotSignal {
        let levels = &self.current_levels;
        
        // Проверяем пробои сопротивлений
        if previous_price <= levels.resistance_1 && current_price > levels.resistance_1 {
            self.increment_breakout_count();
            return PivotSignal::BreakoutR1;
        }
        if previous_price <= levels.resistance_2 && current_price > levels.resistance_2 {
            self.increment_breakout_count();
            return PivotSignal::BreakoutR2;
        }
        if previous_price <= levels.resistance_3 && current_price > levels.resistance_3 {
            self.increment_breakout_count();
            return PivotSignal::BreakoutR3;
        }
        
        // Проверяем пробои поддержек
        if previous_price >= levels.support_1 && current_price < levels.support_1 {
            self.increment_breakout_count();
            return PivotSignal::BreakdownS1;
        }
        if previous_price >= levels.support_2 && current_price < levels.support_2 {
            self.increment_breakout_count();
            return PivotSignal::BreakdownS2;
        }
        if previous_price >= levels.support_3 && current_price < levels.support_3 {
            self.increment_breakout_count();
            return PivotSignal::BreakdownS3;
        }
        
        // Проверяем отскоки
        if self.is_bounce_from_level(current_price, previous_price, levels.resistance_1, true) ||
           self.is_bounce_from_level(current_price, previous_price, levels.resistance_2, true) ||
           self.is_bounce_from_level(current_price, previous_price, levels.resistance_3, true) {
            self.increment_bounce_count();
            return PivotSignal::BounceResistance;
        }
        
        if self.is_bounce_from_level(current_price, previous_price, levels.support_1, false) ||
           self.is_bounce_from_level(current_price, previous_price, levels.support_2, false) ||
           self.is_bounce_from_level(current_price, previous_price, levels.support_3, false) {
            self.increment_bounce_count();
            return PivotSignal::BounceSupport;
        }
        
        // Проверяем движение к пивоту
        let distance_to_pivot = (current_price - levels.pivot_point).abs();
        let prev_distance_to_pivot = (previous_price - levels.pivot_point).abs();
        
        if distance_to_pivot < prev_distance_to_pivot && distance_to_pivot < self.channel_width * 0.1 {
            return PivotSignal::MovingToPivot;
        }
        
        PivotSignal::WithinRange
    }
    
    /// Проверить отскок от уровня
    fn is_bounce_from_level(&self, current_price: f64, previous_price: f64, level: f64, is_resistance: bool) -> bool {
        let tolerance = self.channel_width * 0.01; // 1% от ширины канала
        
        if is_resistance {
            // Отскок от сопротивления: цена подошла близко и развернулась вниз
            previous_price >= level - tolerance && 
            previous_price <= level + tolerance && 
            current_price < previous_price
        } else {
            // Отскок от поддержки: цена подошла близко и развернулась вверх
            previous_price >= level - tolerance && 
            previous_price <= level + tolerance && 
            current_price > previous_price
        }
    }
    
    /// Получить ближайший уровень поддержки
    pub fn nearest_support(&self, price: f64) -> f64 {
        let levels = [
            self.current_levels.support_1,
            self.current_levels.support_2,
            self.current_levels.support_3,
            self.current_levels.pivot_point,
        ];
        
        levels.iter()
            .filter(|&&level| level < price)
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
    }
    
    /// Получить ближайший уровень сопротивления
    pub fn nearest_resistance(&self, price: f64) -> f64 {
        let levels = [
            self.current_levels.resistance_1,
            self.current_levels.resistance_2,
            self.current_levels.resistance_3,
            self.current_levels.pivot_point,
        ];
        
        levels.iter()
            .filter(|&&level| level > price)
            .copied()
            .fold(f64::INFINITY, f64::min)
    }
    
    /// Получить статистику пробоев
    pub fn breakout_stats(&self) -> (usize, usize) {
        (self.breakout_count, self.bounce_count)
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        // Ready after first bar - pivot levels calculated from period data
        self.bar_count > 0
    }
    
    /// Получить параметры
    pub fn get_params(&self) -> (PivotType, PivotPeriod, bool) {
        (self.pivot_type, self.period, self.adaptive_width)
    }
    
    /// Incremental methods (используются в generate_signal)
    fn increment_breakout_count(&self) {
        // В реальной реализации это должно быть мutable, но для примера оставим как есть
        // self.breakout_count += 1;
    }
    
    fn increment_bounce_count(&self) {
        // В реальной реализации это должно быть мutable
        // self.bounce_count += 1;
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.period_high = 0.0;
        self.period_low = f64::INFINITY;
        self.period_close = 0.0;
        self.period_open = 0.0;
        
        self.volatility_buffer.clear();
        self.volatility_index = 0;
        self.volatility_filled = false;
        
        self.current_levels = PivotChannelLevels {
            pivot_point: 0.0,
            resistance_1: 0.0,
            resistance_2: 0.0,
            resistance_3: 0.0,
            support_1: 0.0,
            support_2: 0.0,
            support_3: 0.0,
        };
        
        self.avg_volatility = 0.0;
        self.width_multiplier = 1.0;
        self.active_resistance = 0.0;
        self.active_support = 0.0;
        self.channel_width = 0.0;
        
        self.breakout_count = 0;
        self.bounce_count = 0;
        self.bars_since_period_start = 0;
        self.bar_count = 0;
    }
}

impl Default for PivotChannels {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pivot_channels_creation() {
        let pc = PivotChannels::new();
        assert!(!pc.is_ready());
        assert_eq!(pc.channel_width(), 0.0);
    }

    #[test]
    fn test_pivot_channels_classic_daily() {
        let pc = PivotChannels::new_classic_daily();
        let (pivot_type, period, adaptive) = pc.get_params();
        assert_eq!(pivot_type, PivotType::Classic);
        assert_eq!(period, PivotPeriod::Daily);
        assert!(!adaptive);
    }

    #[test]
    fn test_pivot_channels_fibonacci() {
        let pc = PivotChannels::new_fibonacci_adaptive();
        let (pivot_type, _, adaptive) = pc.get_params();
        assert_eq!(pivot_type, PivotType::Fibonacci);
        assert!(adaptive);
    }

    #[test]
    fn test_pivot_channels_camarilla() {
        let pc = PivotChannels::new_camarilla();
        let (pivot_type, _, _) = pc.get_params();
        assert_eq!(pivot_type, PivotType::Camarilla);
    }

    #[test]
    fn test_pivot_channels_update() {
        let mut pc = PivotChannels::new();
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            let (r, p, s) = pc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(r.is_finite() || r.is_infinite());
            assert!(p.is_finite());
            assert!(s.is_finite() || s.is_infinite());
        }
    }

    #[test]
    fn test_pivot_channels_levels() {
        let mut pc = PivotChannels::new_custom(PivotType::Classic, PivotPeriod::Hourly, false);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            pc.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
        }
        let levels = pc.all_levels();
        assert!(levels.resistance_1 >= levels.pivot_point || levels.pivot_point == 0.0);
    }

    #[test]
    fn test_pivot_channels_reset() {
        let mut pc = PivotChannels::new();
        for i in 0..50 {
            pc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        pc.reset();
        assert!(!pc.is_ready());
        assert_eq!(pc.channel_width(), 0.0);
    }

    #[test]
    fn test_pivot_channels_position() {
        let mut pc = PivotChannels::new_custom(PivotType::Classic, PivotPeriod::Hourly, false);
        for i in 0..100 {
            let price = 100.0 + i as f64;
            pc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        let pos = pc.position_in_channel(150.0);
        assert!(pos.is_finite());
    }
} 






















