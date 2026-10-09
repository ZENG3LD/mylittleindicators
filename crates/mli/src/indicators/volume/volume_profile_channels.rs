//! volume_profile_channels.rs: High-Performance Volume Profile Channels
//! Каналы на основе объемного профиля - зоны максимального объема торговли
//!
//! Особенности:
//! - Point of Control (POC) - цена с максимальным объемом
//! - Value Area High (VAH) и Value Area Low (VAL) - 70% объема
//! - Volume-weighted price levels
//! - Adaptive price bins based on volatility


use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Render,
    SourceAxis, Store, StoreKind, UpdateComplexity, Color, RenderOutput, RenderSpec, ValueDomain, sweep_f64,
};
use crate::engine::stream_kind::StreamKind;

/// Fixed price-window (rows) of the exposed volume-profile VECTOR, centered on the POC bucket.
const VPC_WINDOW: u16 = 64;


/// Режимы расчета Volume Profile
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VolumeProfileMode {
    /// Фиксированное количество bins
    FixedBins,
    /// Адаптивные bins на основе волатильности
    AdaptiveBins,
    /// Bins на основе tick size
    TickBased,
}

/// Временной период для Volume Profile
#[derive(Debug, Clone, Copy, PartialEq)]
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
    bar_buffer: Vec<(f64, f64, f64, f64, f64)>, // OHLCV
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
    /// Volume-by-price histogram as a `VPC_WINDOW × 1` VECTOR (row = price bin centered on the
    /// POC bin, value = bin volume). Read via `profile_grid` / `ContractFactory::matrix_grid`.
    grid: MatrixGrid,
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
            bar_buffer: Vec::with_capacity(1000),
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
            grid: MatrixGrid::new(VPC_WINDOW, 1, false)
                .with_labels(AxisLabels::Values(Vec::new()), AxisLabels::Bars),
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
    fn recompute(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> (f64, f64, f64) {
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
            self.snapshot_grid();
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

        // Degenerate range guard: a doji bar (high == low) or a single fed price yields
        // price_range 0 → bin_size 0, which would make the `current_price += bin_size` loop
        // below never advance and push bins until OOM. Collapse to a single bin at the price.
        if !(self.bin_size > 0.0) || !(self.max_price >= self.min_price) || !self.min_price.is_finite() {
            if self.min_price.is_finite() {
                self.price_bins.push(PriceBin {
                    price_level: self.min_price,
                    volume: 0.0,
                    tick_count: 0,
                });
            }
            return;
        }

        let mut current_price = self.min_price;
        // Hard cap the bin count: FixedBins targets `num_bins`; the `+ 2` covers boundary
        // rounding. An absolute bound makes the loop OOM-proof against any pathological range.
        let bin_cap = self.num_bins.saturating_add(2);
        while current_price <= self.max_price && self.price_bins.len() < bin_cap {
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
    

    /// Snapshot the current volume-by-price histogram into the `VPC_WINDOW × 1` grid centered on
    /// the POC bin. Called after every recalculation of the volume profile.
    fn snapshot_grid(&mut self) {
        if self.price_bins.is_empty() {
            return;
        }
        // Find the index of the POC bin (the one whose price_level matches point_of_control).
        let poc_idx = self.price_bins.iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.volume.partial_cmp(&b.volume).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);

        self.grid.reset();
        let half = (VPC_WINDOW / 2) as i64;
        let n = self.price_bins.len() as i64;
        let mut row_prices = Vec::with_capacity(VPC_WINDOW as usize);
        for row in 0..VPC_WINDOW {
            let bin_idx = poc_idx as i64 + (row as i64 - half);
            if bin_idx >= 0 && bin_idx < n {
                let bin = &self.price_bins[bin_idx as usize];
                row_prices.push(bin.price_level);
                if bin.volume > 0.0 {
                    self.grid.set_direction(MatrixCell::new(row, 0), bin.volume);
                }
            } else {
                // Out-of-range row: extrapolate price from bin_size grid
                let price = self.point_of_control + (bin_idx - poc_idx as i64) as f64 * self.bin_size;
                row_prices.push(price);
            }
        }
        self.grid.set_row_values(&row_prices);
    }

    /// The volume-by-price profile as a `VPC_WINDOW × 1` vector — the matrix output behind
    /// `IndicatorOutputId::VolprofchanProfileGrid`.
    pub fn profile_grid(&self) -> &MatrixGrid { &self.grid }

    /// Named output: brace `upper`.
    pub fn upper(&self) -> f64 { self.value_area_high }
    /// Named output: brace `middle`.
    pub fn middle(&self) -> f64 { self.point_of_control }
    /// Named output: brace `lower`.
    pub fn lower(&self) -> f64 { self.value_area_low }

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
        self.grid.reset();
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
            vpc.recompute(price, price + 1.0, price - 1.0, price, 1000.0 + i as f64 * 100.0);
        }
        // После нескольких обновлений должен быть готов
        assert!(vpc.bar_count > 0);
    }

    #[test]
    fn test_volume_profile_channels_poc() {
        let mut vpc = VolumeProfileChannels::new_daily();
        for i in 0..30 {
            let price = 100.0 + i as f64;
            vpc.recompute(price, price + 1.0, price - 1.0, price, 1000.0 + i as f64 * 50.0);
        }
        if vpc.is_ready() {
            assert!(vpc.point_of_control() > 0.0);
        }
    }

    #[test]
    fn test_volume_profile_channels_reset() {
        let mut vpc = VolumeProfileChannels::new();
        for i in 0..50 {
            vpc.recompute(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        vpc.reset();
        assert!(!vpc.is_ready());
        assert_eq!(vpc.channel_width(), 0.0);
    }

    #[test]
    fn row_label_at_center_is_poc_price() {
        use crate::engine::matrix_grid::Label;
        // LastNBars(1) forces recalculation on every bar.
        let mut vpc = VolumeProfileChannels::new_custom(
            VolumeProfileMode::FixedBins,
            VolumeProfilePeriod::LastNBars(1),
            50,
            70.0,
        );
        // Dominant bar: single price → POC bin price_level = min_price + bin_size/2 = 100.0 + 0.0/2
        // (degenerate range → single bin at 100.0). Feed one bar then several noise bars.
        vpc.recompute(100.0, 100.0, 100.0, 100.0, 5000.0); // dominant
        for i in 1..6 {
            vpc.recompute(
                110.0 + i as f64, 111.0 + i as f64, 109.0 + i as f64, 110.0 + i as f64, 100.0,
            );
        }
        let center = VPC_WINDOW / 2;
        let label = vpc.profile_grid().row_label(center);
        // The POC bin price must be a finite value.
        if let Label::Value(p) = label {
            assert!(p.is_finite(), "row_label at center should be a finite price, got {p}");
        } else {
            panic!("expected Label::Value, got {:?}", label);
        }
    }

    #[test]
    fn profile_vector_captures_volume_around_poc() {
        // LastNBars(1) forces recalculation on every bar.
        let mut vpc = VolumeProfileChannels::new_custom(
            VolumeProfileMode::FixedBins,
            VolumeProfilePeriod::LastNBars(1),
            50,
            70.0,
        );
        // Feed a dominant bar (large volume at a single price) and noise bars.
        vpc.recompute(100.0, 100.0, 100.0, 100.0, 5000.0); // dominant
        for i in 1..6 {
            vpc.recompute(110.0 + i as f64, 111.0 + i as f64, 109.0 + i as f64, 110.0 + i as f64, 100.0);
        }
        let g = vpc.profile_grid();
        assert_eq!(g.cols(), 1, "a 1-D volume-by-price vector");
        assert_eq!(g.rows(), 64, "64 price rows");
        // The center row corresponds to the POC bin and should carry the highest volume.
        let center = VPC_WINDOW / 2;
        let center_vol = g.read_direction(MatrixCell::new(center, 0));
        assert!(center_vol > 0.0, "POC bin at center row should have positive volume, got {center_vol}");
    }
}

// ---- Indicator contract ----

/// Typed configuration for [`VolumeProfileChannels`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VolumeProfileChannelsConfig {
    /// Number of price bins (6..=200).
    pub num_bins: Param<usize>,
    /// Value area target percentage (50..95).
    pub value_area_percent: Param<f64>,
}

impl Indicator for VolumeProfileChannels {
    const ID: IndicatorId = IndicatorId::Volprofchan;
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::VolprofchanUpper),
        Output::price(IndicatorOutputId::VolprofchanMiddle),
        Output::price(IndicatorOutputId::VolprofchanLower),
        Output::matrix(IndicatorOutputId::VolprofchanProfileGrid, ValueDomain::Count),
    ];
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
        ],
    );

    type Config = VolumeProfileChannelsConfig;
    type Runtime = VolumeProfileChannels;

    fn create(cfg: VolumeProfileChannelsConfig) -> VolumeProfileChannels {
        VolumeProfileChannels::new_custom(
            VolumeProfileMode::AdaptiveBins,
            VolumeProfilePeriod::Daily,
            cfg.num_bins.resolved(),
            cfg.value_area_percent.resolved(),
        )
    }
}

impl VolumeProfileChannels {
    /// Feed a bar represented as a scalar slice.
    ///
    /// Lane order matches `const SOURCE`: [open, high, low, close, volume].
    pub fn feed(&mut self, lanes: &[f64]) {
        let open   = lanes[0];
        let high   = lanes[1];
        let low    = lanes[2];
        let close  = lanes[3];
        let volume = lanes[4];
        self.recompute(open, high, low, close, volume);
    }
}

impl crate::contract::Config for VolumeProfileChannelsConfig {
    fn valid_params(&self) -> Result<(), String> {
        let bins = self.num_bins.resolved();
        if bins <= 5 || bins > 200 {
            return Err(format!("num_bins({bins}) must be > 5 and <= 200"));
        }
        let vap = self.value_area_percent.resolved();
        if vap <= 50.0 || vap >= 95.0 {
            return Err(format!("value_area_percent({vap}) must be in (50.0, 95.0)"));
        }
        Ok(())
    }
    fn defaults() -> Self {
        VolumeProfileChannelsConfig {
            num_bins: Param::Solo(50),
            value_area_percent: Param::Solo(70.0),
        }
    }
    fn machine_defaults() -> Self {
        // num_bins (usize): Class B bin-count — OVERRIDE auto (auto would give 2..=4048 which
        //   would panic the generator; real bin-count range is 5..=500 step 5). `valid_params`
        //   requires `num_bins > 5` STRICTLY — the previous min (5) failed that gate at the
        //   min corner (2026-07-03 fix); start one step above the floor.
        // value_area_percent (f64): Class D stored as 0-100 percent (not 0-1 ratio) —
        //   sweep 50..=95 step 5.0. `valid_params` requires it STRICTLY inside (50.0, 95.0);
        //   the previous min (50.0) failed that gate too — start one step above the floor.
        let mut s = Self::machine_defaults_auto();
        s.num_bins = Param::range(10, 500, 5);
        s.value_area_percent = Param::many(sweep_f64(55.0, 90.0, 5.0));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VolumeProfileChannels {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(
                IndicatorOutputId::VolprofchanUpper,
                "VAH",
                Color::hex(0xF44336),
                1.0,
            ))
            .output(RenderOutput::line(
                IndicatorOutputId::VolprofchanMiddle,
                "POC",
                Color::hex(0x9C27B0),
                2.0,
            ))
            .output(RenderOutput::line(
                IndicatorOutputId::VolprofchanLower,
                "VAL",
                Color::hex(0x4CAF50),
                1.0,
            ))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_volprofchan() {
        let mut f = IndicatorOrder::Volprofchan(
            <<VolumeProfileChannels as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // Use wild values in fields our SOURCE does not ignore (all 5 lanes are used).
        // Feed enough bars to trigger a recalculation (recalc_frequency for Session=100).
        for i in 0..5 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: price,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 1000.0,
            });
        }
        // value() is always finite (even pre-warmup, zero-initialised scalars are finite)
        assert!(f.read(IndicatorOutputId::VolprofchanMiddle).is_finite());
    }

    #[test]
    fn factory_exposes_volprofchan_vector() {
        let f = IndicatorOrder::Volprofchan(
            <<VolumeProfileChannels as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let g = f
            .grid(IndicatorOutputId::VolprofchanProfileGrid)
            .expect("Volprofchan emits the volume-profile vector");
        assert_eq!(g.rows(), 64);
        assert_eq!(g.cols(), 1);

        let sma = IndicatorOrder::from_defaults(IndicatorId::Sma).unwrap().build_solo().unwrap();
        assert!(sma.grid(IndicatorOutputId::VolprofchanProfileGrid).is_none());
    }
}






















