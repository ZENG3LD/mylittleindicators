//! fibonacci_channels.rs: High-Performance Fibonacci Channels
//! Каналы Фибоначчи на основе автоматического определения свингов
//! 
//! Особенности:
//! - AutoFibo для автоматического определения уровней
//! - Каналы между ключевыми уровнями (23.6%, 38.2%, 61.8%)
//! - Поддержка как откатных, так и расширяющих каналов
//! - Адаптация к актуальным свингам

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::momentum::auto_fibo::{AutoFibo, FiboLevels, SwingPoint};
use serde::{Serialize, Deserialize};

/// Режимы Fibonacci Channels
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub enum FibonacciChannelMode {
    /// Retracement - каналы на откатных уровнях (23.6-38.2%, 38.2-61.8%)
    #[default]
    Retracement,
    /// Extension - каналы на расширяющих уровнях (100-161.8%, 161.8-261.8%)
    Extension,
    /// Combined - комбинированные каналы (основные + расширения)
    Combined,
}


/// Fibonacci Channel - канал между двумя уровнями Фибоначчи
#[derive(Debug, Clone, Copy)]
pub struct FibonacciChannel {
    pub upper: f64,
    pub lower: f64,
    pub level_name: &'static str,
    pub is_active: bool,
}

impl FibonacciChannel {
    fn new(upper: f64, lower: f64, level_name: &'static str) -> Self {
        Self {
            upper,
            lower,
            level_name,
            is_active: upper != 0.0 && lower != 0.0,
        }
    }
    
    /// Получить ширину канала
    pub fn width(&self) -> f64 {
        if self.is_active {
            (self.upper - self.lower).abs()
        } else {
            0.0
        }
    }
    
    /// Получить середину канала
    pub fn middle(&self) -> f64 {
        if self.is_active {
            (self.upper + self.lower) / 2.0
        } else {
            0.0
        }
    }
    
    /// Проверить попадание цены в канал
    pub fn contains_price(&self, price: f64) -> bool {
        if !self.is_active {
            return false;
        }
        
        let min = self.upper.min(self.lower);
        let max = self.upper.max(self.lower);
        price >= min && price <= max
    }
    
    /// Получить позицию цены в канале (0.0-1.0)
    pub fn position_in_channel(&self, price: f64) -> f64 {
        if !self.is_active {
            return 0.5;
        }
        
        let min = self.upper.min(self.lower);
        let max = self.upper.max(self.lower);
        
        if max == min {
            0.5
        } else {
            ((price - min) / (max - min)).clamp(0.0, 1.0)
        }
    }
}

/// High-Performance Fibonacci Channels
#[derive(Debug, Clone)]
pub struct FibonacciChannels {
    mode: FibonacciChannelMode,
    
    // AutoFibo для определения уровней
    auto_fibo: AutoFibo,
    
    // Каналы между уровнями Фибоначчи
    channels: Vec<FibonacciChannel>,
    
    // Текущие уровни Фибоначчи
    current_levels: Option<FiboLevels>,
    
    // Основной канал (самый значимый)
    primary_channel: FibonacciChannel,
    
    // Статистика
    bar_count: usize,
    last_swing_update: usize,
}

impl FibonacciChannels {
    /// Создать Fibonacci Channels с параметрами AutoFibo
    pub fn new(
        zigzag_period: usize, 
        atr_period: usize, 
        atr_multiplier: f64,
        mode: FibonacciChannelMode
    ) -> Self {
        Self {
            mode,
            auto_fibo: AutoFibo::new(zigzag_period, atr_period, atr_multiplier),
            channels: Vec::new(),
            current_levels: None,
            primary_channel: FibonacciChannel::new(0.0, 0.0, "None"),
            bar_count: 0,
            last_swing_update: 0,
        }
    }
    
    /// Создать Retracement Fibonacci Channels
    pub fn new_retracement(zigzag_period: usize, atr_period: usize, atr_multiplier: f64) -> Self {
        Self::new(zigzag_period, atr_period, atr_multiplier, FibonacciChannelMode::Retracement)
    }
    
    /// Создать Extension Fibonacci Channels
    pub fn new_extension(zigzag_period: usize, atr_period: usize, atr_multiplier: f64) -> Self {
        Self::new(zigzag_period, atr_period, atr_multiplier, FibonacciChannelMode::Extension)
    }
    
    /// Создать Combined Fibonacci Channels
    pub fn new_combined(zigzag_period: usize, atr_period: usize, atr_multiplier: f64) -> Self {
        Self::new(zigzag_period, atr_period, atr_multiplier, FibonacciChannelMode::Combined)
    }
    
    /// Обновить каналы новым баром
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> Vec<FibonacciChannel> {
        self.bar_count += 1;
        
        // Обновляем AutoFibo
        if let Some(_swing) = self.auto_fibo.update(high, low, close, self.bar_count) {
            self.last_swing_update = self.bar_count;
            self.update_fibonacci_levels();
            self.calculate_channels();
        }
        
        self.channels.clone()
    }
    
    /// Обновить уровни Фибоначчи
    fn update_fibonacci_levels(&mut self) {
        self.current_levels = match self.mode {
            FibonacciChannelMode::Retracement => self.auto_fibo.get_retracement_levels(),
            FibonacciChannelMode::Extension => self.auto_fibo.get_extension_levels(),
            FibonacciChannelMode::Combined => {
                // Для Combined используем retracement как основные
                self.auto_fibo.get_retracement_levels()
            }
        };
    }
    
    /// Рассчитать каналы между уровнями Фибоначчи
    fn calculate_channels(&mut self) {
        self.channels.clear();
        
        // Клонируем levels чтобы избежать проблем с заимствованием
        let levels_copy = self.current_levels;
        
        if let Some(levels) = levels_copy {
            match self.mode {
                FibonacciChannelMode::Retracement => {
                    self.calculate_retracement_channels(&levels);
                }
                FibonacciChannelMode::Extension => {
                    self.calculate_extension_channels(&levels);
                }
                FibonacciChannelMode::Combined => {
                    self.calculate_combined_channels(&levels);
                }
            }
            
            // Устанавливаем основной канал (самый важный)
            self.set_primary_channel();
        }
    }
    
    /// Рассчитать каналы для откатных уровней
    fn calculate_retracement_channels(&mut self, levels: &FiboLevels) {
        // Канал 0% - 23.6%
        self.channels.push(FibonacciChannel::new(
            levels.level_236, levels.level_0, "0%-23.6%"
        ));
        
        // Канал 23.6% - 38.2% (важный)
        self.channels.push(FibonacciChannel::new(
            levels.level_382, levels.level_236, "23.6%-38.2%"
        ));
        
        // Канал 38.2% - 50%
        self.channels.push(FibonacciChannel::new(
            levels.level_500, levels.level_382, "38.2%-50%"
        ));
        
        // Канал 50% - 61.8% (золотое сечение)
        self.channels.push(FibonacciChannel::new(
            levels.level_618, levels.level_500, "50%-61.8%"
        ));
        
        // Канал 61.8% - 78.6%
        self.channels.push(FibonacciChannel::new(
            levels.level_786, levels.level_618, "61.8%-78.6%"
        ));
        
        // Канал 78.6% - 100%
        self.channels.push(FibonacciChannel::new(
            levels.level_1, levels.level_786, "78.6%-100%"
        ));
    }
    
    /// Рассчитать каналы для расширений
    fn calculate_extension_channels(&mut self, levels: &FiboLevels) {
        // Канал 100% - 123.6%
        self.channels.push(FibonacciChannel::new(
            levels.level_1236, levels.level_1, "100%-123.6%"
        ));
        
        // Канал 123.6% - 161.8% (важный)
        self.channels.push(FibonacciChannel::new(
            levels.level_1618, levels.level_1236, "123.6%-161.8%"
        ));
        
        // Канал 161.8% - 261.8%
        self.channels.push(FibonacciChannel::new(
            levels.level_2618, levels.level_1618, "161.8%-261.8%"
        ));
        
        // Канал 261.8% - 423.6%
        self.channels.push(FibonacciChannel::new(
            levels.level_4236, levels.level_2618, "261.8%-423.6%"
        ));
    }
    
    /// Рассчитать комбинированные каналы
    fn calculate_combined_channels(&mut self, levels: &FiboLevels) {
        // Добавляем основные откатные каналы
        self.calculate_retracement_channels(levels);
        
        // Добавляем расширения если есть extension уровни
        if let Some(ext_levels) = self.auto_fibo.get_extension_levels() {
            self.calculate_extension_channels(&ext_levels);
        }
    }
    
    /// Установить основной (самый важный) канал
    fn set_primary_channel(&mut self) {
        // Приоритет каналам с золотым сечением (61.8%, 38.2%)
        for channel in &self.channels {
            if (channel.level_name.contains("61.8") || channel.level_name.contains("38.2"))
                && channel.is_active {
                    self.primary_channel = *channel;
                    return;
                }
        }
        
        // Если нет золотого сечения, берем первый активный канал
        for channel in &self.channels {
            if channel.is_active {
                self.primary_channel = *channel;
                return;
            }
        }
        
        // Если нет активных каналов
        self.primary_channel = FibonacciChannel::new(0.0, 0.0, "None");
    }
    
    /// Получить все активные каналы
    pub fn get_channels(&self) -> &[FibonacciChannel] {
        &self.channels
    }
    
    /// Получить основной канал
    pub fn primary_channel(&self) -> &FibonacciChannel {
        &self.primary_channel
    }

    /// Get main indicator value as Channel3 (upper, middle, lower)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.primary_channel.upper,
            middle: self.primary_channel.middle(),
            lower: self.primary_channel.lower,
        }
    }
    
    /// Получить канал, содержащий указанную цену
    pub fn get_channel_containing_price(&self, price: f64) -> Option<&FibonacciChannel> {
        self.channels.iter().find(|channel| channel.contains_price(price))
    }
    
    /// Получить текущие уровни Фибоначчи
    pub fn current_levels(&self) -> Option<&FiboLevels> {
        self.current_levels.as_ref()
    }
    
    /// Получить последние свинги
    pub fn get_swings(&self) -> (Option<SwingPoint>, Option<SwingPoint>) {
        self.auto_fibo.get_swings()
    }
    
    /// Проверить, обновлялись ли свинги недавно
    pub fn swings_recently_updated(&self, bars_threshold: usize) -> bool {
        self.bar_count.saturating_sub(self.last_swing_update) <= bars_threshold
    }
    
    /// Получить ближайший уровень сопротивления
    pub fn nearest_resistance(&self, current_price: f64) -> Option<f64> {
        if let Some(levels) = &self.current_levels {
            let resistance_levels = [
                levels.level_236, levels.level_382, levels.level_500,
                levels.level_618, levels.level_786, levels.level_1,
                levels.level_1236, levels.level_1618, levels.level_2618,
            ];
            
            resistance_levels
                .iter()
                .filter(|&&level| level > current_price)
                .min_by(|a, b| (*a - current_price).abs().partial_cmp(&(*b - current_price).abs()).unwrap())
                .copied()
        } else {
            None
        }
    }
    
    /// Получить ближайший уровень поддержки
    pub fn nearest_support(&self, current_price: f64) -> Option<f64> {
        if let Some(levels) = &self.current_levels {
            let support_levels = [
                levels.level_0, levels.level_236, levels.level_382,
                levels.level_500, levels.level_618, levels.level_786,
                levels.level_1,
            ];
            
            support_levels
                .iter()
                .filter(|&&level| level < current_price)
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .copied()
        } else {
            None
        }
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.auto_fibo.is_ready() && self.current_levels.is_some()
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.auto_fibo.reset();
        self.channels.clear();
        self.current_levels = None;
        self.primary_channel = FibonacciChannel::new(0.0, 0.0, "None");
        self.bar_count = 0;
        self.last_swing_update = 0;
    }
    
    /// Получить режим
    pub fn mode(&self) -> FibonacciChannelMode {
        self.mode
    }
    
    /// Получить параметры AutoFibo
    pub fn get_auto_fibo_params(&self) -> (usize, usize, f64) {
        self.auto_fibo.get_params()
    }
}

impl Default for FibonacciChannels {
    fn default() -> Self {
        Self::new_retracement(20, 14, 2.0) // Стандартные параметры
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fibonacci_channels_creation() {
        let fc = FibonacciChannels::new_retracement(20, 14, 2.0);
        assert!(!fc.is_ready());
        assert_eq!(fc.mode(), FibonacciChannelMode::Retracement);
    }

    #[test]
    fn test_fibonacci_channels_update() {
        let mut fc = FibonacciChannels::new_retracement(10, 10, 1.5);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 15.0;
            fc.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
        }
        // Fibonacci channels need swings to be detected
        let _channels = fc.get_channels();
        // May or may not be ready depending on swing detection
    }

    #[test]
    fn test_fibonacci_channel_struct() {
        let channel = FibonacciChannel::new(110.0, 100.0, "test");
        assert!(channel.is_active);
        assert_eq!(channel.width(), 10.0);
        assert_eq!(channel.middle(), 105.0);
        assert!(channel.contains_price(105.0));
        assert!(!channel.contains_price(120.0));
    }

    #[test]
    fn test_fibonacci_channels_reset() {
        let mut fc = FibonacciChannels::new_retracement(20, 14, 2.0);
        for i in 0..50 {
            fc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        fc.reset();
        assert!(!fc.is_ready());
    }
} 






















