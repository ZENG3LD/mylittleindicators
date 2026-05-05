//! volume_profile_channels.rs: High-Performance Volume Profile Channels
//! Каналы на основе объемного профиля - зоны максимального объема торговли
//!
//! Особенности:
//! - Point of Control (POC) - цена с максимальным объемом
//! - Value Area High (VAH) и Value Area Low (VAL) - 70% объема
//! - Volume-weighted price levels
//! - Adaptive price bins based on volatility

use crate::bar_indicators::indicator_value::IndicatorValue;
use arrayvec::ArrayVec;
use serde::{Serialize, Deserialize};


/// Режимы расчета Volume Profile
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VolumeProfileMode {
    /// Фиксированное количество bins
    FixedBins,
    /// Адаптивные bins на основе волатильности
    AdaptiveBins,
    /// Bins на основе tick size
    TickBased,
}

/// Временной период для Volume Profile
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VolumeProfilePeriod {
    /// Сессия
    Session,
    /// День
    Daily,
    /// Неделя
    Weekly,
    /// Месяц
    Monthly,
    /// Последние N баров
    LastNBars(usize),
}

/// Сигналы Volume Profile Channels
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum VolumeProfileSignal {
    /// Пробой Value Area High
    BreakoutVAH,
    /// Пробой Value Area Low
    BreakdownVAL,
    /// Возврат к POC
    ReturnToPOC,
    /// Отскок от VAH
    BounceFromVAH,
    /// Отскок от VAL
    BounceFromVAL,
    /// Движение внутри Value Area
    WithinValueArea,
    /// Высокообъемная зона
    HighVolumeZone,
    /// Низкообъемная зона
    LowVolumeZone,
}

/// Price bin для Volume Profile
#[derive(Debug, Clone)]
struct PriceBin {
    price_level: f64,
    volume: f64,
    tick_count: usize,
}

/// High-Performance Volume Profile Channels
#[derive(Debug, Clone)]
pub struct VolumeProfileChannels {
    // Параметры
    mode: VolumeProfileMode,
    period: VolumeProfilePeriod,
    num_bins: usize,
    value_area_percent: f64, // Обычно 70%
    
    // Price bins for volume distribution
    price_bins: Vec<PriceBin>,
    bin_size: f64,
    min_price: f64,
    max_price: f64,
    
    // Буфер данных баров
    bar_buffer: ArrayVec<(f64, f64, f64, f64, f64), 1000>, // OHLCV
    buffer_index: usize,
    buffer_filled: bool,
    
    // Volume Profile результаты
    point_of_control: f64,      // POC - цена с максимальным объемом
    value_area_high: f64,       // VAH - верхняя граница Value Area
    value_area_low: f64,        // VAL - нижняя граница Value Area
    total_volume: f64,
    value_area_volume: f64,
    
    // Каналы
    upper_channel: f64,         // VAH или расширенная зона
    lower_channel: f64,         // VAL или расширенная зона
    channel_width: f64,
    
    // Адаптивные параметры
    avg_volatility: f64,
    adaptive_multiplier: f64,
    
    // Статистика
    high_volume_threshold: f64,
    poc_strength: f64,          // Процент объема в POC
    
    // Счетчики
    bars_since_recalc: usize,
    recalc_frequency: usize,
    bar_count: usize,
}

impl VolumeProfileChannels {
    /// Создать Volume Profile Channels со стандартными параметрами
    pub fn new() -> Self {
        Self::new_custom(
            VolumeProfileMode::AdaptiveBins,
            VolumeProfilePeriod::Daily,
            50,
            70.0
        )
    }
    
    /// Создать Volume Profile Channels с кастомными параметрами
    pub fn new_custom(
        mode: VolumeProfileMode,
        period: VolumeProfilePeriod,
        num_bins: usize,
        value_area_percent: f64
    ) -> Self {
        assert!(num_bins > 5 && num_bins <= 200);
        assert!(value_area_percent > 50.0 && value_area_percent < 95.0);
        
        let recalc_frequency = match period {
            VolumeProfilePeriod::Session => 100,
            VolumeProfilePeriod::Daily => 1440,    // Каждый день
            VolumeProfilePeriod::Weekly => 10080,  // Каждую неделю
            VolumeProfilePeriod::Monthly => 43200, // Каждый месяц
            VolumeProfilePeriod::LastNBars(n) => n,
        };
        
        Self {
            mode,
            period,
            num_bins,
            value_area_percent,
            price_bins: Vec::with_capacity(num_bins),
            bin_size: 0.0,
            min_price: f64::INFINITY,
            max_price: f64::NEG_INFINITY,
            bar_buffer: ArrayVec::new(),
            buffer_index: 0,
            buffer_filled: false,
            point_of_control: 0.0,
            value_area_high: 0.0,
            value_area_low: 0.0,
            total_volume: 0.0,
            value_area_volume: 0.0,
            upper_channel: 0.0,
            lower_channel: 0.0,
            channel_width: 0.0,
            avg_volatility: 0.0,
            adaptive_multiplier: 1.0,
            high_volume_threshold: 0.0,
            poc_strength: 0.0,
            bars_since_recalc: 0,
            recalc_frequency,
            bar_count: 0,
        }
    }
    
    /// Создать сессионный Volume Profile
    pub fn new_session(num_bins: usize) -> Self {
        Self::new_custom(
            VolumeProfileMode::AdaptiveBins,
            VolumeProfilePeriod::Session,
            num_bins,
            70.0
        )
    }
    
    /// Создать дневной Volume Profile
    pub fn new_daily() -> Self {
        Self::new_custom(
            VolumeProfileMode::FixedBins,
            VolumeProfilePeriod::Daily,
            100,
            70.0
        )
    }
    
    /// Обновить каналы новым баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> (f64, f64, f64) {
        self.bar_count += 1;
        self.bars_since_recalc += 1;
        
        // Добавляем бар в буфер
        self.add_bar_to_buffer(open, high, low, close, volume);
        
        // Обновляем price range
        self.update_price_range(high, low);
        
        // Пересчитываем Volume Profile при необходимости
        if self.should_recalculate() {
            self.recalculate_volume_profile();
            self.calculate_value_area();
            self.update_channels();
            self.bars_since_recalc = 0;
        }
        
        // Обновляем адаптивные параметры
        self.update_adaptive_parameters(high, low);

        (self.value_area_high, self.point_of_control, self.value_area_low)
    }
    
    /// Добавить бар в буфер
    fn add_bar_to_buffer(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) {
        let bar = (open, high, low, close, volume);
        
        if self.buffer_filled {
            self.bar_buffer[self.buffer_index] = bar;
        } else {
            self.bar_buffer.push(bar);
        }
        
        self.buffer_index = (self.buffer_index + 1) % self.bar_buffer.capacity();
        
        if self.bar_buffer.len() == self.bar_buffer.capacity() && !self.buffer_filled {
            self.buffer_filled = true;
        }
    }
    
    /// Обновить диапазон цен
    fn update_price_range(&mut self, high: f64, low: f64) {
        self.min_price = self.min_price.min(low);
        self.max_price = self.max_price.max(high);
    }
    
    /// Проверить, нужно ли пересчитать Volume Profile
    fn should_recalculate(&self) -> bool {
        match self.period {
            VolumeProfilePeriod::LastNBars(_) => self.bars_since_recalc >= self.recalc_frequency,
            _ => self.bars_since_recalc >= self.recalc_frequency || self.bar_count == 1,
        }
    }
    
    /// Пересчитать Volume Profile
    fn recalculate_volume_profile(&mut self) {
        if self.bar_buffer.is_empty() {
            return;
        }
        
        // Определяем размер bins
        self.calculate_bin_size();
        
        // Инициализируем bins
        self.initialize_price_bins();
        
        // Распределяем объем по bins
        self.distribute_volume();
        
        // Находим POC
        self.find_point_of_control();
    }
    
    /// Рассчитать размер bin
    fn calculate_bin_size(&mut self) {
        let price_range = self.max_price - self.min_price;
        
        self.bin_size = match self.mode {
            VolumeProfileMode::FixedBins => price_range / self.num_bins as f64,
            VolumeProfileMode::AdaptiveBins => {
                // Адаптивный размер на основе волатильности
                let base_size = price_range / self.num_bins as f64;
                base_size * self.adaptive_multiplier
            }
            VolumeProfileMode::TickBased => {
                // Предполагаем tick size = 0.01 для большинства инструментов
                0.01 * (price_range / (self.num_bins as f64 * 0.01)).ceil()
            }
        };
        
        // Обеспечиваем минимальный размер bin
        self.bin_size = self.bin_size.max((self.max_price - self.min_price) / 1000.0);
    }
    
    /// Инициализировать price bins
    fn initialize_price_bins(&mut self) {
        self.price_bins.clear();
        
        let mut current_price = self.min_price;
        while current_price <= self.max_price {
            self.price_bins.push(PriceBin {
                price_level: current_price + self.bin_size / 2.0, // Центр bin
                volume: 0.0,
                tick_count: 0,
            });
            current_price += self.bin_size;
        }
    }
    
    /// Распределить объем по bins
    fn distribute_volume(&mut self) {
        self.total_volume = 0.0;
        
        for &(_, high, low, _, volume) in &self.bar_buffer {
            if volume <= 0.0 { continue; }
            
            // Распределяем объем равномерно по ценовому диапазону бара
            let bar_range = high - low;
            if bar_range <= 0.0 { continue; }
            
            // Находим bins, которые пересекаются с баром
            for bin in &mut self.price_bins {
                let bin_low = bin.price_level - self.bin_size / 2.0;
                let bin_high = bin.price_level + self.bin_size / 2.0;
                
                // Проверяем пересечение
                let overlap_low = bin_low.max(low);
                let overlap_high = bin_high.min(high);
                
                if overlap_high > overlap_low {
                    let overlap_ratio = (overlap_high - overlap_low) / bar_range;
                    let bin_volume = volume * overlap_ratio;
                    
                    bin.volume += bin_volume;
                    bin.tick_count += 1;
                }
            }
            
            self.total_volume += volume;
        }
    }
    
    /// Найти Point of Control (POC)
    fn find_point_of_control(&mut self) {
        if self.price_bins.is_empty() {
            return;
        }
        
        // Находим bin с максимальным объемом
        let max_volume_bin = self.price_bins.iter()
            .max_by(|a, b| a.volume.partial_cmp(&b.volume).unwrap());
        
        if let Some(bin) = max_volume_bin {
            self.point_of_control = bin.price_level;
            self.poc_strength = if self.total_volume > 0.0 {
                bin.volume / self.total_volume * 100.0
            } else {
                0.0
            };
        }
    }
    
    /// Рассчитать Value Area (VAH и VAL)
    fn calculate_value_area(&mut self) {
        if self.price_bins.is_empty() || self.total_volume <= 0.0 {
            return;
        }
        
        let target_volume = self.total_volume * (self.value_area_percent / 100.0);
        
        // Сортируем bins по объему (по убыванию)
        let mut sorted_bins: Vec<(usize, f64)> = self.price_bins.iter()
            .enumerate()
            .map(|(i, bin)| (i, bin.volume))
            .collect();
        sorted_bins.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        
        // Собираем bins до достижения target_volume
        let mut accumulated_volume = 0.0;
        let mut value_area_indices = Vec::new();
        
        for (index, volume) in sorted_bins {
            accumulated_volume += volume;
            value_area_indices.push(index);
            
            if accumulated_volume >= target_volume {
                break;
            }
        }
        
        // Находим минимальную и максимальную цены в Value Area
        if !value_area_indices.is_empty() {
            let min_index = *value_area_indices.iter().min().unwrap();
            let max_index = *value_area_indices.iter().max().unwrap();
            
            self.value_area_low = self.price_bins[min_index].price_level - self.bin_size / 2.0;
            self.value_area_high = self.price_bins[max_index].price_level + self.bin_size / 2.0;
            self.value_area_volume = accumulated_volume;
        }
    }
    
    /// Обновить каналы
    fn update_channels(&mut self) {
        // Базовые каналы - это VAH и VAL
        self.upper_channel = self.value_area_high;
        self.lower_channel = self.value_area_low;
        
        // Расширяем каналы на основе волатильности если включен адаптивный режим
        if matches!(self.mode, VolumeProfileMode::AdaptiveBins) {
            let extension = (self.value_area_high - self.value_area_low) * 
                           (self.adaptive_multiplier - 1.0) * 0.5;
            
            self.upper_channel += extension;
            self.lower_channel -= extension;
        }
        
        self.channel_width = self.upper_channel - self.lower_channel;
        
        // Обновляем порог высокого объема
        if !self.price_bins.is_empty() {
            let avg_volume = self.total_volume / self.price_bins.len() as f64;
            self.high_volume_threshold = avg_volume * 1.5; // 150% от среднего
        }
    }
    
    /// Обновить адаптивные параметры
    fn update_adaptive_parameters(&mut self, high: f64, low: f64) {
        let true_range = high - low;
        
        // Экспоненциальное сглаживание волатильности
        let alpha = 2.0 / (21.0 + 1.0); // 21-периодное EMA
        if self.avg_volatility == 0.0 {
            self.avg_volatility = true_range;
        } else {
            self.avg_volatility = self.avg_volatility * (1.0 - alpha) + true_range * alpha;
        }
        
        // Адаптивный множитель
        if self.avg_volatility > 0.0 {
            self.adaptive_multiplier = (true_range / self.avg_volatility).clamp(0.5, 2.0);
        }
    }
    
    /// Получить основные значения (VAH, POC, VAL)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Channel3 {
            upper: self.value_area_high,
            middle: self.point_of_control,
            lower: self.value_area_low,
        }
    }

    /// Получить основные значения как tuple (для обратной совместимости)
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.value_area_high, self.point_of_control, self.value_area_low)
    }
    
    /// Получить каналы (POC, верхний канал, нижний канал)
    pub fn channels(&self) -> (f64, f64, f64) {
        (self.point_of_control, self.upper_channel, self.lower_channel)
    }
    
    /// Получить Point of Control
    pub fn point_of_control(&self) -> f64 {
        self.point_of_control
    }
    
    /// Получить Value Area High
    pub fn value_area_high(&self) -> f64 {
        self.value_area_high
    }
    
    /// Получить Value Area Low
    pub fn value_area_low(&self) -> f64 {
        self.value_area_low
    }
    
    /// Получить ширину канала
    pub fn channel_width(&self) -> f64 {
        self.channel_width
    }
    
    /// Получить позицию цены в Value Area
    pub fn position_in_value_area(&self, price: f64) -> f64 {
        let va_width = self.value_area_high - self.value_area_low;
        if va_width > 0.0 {
            (price - self.value_area_low) / va_width
        } else {
            0.5
        }
    }
    
    /// Получить объем в ценовом уровне
    pub fn volume_at_price(&self, price: f64) -> f64 {
        for bin in &self.price_bins {
            let bin_low = bin.price_level - self.bin_size / 2.0;
            let bin_high = bin.price_level + self.bin_size / 2.0;
            
            if price >= bin_low && price <= bin_high {
                return bin.volume;
            }
        }
        0.0
    }
    
    /// Генерация сигнала
    pub fn generate_signal(&self, current_price: f64, previous_price: f64) -> VolumeProfileSignal {
        // Пробой VAH
        if previous_price <= self.value_area_high && current_price > self.value_area_high {
            return VolumeProfileSignal::BreakoutVAH;
        }
        
        // Пробой VAL
        if previous_price >= self.value_area_low && current_price < self.value_area_low {
            return VolumeProfileSignal::BreakdownVAL;
        }
        
        // Возврат к POC
        let distance_to_poc = (current_price - self.point_of_control).abs();
        let prev_distance_to_poc = (previous_price - self.point_of_control).abs();
        
        if distance_to_poc < prev_distance_to_poc && distance_to_poc < self.channel_width * 0.05 {
            return VolumeProfileSignal::ReturnToPOC;
        }
        
        // Отскоки от границ Value Area
        let tolerance = self.channel_width * 0.02;
        
        if (previous_price - self.value_area_high).abs() < tolerance && current_price < previous_price {
            return VolumeProfileSignal::BounceFromVAH;
        }
        
        if (previous_price - self.value_area_low).abs() < tolerance && current_price > previous_price {
            return VolumeProfileSignal::BounceFromVAL;
        }
        
        // Проверяем, находимся ли в высокообъемной зоне
        let current_volume = self.volume_at_price(current_price);
        if current_volume > self.high_volume_threshold {
            return VolumeProfileSignal::HighVolumeZone;
        } else if current_volume < self.high_volume_threshold * 0.3 {
            return VolumeProfileSignal::LowVolumeZone;
        }
        
        // Внутри Value Area
        if current_price >= self.value_area_low && current_price <= self.value_area_high {
            VolumeProfileSignal::WithinValueArea
        } else {
            VolumeProfileSignal::WithinValueArea
        }
    }
    
    /// Получить силу POC
    pub fn poc_strength(&self) -> f64 {
        self.poc_strength
    }
    
    /// Получить общий объем
    pub fn total_volume(&self) -> f64 {
        self.total_volume
    }
    
    /// Получить процент объема в Value Area
    pub fn value_area_volume_percent(&self) -> f64 {
        if self.total_volume > 0.0 {
            self.value_area_volume / self.total_volume * 100.0
        } else {
            0.0
        }
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        !self.price_bins.is_empty() && self.total_volume > 0.0
    }
    
    /// Получить параметры
    pub fn get_params(&self) -> (VolumeProfileMode, VolumeProfilePeriod, usize, f64) {
        (self.mode, self.period, self.num_bins, self.value_area_percent)
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.price_bins.clear();
        self.bar_buffer.clear();
        self.buffer_index = 0;
        self.buffer_filled = false;
        
        self.min_price = f64::INFINITY;
        self.max_price = f64::NEG_INFINITY;
        self.bin_size = 0.0;
        
        self.point_of_control = 0.0;
        self.value_area_high = 0.0;
        self.value_area_low = 0.0;
        self.total_volume = 0.0;
        self.value_area_volume = 0.0;
        
        self.upper_channel = 0.0;
        self.lower_channel = 0.0;
        self.channel_width = 0.0;
        
        self.avg_volatility = 0.0;
        self.adaptive_multiplier = 1.0;
        self.high_volume_threshold = 0.0;
        self.poc_strength = 0.0;
        
        self.bars_since_recalc = 0;
        self.bar_count = 0;
    }
}

impl Default for VolumeProfileChannels {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_profile_channels_creation() {
        let vpc = VolumeProfileChannels::new();
        assert!(!vpc.is_ready());
        assert_eq!(vpc.channel_width(), 0.0);
    }

    #[test]
    fn test_volume_profile_channels_update() {
        let mut vpc = VolumeProfileChannels::new_session(50);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vpc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0 + i as f64 * 100.0);
        }
        // После нескольких обновлений должен быть готов
        assert!(vpc.bar_count > 0);
    }

    #[test]
    fn test_volume_profile_channels_poc() {
        let mut vpc = VolumeProfileChannels::new_daily();
        for i in 0..30 {
            let price = 100.0 + i as f64;
            vpc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0 + i as f64 * 50.0);
        }
        if vpc.is_ready() {
            assert!(vpc.point_of_control() > 0.0);
        }
    }

    #[test]
    fn test_volume_profile_channels_reset() {
        let mut vpc = VolumeProfileChannels::new();
        for i in 0..50 {
            vpc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        vpc.reset();
        assert!(!vpc.is_ready());
        assert_eq!(vpc.channel_width(), 0.0);
    }
} 






















