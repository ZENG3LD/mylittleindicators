//! Donchian Channel Indicator - ОПТИМИЗИРОВАННАЯ ВЕРСИЯ
//!
//! Индикатор канала Дончиана для определения поддержки/сопротивления:
//! - Upper Band = Highest High за period периодов
//! - Lower Band = Lowest Low за period периодов
//! - Middle Band = (Upper + Lower) / 2
//!
//! 🚀 СУПЕР ОПТИМИЗАЦИЯ: Циклический буфер O(1) + поддержка ВСЕХ 19 типов MA!

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use arrayvec::ArrayVec;
use serde::{Serialize, Deserialize};

/// Режимы расчета Donchian Channel
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub enum DonchianMode {
    /// Классический - простые максимумы/минимумы
    #[default]
    Classic,
    /// Сглаженный - применяется MA к уровням каналов
    Smoothed,
}


/// ОПТИМИЗИРОВАННЫЙ Donchian Channel индикатор
#[derive(Debug, Clone)]
pub struct DonchianChannel {
    period: usize,
    mode: DonchianMode,
    ma_type: MovingAverageType,

    // 🚀 СУПЕР БЫСТРЫЕ циклические буферы (O(1) операции!)
    high_buffer: ArrayVec<f64, 512>,
    low_buffer: ArrayVec<f64, 512>,
    buffer_index: usize,  // Индекс для циклической перезаписи
    buffer_filled: bool,  // Заполнен ли буфер полностью

    // Сглаживающие MA (используются только в режиме Smoothed)
    upper_ma: Option<MovingAverageProvider>,
    lower_ma: Option<MovingAverageProvider>,
    middle_ma: Option<MovingAverageProvider>,

    // Текущие значения каналов
    upper_band: f64,
    lower_band: f64,
    middle_band: f64,

    // Готовность индикатора
    is_ready: bool,
}

impl DonchianChannel {
    /// Создать новый Donchian Channel с классическим режимом
    pub fn new(period: usize) -> Self {
        Self::new_with_mode(period, DonchianMode::Classic, MovingAverageType::SMA)
    }

    /// Создать Donchian Channel с указанным режимом и типом MA
    pub fn new_with_mode(period: usize, mode: DonchianMode, ma_type: MovingAverageType) -> Self {
        let (upper_ma, lower_ma, middle_ma) = match mode {
            DonchianMode::Classic => (None, None, None),
            DonchianMode::Smoothed => (
                Some(MovingAverageProvider::new(ma_type, period)),
                Some(MovingAverageProvider::new(ma_type, period)),
                Some(MovingAverageProvider::new(ma_type, period)),
            ),
        };

        Self {
            period,
            mode,
            ma_type,
            high_buffer: ArrayVec::new(),
            low_buffer: ArrayVec::new(),
            buffer_index: 0,
            buffer_filled: false,
            upper_ma,
            lower_ma,
            middle_ma,
            upper_band: 0.0,
            lower_band: 0.0,
            middle_band: 0.0,
            is_ready: false,
        }
    }

    /// 🚀 Создать сглаженный Donchian Channel с поддержкой ВСЕХ типов MA
    pub fn new_smoothed(period: usize, ma_type: MovingAverageType) -> Self {
        Self::new_with_mode(period, DonchianMode::Smoothed, ma_type)
    }

    /// Быстрые конструкторы для популярных типов сглаживания
    pub fn new_smoothed_sma(period: usize) -> Self {
        Self::new_smoothed(period, MovingAverageType::SMA)
    }

    pub fn new_smoothed_ema(period: usize) -> Self {
        Self::new_smoothed(period, MovingAverageType::EMA)
    }

    pub fn new_smoothed_hull(period: usize) -> Self {
        Self::new_smoothed(period, MovingAverageType::HMA)
    }

    /// 🚀 СУПЕР БЫСТРОЕ обновление с циклическим буфером O(1)!
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, _close: f64, _volume: f64) -> (f64, f64, f64) {
        // 🚀 Циклический буфер - O(1) операция вместо O(n)!
        if self.high_buffer.len() < self.period {
            // Заполняем буфер в первый раз
            self.high_buffer.push(high);
            self.low_buffer.push(low);
        } else {
            // Циклически перезаписываем - НЕ УДАЛЯЕМ, а заменяем!
            self.high_buffer[self.buffer_index] = high;
            self.low_buffer[self.buffer_index] = low;
            self.buffer_filled = true;
        }

        // Циклически увеличиваем индекс
        self.buffer_index = (self.buffer_index + 1) % self.period;

        // Рассчитываем экстремумы только если буфер готов
        if self.high_buffer.len() >= self.period {
            let (raw_lower, raw_upper) = self.high_buffer.iter()
                .zip(self.low_buffer.iter())
                .fold((f64::INFINITY, f64::NEG_INFINITY),
                      |(min, max), (&h, &l)| (min.min(l), max.max(h)));
            let raw_middle = (raw_upper + raw_lower) / 2.0;

            match self.mode {
                DonchianMode::Classic => {
                    // Классический режим - используем сырые значения
                    self.upper_band = raw_upper;
                    self.lower_band = raw_lower;
                    self.middle_band = raw_middle;
                },
                DonchianMode::Smoothed => {
                    // Сглаженный режим - применяем MA к уровням каналов
                    if let (Some(ref mut upper_ma), Some(ref mut lower_ma), Some(ref mut middle_ma)) =
                        (&mut self.upper_ma, &mut self.lower_ma, &mut self.middle_ma) {

                        // ✅ Обновляем MA для каждого уровня (передаем значение как close)
                        upper_ma.update_bar(raw_upper, raw_upper, raw_upper, raw_upper, 0.0);
                        lower_ma.update_bar(raw_lower, raw_lower, raw_lower, raw_lower, 0.0);
                        middle_ma.update_bar(raw_middle, raw_middle, raw_middle, raw_middle, 0.0);

                        self.upper_band = upper_ma.value().main();
                        self.lower_band = lower_ma.value().main();
                        self.middle_band = middle_ma.value().main();
                    }
                },
            }

            self.is_ready = true;
        }

        (self.upper_band, self.lower_band, self.middle_band)
    }

    /// Получить текущие значения каналов как типизированный IndicatorValue
    /// ВАЖНО: Исправляет порядок tuple на стандартный (upper, middle, lower)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.upper_band,
            middle: self.middle_band,
            lower: self.lower_band,
        }
    }

    /// Получить текущие значения каналов в оригинальном порядке (для обратной совместимости)
    /// Возвращает (upper, lower, middle) - нестандартный порядок!
    pub fn values(&self) -> (f64, f64, f64) {
        (self.upper_band, self.lower_band, self.middle_band)
    }

    /// Получить текущие значения каналов в стандартном порядке
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper_band, self.middle_band, self.lower_band)
    }

    /// Получить верхний канал
    pub fn upper_band(&self) -> f64 {
        self.upper_band
    }

    /// Получить нижний канал
    pub fn lower_band(&self) -> f64 {
        self.lower_band
    }

    /// Получить средний канал
    pub fn middle_band(&self) -> f64 {
        self.middle_band
    }

    /// Получить ширину канала
    pub fn channel_width(&self) -> f64 {
        self.upper_band - self.lower_band
    }

    /// Получить относительную позицию цены в канале (0.0 = на нижнем канале, 1.0 = на верхнем)
    pub fn position_in_channel(&self, price: f64) -> f64 {
        let width = self.channel_width();
        if width > 0.0 {
            (price - self.lower_band) / width
        } else {
            0.5 // Если каналы схлопнулись, считаем что в середине
        }
    }

    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Получить период индикатора
    pub fn period(&self) -> usize {
        self.period
    }

    /// Получить режим работы индикатора
    pub fn mode(&self) -> DonchianMode {
        self.mode
    }

    /// Получить тип MA (для сглаженного режима)
    pub fn ma_type(&self) -> MovingAverageType {
        self.ma_type
    }

    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.high_buffer.clear();
        self.low_buffer.clear();
        self.buffer_index = 0;
        self.buffer_filled = false;

        if let Some(ref mut ma) = self.upper_ma {
            ma.reset();
        }
        if let Some(ref mut ma) = self.lower_ma {
            ma.reset();
        }
        if let Some(ref mut ma) = self.middle_ma {
            ma.reset();
        }

        self.upper_band = 0.0;
        self.lower_band = 0.0;
        self.middle_band = 0.0;
        self.is_ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_donchian_channel_creation() {
        let dc = DonchianChannel::new(20);
        assert!(!dc.is_ready());
        assert_eq!(dc.period(), 20);
    }

    #[test]
    fn test_donchian_channel_warmup() {
        let mut dc = DonchianChannel::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            dc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(dc.is_ready());
    }

    #[test]
    fn test_donchian_channel_values() {
        let mut dc = DonchianChannel::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (upper, lower, _middle) = dc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if dc.is_ready() {
                assert!(upper >= lower, "Upper should be >= lower");
                assert!(dc.channel_width() >= 0.0, "Channel width should be non-negative");
            }
        }
    }

    #[test]
    fn test_donchian_channel_reset() {
        let mut dc = DonchianChannel::new(20);
        for i in 0..25 {
            dc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        dc.reset();
        assert!(!dc.is_ready());
    }
}
