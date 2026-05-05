//! vwap_channels.rs: High-Performance VWAP Channels
//! VWAP каналы - очень популярны у институциональных трейдеров
//!
//! Особенности:
//! - VWAP как средняя линия
//! - Стандартные отклонения от VWAP как границы
//! - Поддержка множественных стандартных отклонений (1σ, 2σ, 3σ)
//! - Efficient circular buffer для расчета std dev

use crate::bar_indicators::average::vwap::Vwap;
use crate::bar_indicators::indicator_value::IndicatorValue;
use arrayvec::ArrayVec;
use serde::{Serialize, Deserialize};

/// Режимы расчета VWAP Channels
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub enum VwapChannelMode {
    /// Standard - стандартные отклонения от VWAP
    #[default]
    Standard,
    /// Percentage - процентные отклонения от VWAP  
    Percentage,
}


/// High-Performance VWAP Channels
#[derive(Debug, Clone)]
pub struct VwapChannels {
    period: usize,
    std_dev_mult: f64,
    mode: VwapChannelMode,
    
    // VWAP для средней линии
    vwap: Vwap,
    
    // Circular buffer для расчета стандартного отклонения от VWAP
    price_buffer: ArrayVec<f64, 512>, // Typical prices
    vwap_diff_buffer: ArrayVec<f64, 512>, // (price - vwap)^2 для std dev
    buffer_index: usize,
    buffer_filled: bool,
    
    // Текущие значения канала
    upper: f64,
    middle: f64, // VWAP
    lower: f64,
    
    // Дополнительные уровни (1σ, 2σ, 3σ)
    upper_1: f64,
    lower_1: f64,
    upper_2: f64,
    lower_2: f64,
    upper_3: f64,
    lower_3: f64,
    
    // Метрики
    std_dev: f64,
    vwap_distance: f64, // Текущее расстояние цены от VWAP в σ
}

impl VwapChannels {
    /// Создать VWAP Channels с указанными параметрами
    pub fn new(period: usize, std_dev_mult: f64, mode: VwapChannelMode) -> Self {
        assert!(period > 0 && period <= 512, "Period must be between 1 and 512");
        assert!(std_dev_mult > 0.0, "Standard deviation multiplier must be positive");
        
        Self {
            period,
            std_dev_mult,
            mode,
            vwap: Vwap::new(period),
            price_buffer: ArrayVec::new(),
            vwap_diff_buffer: ArrayVec::new(),
            buffer_index: 0,
            buffer_filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
            upper_1: 0.0,
            lower_1: 0.0,
            upper_2: 0.0,
            lower_2: 0.0,
            upper_3: 0.0,
            lower_3: 0.0,
            std_dev: 0.0,
            vwap_distance: 0.0,
        }
    }
    
    /// Создать классические VWAP Channels (стандартные отклонения)
    pub fn new_standard(period: usize, std_dev_mult: f64) -> Self {
        Self::new(period, std_dev_mult, VwapChannelMode::Standard)
    }
    
    /// Создать процентные VWAP Channels
    pub fn new_percentage(period: usize, percentage: f64) -> Self {
        Self::new(period, percentage, VwapChannelMode::Percentage)
    }
    
    /// Обновить каналы новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> (f64, f64, f64) {
        // Обновляем VWAP
        self.middle = self.vwap.update_bar(open, high, low, close, volume);
        
        // Typical price для расчета отклонений
        let typical_price = (high + low + close) / 3.0;
        
        // Добавляем в буфер цен
        if self.buffer_filled {
            // Circular buffer - заменяем старое значение
            self.price_buffer[self.buffer_index] = typical_price;
        } else {
            // Добавляем новое значение
            self.price_buffer.push(typical_price);
        }
        
        // Обновляем индекс циклически
        self.buffer_index = (self.buffer_index + 1) % self.period;
        
        // Проверяем, заполнен ли буфер
        if self.price_buffer.len() == self.period && !self.buffer_filled {
            self.buffer_filled = true;
        }
        
        // Рассчитываем каналы если готово
        if self.is_ready() {
            self.calculate_channels(typical_price);
        } else {
            self.reset_channels();
        }
        
        (self.upper, self.middle, self.lower)
    }
    
    /// Рассчитать границы каналов
    fn calculate_channels(&mut self, current_price: f64) {
        match self.mode {
            VwapChannelMode::Standard => {
                self.calculate_standard_deviation_channels(current_price);
            }
            VwapChannelMode::Percentage => {
                self.calculate_percentage_channels();
            }
        }
    }
    
    /// Рассчитать каналы на основе стандартного отклонения
    fn calculate_standard_deviation_channels(&mut self, current_price: f64) {
        // Рассчитываем стандартное отклонение от VWAP
        let buffer_len = if self.buffer_filled { self.period } else { self.price_buffer.len() };
        
        let variance = self.price_buffer.iter()
            .take(buffer_len)
            .map(|&price| {
                let diff = price - self.middle;
                diff * diff
            })
            .sum::<f64>() / buffer_len as f64;
        
        self.std_dev = variance.sqrt();
        
        // Рассчитываем границы основного канала
        self.upper = self.middle + self.std_dev_mult * self.std_dev;
        self.lower = self.middle - self.std_dev_mult * self.std_dev;
        
        // Рассчитываем дополнительные уровни (1σ, 2σ, 3σ)
        self.upper_1 = self.middle + 1.0 * self.std_dev;
        self.lower_1 = self.middle - 1.0 * self.std_dev;
        self.upper_2 = self.middle + 2.0 * self.std_dev;
        self.lower_2 = self.middle - 2.0 * self.std_dev;
        self.upper_3 = self.middle + 3.0 * self.std_dev;
        self.lower_3 = self.middle - 3.0 * self.std_dev;
        
        // Рассчитываем текущее расстояние от VWAP
        self.vwap_distance = if self.std_dev != 0.0 {
            (current_price - self.middle) / self.std_dev
        } else {
            0.0
        };
    }
    
    /// Рассчитать процентные каналы
    fn calculate_percentage_channels(&mut self) {
        let percentage_band = self.middle * (self.std_dev_mult / 100.0);
        
        self.upper = self.middle + percentage_band;
        self.lower = self.middle - percentage_band;
        
        // Дополнительные процентные уровни
        let band_1 = self.middle * 0.01; // 1%
        let band_2 = self.middle * 0.02; // 2%
        let band_3 = self.middle * 0.03; // 3%
        
        self.upper_1 = self.middle + band_1;
        self.lower_1 = self.middle - band_1;
        self.upper_2 = self.middle + band_2;
        self.lower_2 = self.middle - band_2;
        self.upper_3 = self.middle + band_3;
        self.lower_3 = self.middle - band_3;
        
        self.std_dev = percentage_band; // Для совместимости
        self.vwap_distance = 0.0; // Не применимо для процентного режима
    }
    
    /// Сбросить значения каналов
    fn reset_channels(&mut self) {
        self.upper = 0.0;
        self.lower = 0.0;
        self.upper_1 = 0.0;
        self.lower_1 = 0.0;
        self.upper_2 = 0.0;
        self.lower_2 = 0.0;
        self.upper_3 = 0.0;
        self.lower_3 = 0.0;
        self.std_dev = 0.0;
        self.vwap_distance = 0.0;
    }
    
    /// Получить текущие значения основного канала (upper, middle, lower)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.upper,
            middle: self.middle,
            lower: self.lower,
        }
    }

    /// Получить текущие значения основного канала как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper, self.middle, self.lower)
    }
    
    /// Получить VWAP (средняя линия)
    pub fn vwap(&self) -> f64 {
        self.middle
    }
    
    /// Получить верхнюю границу основного канала
    pub fn upper(&self) -> f64 {
        self.upper
    }
    
    /// Получить нижнюю границу основного канала
    pub fn lower(&self) -> f64 {
        self.lower
    }
    
    /// Получить уровни 1σ (upper_1, lower_1)
    pub fn sigma_1_levels(&self) -> (f64, f64) {
        (self.upper_1, self.lower_1)
    }
    
    /// Получить уровни 2σ (upper_2, lower_2)
    pub fn sigma_2_levels(&self) -> (f64, f64) {
        (self.upper_2, self.lower_2)
    }
    
    /// Получить уровни 3σ (upper_3, lower_3)
    pub fn sigma_3_levels(&self) -> (f64, f64) {
        (self.upper_3, self.lower_3)
    }
    
    /// Получить стандартное отклонение
    pub fn std_dev(&self) -> f64 {
        self.std_dev
    }
    
    /// Получить расстояние текущей цены от VWAP в сигмах
    pub fn vwap_distance(&self) -> f64 {
        self.vwap_distance
    }
    
    /// Получить ширину канала
    pub fn channel_width(&self) -> f64 {
        if self.is_ready() {
            self.upper - self.lower
        } else {
            0.0
        }
    }
    
    /// Получить позицию цены в канале (0.0 = нижняя граница, 1.0 = верхняя граница)
    pub fn position_in_channel(&self, price: f64) -> f64 {
        if !self.is_ready() || self.upper == self.lower {
            0.5 // По центру если канал не готов или нулевой ширины
        } else {
            ((price - self.lower) / (self.upper - self.lower)).clamp(0.0, 1.0)
        }
    }
    
    /// Проверить пробой указанного σ уровня
    pub fn is_sigma_breakout(&self, price: f64, sigma_level: f64) -> Option<bool> {
        if !self.is_ready() || self.std_dev == 0.0 {
            return None;
        }
        
        let upper_sigma = self.middle + sigma_level * self.std_dev;
        let lower_sigma = self.middle - sigma_level * self.std_dev;
        
        if price > upper_sigma {
            Some(true) // Пробой вверх
        } else if price < lower_sigma {
            Some(false) // Пробой вниз
        } else {
            None // Нет пробоя
        }
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.vwap.is_ready() && self.buffer_filled
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.vwap.reset();
        self.price_buffer.clear();
        self.vwap_diff_buffer.clear();
        self.buffer_index = 0;
        self.buffer_filled = false;
        self.reset_channels();
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }
    
    /// Получить множитель стандартного отклонения
    pub fn std_dev_mult(&self) -> f64 {
        self.std_dev_mult
    }
    
    /// Получить режим расчета
    pub fn mode(&self) -> VwapChannelMode {
        self.mode
    }
}

impl Default for VwapChannels {
    fn default() -> Self {
        Self::new_standard(20, 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vwap_channels_creation() {
        let vc = VwapChannels::new_standard(20, 2.0);
        assert!(!vc.is_ready());
        assert_eq!(vc.upper(), 0.0);
        assert_eq!(vc.lower(), 0.0);
    }

    #[test]
    fn test_vwap_channels_warmup() {
        let mut vc = VwapChannels::new_standard(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vc.is_ready());
    }

    #[test]
    fn test_vwap_channels_values() {
        let mut vc = VwapChannels::new_standard(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            vc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vc.upper() >= vc.vwap());
        assert!(vc.vwap() >= vc.lower());
    }

    #[test]
    fn test_vwap_channels_sigma_levels() {
        let mut vc = VwapChannels::new_standard(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            vc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        let (u1, l1) = vc.sigma_1_levels();
        let (u2, l2) = vc.sigma_2_levels();
        let (u3, l3) = vc.sigma_3_levels();
        assert!(u3 >= u2 && u2 >= u1);
        assert!(l1 >= l2 && l2 >= l3);
    }

    #[test]
    fn test_vwap_channels_reset() {
        let mut vc = VwapChannels::new_standard(20, 2.0);
        for i in 0..25 {
            vc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        vc.reset();
        assert!(!vc.is_ready());
        assert_eq!(vc.upper(), 0.0);
        assert_eq!(vc.lower(), 0.0);
    }
} 






















