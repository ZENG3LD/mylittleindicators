//! price_channels.rs: High-Performance Price Channels
//! Каналы цен - похожие на Donchian, но с MA сглаживанием самих экстремумов
//!
//! Особенности:
//! - Circular buffer O(1) operations
//! - MA сглаживание максимумов и минимумов
//! - Поддержка ВСЕХ 19 типов MA
//! - Дополнительные методы аналогично другим каналам

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::average::moving_average::MovingAverageProvider;
use crate::bar_indicators::indicator_value::IndicatorValue;
use arrayvec::ArrayVec;
use serde::{Serialize, Deserialize};

/// Режимы расчета Price Channels
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub enum PriceChannelMode {
    /// Raw - сырые max/min без сглаживания (как Donchian)
    #[default]
    Raw,
    /// Smoothed - с MA сглаживанием max/min
    Smoothed,
}


/// High-Performance Price Channels
#[derive(Debug, Clone)]
pub struct PriceChannels {
    period: usize,
    mode: PriceChannelMode,
    ma_type: MovingAverageType,

    // Circular buffers для high/low - O(1) operations
    high_buffer: ArrayVec<f64, 512>,
    low_buffer: ArrayVec<f64, 512>,
    buffer_index: usize,
    buffer_filled: bool,

    // MovingAverages для сглаживания (только для Smoothed режима)
    ma_high: Option<MovingAverageProvider>,
    ma_low: Option<MovingAverageProvider>,

    // Текущие значения канала
    upper: f64,
    middle: f64,
    lower: f64,
}

impl PriceChannels {
    /// Создать Price Channels с указанными параметрами
    pub fn new(period: usize, mode: PriceChannelMode, ma_type: MovingAverageType) -> Self {
        assert!(period > 0 && period <= 512, "Period must be between 1 and 512");

        let (ma_high, ma_low) = if mode == PriceChannelMode::Smoothed {
            (
                Some(MovingAverageProvider::new(ma_type, period)),
                Some(MovingAverageProvider::new(ma_type, period)),
            )
        } else {
            (None, None)
        };

        Self {
            period,
            mode,
            ma_type,
            high_buffer: ArrayVec::new(),
            low_buffer: ArrayVec::new(),
            buffer_index: 0,
            buffer_filled: false,
            ma_high,
            ma_low,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    /// Создать Raw Price Channels (аналогично Donchian)
    pub fn new_raw(period: usize) -> Self {
        Self::new(period, PriceChannelMode::Raw, MovingAverageType::SMA)
    }

    /// Создать Smoothed Price Channels с SMA
    pub fn new_smoothed_sma(period: usize) -> Self {
        Self::new(period, PriceChannelMode::Smoothed, MovingAverageType::SMA)
    }

    /// Создать Smoothed Price Channels с EMA
    pub fn new_smoothed_ema(period: usize) -> Self {
        Self::new(period, PriceChannelMode::Smoothed, MovingAverageType::EMA)
    }

    /// Создать Smoothed Price Channels с указанным MA типом
    pub fn new_smoothed(period: usize, ma_type: MovingAverageType) -> Self {
        Self::new(period, PriceChannelMode::Smoothed, ma_type)
    }

    /// Обновить каналы новым баром
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, _close: f64, _volume: f64) -> (f64, f64, f64) {
        // Обновляем circular buffers - O(1) operations
        if self.buffer_filled {
            // Заменяем старые значения
            self.high_buffer[self.buffer_index] = high;
            self.low_buffer[self.buffer_index] = low;
        } else {
            // Добавляем новые значения
            self.high_buffer.push(high);
            self.low_buffer.push(low);
        }

        // Обновляем индекс циклически
        self.buffer_index = (self.buffer_index + 1) % self.period;

        // Проверяем, заполнен ли буфер
        if self.high_buffer.len() == self.period && !self.buffer_filled {
            self.buffer_filled = true;
        }

        // Рассчитываем каналы в зависимости от режима
        match self.mode {
            PriceChannelMode::Raw => {
                self.calculate_raw_channels();
            }
            PriceChannelMode::Smoothed => {
                self.calculate_smoothed_channels(high, low);
            }
        }

        (self.upper, self.middle, self.lower)
    }

    /// Рассчитать сырые каналы (max/min)
    fn calculate_raw_channels(&mut self) {
        if self.high_buffer.is_empty() || self.low_buffer.is_empty() {
            self.upper = 0.0;
            self.lower = 0.0;
        } else {
            let buffer_len = if self.buffer_filled { self.period } else { self.high_buffer.len() };

            let (lower, upper) = self.high_buffer.iter()
                .zip(self.low_buffer.iter())
                .take(buffer_len)
                .fold((f64::INFINITY, f64::NEG_INFINITY),
                      |(min, max), (&h, &l)| (min.min(l), max.max(h)));
            self.upper = upper;
            self.lower = lower;
        }

        // Средняя линия
        self.middle = (self.upper + self.lower) / 2.0;
    }

    /// Рассчитать сглаженные каналы
    fn calculate_smoothed_channels(&mut self, _current_high: f64, _current_low: f64) {
        if let (Some(ref mut ma_high), Some(ref mut ma_low)) = (&mut self.ma_high, &mut self.ma_low) {
            // Получаем текущие max/min для периода
            let buffer_len = if self.buffer_filled { self.period } else { self.high_buffer.len() };

            let (current_min, current_max) = self.high_buffer.iter()
                .zip(self.low_buffer.iter())
                .take(buffer_len)
                .fold((f64::INFINITY, f64::NEG_INFINITY),
                      |(min, max), (&h, &l)| (min.min(l), max.max(h)));

            // Применяем MA к max/min значениям
            self.upper = ma_high.update_bar(current_max, current_max, current_max, current_max, 0.0);
            self.lower = ma_low.update_bar(current_min, current_min, current_min, current_min, 0.0);
        } else {
            // Fallback к сырым каналам
            self.calculate_raw_channels();
        }

        // Средняя линия
        self.middle = (self.upper + self.lower) / 2.0;
    }

    /// Получить текущие значения каналов как типизированный IndicatorValue
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.upper,
            middle: self.middle,
            lower: self.lower,
        }
    }

    /// Получить текущие значения каналов как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper, self.middle, self.lower)
    }

    /// Получить верхний канал
    pub fn upper(&self) -> f64 {
        self.upper
    }

    /// Получить среднюю линию
    pub fn middle(&self) -> f64 {
        self.middle
    }

    /// Получить нижний канал
    pub fn lower(&self) -> f64 {
        self.lower
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

    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        match self.mode {
            PriceChannelMode::Raw => self.buffer_filled,
            PriceChannelMode::Smoothed => {
                if let (Some(ref ma_high), Some(ref ma_low)) = (&self.ma_high, &self.ma_low) {
                    self.buffer_filled && ma_high.is_ready() && ma_low.is_ready()
                } else {
                    self.buffer_filled
                }
            }
        }
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.high_buffer.clear();
        self.low_buffer.clear();
        self.buffer_index = 0;
        self.buffer_filled = false;

        if let Some(ref mut ma_high) = self.ma_high {
            ma_high.reset();
        }
        if let Some(ref mut ma_low) = self.ma_low {
            ma_low.reset();
        }

        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }

    /// Получить период
    pub fn period(&self) -> usize {
        self.period
    }

    /// Получить режим расчета
    pub fn mode(&self) -> PriceChannelMode {
        self.mode
    }

    /// Получить тип MA (для Smoothed режима)
    pub fn ma_type(&self) -> MovingAverageType {
        self.ma_type
    }
}

impl Default for PriceChannels {
    fn default() -> Self {
        Self::new_raw(20)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_price_channels_creation() {
        let pc = PriceChannels::new_raw(20);
        assert!(!pc.is_ready());
        assert_eq!(pc.upper(), 0.0);
        assert_eq!(pc.lower(), 0.0);
    }

    #[test]
    fn test_price_channels_warmup() {
        let mut pc = PriceChannels::new_raw(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pc.is_ready());
    }

    #[test]
    fn test_price_channels_values() {
        let mut pc = PriceChannels::new_raw(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            pc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pc.upper() >= pc.middle());
        assert!(pc.middle() >= pc.lower());
    }

    #[test]
    fn test_price_channels_smoothed() {
        let mut pc = PriceChannels::new_smoothed_ema(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            pc.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
        }
        assert!(pc.is_ready());
        assert!(pc.channel_width() >= 0.0);
    }

    #[test]
    fn test_price_channels_position() {
        let mut pc = PriceChannels::new_raw(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            pc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        let pos = pc.position_in_channel(110.0);
        assert!(pos >= 0.0 && pos <= 1.0);
    }

    #[test]
    fn test_price_channels_reset() {
        let mut pc = PriceChannels::new_raw(20);
        for i in 0..25 {
            pc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        pc.reset();
        assert!(!pc.is_ready());
        assert_eq!(pc.upper(), 0.0);
        assert_eq!(pc.lower(), 0.0);
    }
}
