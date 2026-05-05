//! Volume Profile Indicator
//! Анализирует распределение объема по ценовым уровням в рамках сессии
//! НЕ скользящий! Сбрасывается каждую сессию/период

use arrayvec::ArrayVec;
use crate::types::Bar;

/// Ценовой уровень с объемом
#[derive(Debug, Clone, Copy)]
pub struct PriceLevel {
    pub price: f64,
    pub volume: f64,
}

/// Volume Profile индикатор (сессионный)
#[derive(Debug, Clone)]
pub struct VolumeProfile {
    /// Ценовые уровни (фиксированный размер)
    levels: ArrayVec<PriceLevel, 1024>,
    /// Размер тика для группировки цен
    tick_size: f64,
    /// Общий объем за сессию
    total_volume: f64,
    /// Время начала сессии
    session_start_time: i64,
    /// Длительность сессии в секундах
    session_duration: i64,
    /// Количество обработанных баров в текущей сессии
    bars_count: usize,
    /// Флаг готовности
    ready: bool,
    /// Множитель для конвертации цены в индекс
    price_multiplier: f64,
}

impl VolumeProfile {
    /// Создать новый Volume Profile
    /// tick_size - размер тика для группировки
    /// session_duration - длительность сессии в секундах (0 = без автосброса)
    pub fn new(tick_size: f64, session_duration: i64) -> Self {
        let price_multiplier = 1.0 / tick_size;
        Self {
            levels: ArrayVec::new(),
            tick_size,
            total_volume: 0.0,
            session_start_time: 0,
            session_duration,
            bars_count: 0,
            ready: false,
            price_multiplier,
        }
    }
    
    /// Обновить профиль новым Bar
    pub fn update(&mut self, volume_bar: &Bar) -> bool {
        // Проверяем нужно ли начать новую сессию
        if self.should_reset_session(volume_bar.time) {
            self.reset_session(volume_bar.time);
        }
        
        if volume_bar.volume <= 0.0 {
            return false;
        }
        
        // Распределяем объем по ценовым уровням
        self.distribute_volume(volume_bar);
        
        self.total_volume += volume_bar.volume;
        self.bars_count += 1;
        self.ready = true;
        
        true
    }
    
    /// Проверить нужно ли сбросить сессию
    fn should_reset_session(&self, current_time: i64) -> bool {
        if self.session_duration <= 0 {
            return false; // Без автосброса
        }
        
        if self.session_start_time == 0 {
            return true; // Первая сессия
        }
        
        current_time >= self.session_start_time + self.session_duration
    }
    
    /// Сбросить сессию
    fn reset_session(&mut self, start_time: i64) {
        self.levels.clear();
        self.total_volume = 0.0;
        self.session_start_time = start_time;
        self.bars_count = 0;
        self.ready = false;
    }
    
    /// Распределить объем по ценовым уровням
    fn distribute_volume(&mut self, volume_bar: &Bar) {
        // Простая модель: основной объем на OHLC точках
        let ohlc_volume = volume_bar.volume * 0.25; // 25% на каждую точку
        
        self.add_volume_at_price(volume_bar.open, ohlc_volume);
        self.add_volume_at_price(volume_bar.high, ohlc_volume);
        self.add_volume_at_price(volume_bar.low, ohlc_volume);
        self.add_volume_at_price(volume_bar.close, ohlc_volume);
    }
    
    /// Добавить объем на ценовом уровне
    fn add_volume_at_price(&mut self, price: f64, volume: f64) {
        let rounded_price = self.round_to_tick(price);
        
        // Ищем существующий уровень
        if let Some(level) = self.levels.iter_mut().find(|l| (l.price - rounded_price).abs() < self.tick_size * 0.5) {
            level.volume += volume;
        } else if !self.levels.is_full() {
            // Добавляем новый уровень
            self.levels.push(PriceLevel {
                price: rounded_price,
                volume,
            });
        }
        // Если буфер полный - игнорируем (можно добавить логику замещения)
    }
    
    /// Округлить цену до тика
    fn round_to_tick(&self, price: f64) -> f64 {
        // Используем price_multiplier для лучшей производительности (умножение быстрее деления)
        (price * self.price_multiplier).round() / self.price_multiplier
    }

    /// Получить эффективный индекс цены для хеширования
    #[allow(dead_code)]
    fn price_to_index(&self, price: f64) -> i64 {
        (price * self.price_multiplier).round() as i64
    }

    /// Конвертировать индекс обратно в цену
    #[allow(dead_code)]
    fn index_to_price(&self, index: i64) -> f64 {
        index as f64 / self.price_multiplier
    }
    
    /// Получить POC (Point of Control) - уровень с максимальным объемом
    pub fn get_poc(&self) -> Option<PriceLevel> {
        self.levels.iter()
            .max_by(|a, b| a.volume.partial_cmp(&b.volume).unwrap())
            .copied()
    }
    
    /// Получить объем на определенном ценовом уровне
    pub fn volume_at_price(&self, price: f64) -> f64 {
        let rounded_price = self.round_to_tick(price);
        
        self.levels.iter()
            .find(|l| (l.price - rounded_price).abs() < self.tick_size * 0.5)
            .map(|l| l.volume)
            .unwrap_or(0.0)
    }
    
    /// Получить топ N ценовых уровней по объему
    pub fn top_volume_levels(&self, n: usize) -> Vec<PriceLevel> {
        let mut levels = self.levels.to_vec();
        levels.sort_by(|a, b| b.volume.partial_cmp(&a.volume).unwrap());
        levels.truncate(n);
        levels
    }
    
    /// Получить все уровни
    pub fn all_levels(&self) -> &[PriceLevel] {
        &self.levels
    }
    
    /// Получить уровни в ценовом диапазоне
    pub fn levels_in_range(&self, min_price: f64, max_price: f64) -> Vec<PriceLevel> {
        self.levels.iter()
            .filter(|l| l.price >= min_price && l.price <= max_price)
            .copied()
            .collect()
    }
    
    /// Получить общий объем сессии
    pub fn total_volume(&self) -> f64 {
        self.total_volume
    }
    
    /// Получить количество ценовых уровней
    pub fn levels_count(&self) -> usize {
        self.levels.len()
    }
    
    /// Получить ценовой диапазон профиля
    pub fn price_range(&self) -> Option<(f64, f64)> {
        if self.levels.is_empty() {
            return None;
        }
        
        let min_price = self.levels.iter().map(|l| l.price).fold(f64::INFINITY, f64::min);
        let max_price = self.levels.iter().map(|l| l.price).fold(f64::NEG_INFINITY, f64::max);
        
        Some((min_price, max_price))
    }
    
    /// Получить время сессии
    pub fn session_info(&self) -> (i64, i64, usize) {
        (self.session_start_time, self.session_duration, self.bars_count)
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    
    /// Принудительно сбросить профиль
    pub fn reset(&mut self) {
        self.levels.clear();
        self.total_volume = 0.0;
        self.session_start_time = 0;
        self.bars_count = 0;
        self.ready = false;
    }

    /// Получить размер тика
    pub fn get_tick_size(&self) -> f64 {
        self.tick_size
    }

    /// Установить новый размер тика (пересчитывает price_multiplier)
    pub fn set_tick_size(&mut self, new_tick_size: f64) {
        assert!(new_tick_size > 0.0, "Tick size must be positive");
        self.tick_size = new_tick_size;
        self.price_multiplier = 1.0 / new_tick_size;
        // Очищаем уровни, так как они не будут соответствовать новому размеру тика
        self.reset();
    }

    /// Получить множитель цены
    pub fn get_price_multiplier(&self) -> f64 {
        self.price_multiplier
    }

    /// Получить длительность сессии
    pub fn get_session_duration(&self) -> i64 {
        self.session_duration
    }

    /// Установить новую длительность сессии
    pub fn set_session_duration(&mut self, new_duration: i64) {
        self.session_duration = new_duration;
    }

    /// Получить полную конфигурацию
    pub fn get_config(&self) -> VolumeProfileConfig {
        VolumeProfileConfig {
            tick_size: self.tick_size,
            session_duration: self.session_duration,
            price_multiplier: self.price_multiplier,
        }
    }

    /// Установить новую конфигурацию
    pub fn set_config(&mut self, config: VolumeProfileConfig) {
        self.tick_size = config.tick_size;
        self.session_duration = config.session_duration;
        self.price_multiplier = config.price_multiplier;
        // Проверяем консистентность
        if (self.price_multiplier - 1.0 / self.tick_size).abs() > 1e-10 {
            self.price_multiplier = 1.0 / self.tick_size;
        }
        self.reset();
    }

    /// Получить подробную статистику профиля
    pub fn get_stats(&self) -> VolumeProfileStats {
        let poc = self.get_poc();
        let range = self.price_range();
        let avg_volume = if !self.levels.is_empty() {
            self.total_volume / self.levels.len() as f64
        } else {
            0.0
        };

        VolumeProfileStats {
            total_volume: self.total_volume,
            levels_count: self.levels.len(),
            price_range: range,
            average_volume_per_level: avg_volume,
            poc,
            session_bars_count: self.bars_count,
            session_start_time: self.session_start_time,
        }
    }

    /// Получить уровни поддержки и сопротивления
    pub fn get_support_resistance_levels(&self, min_volume_threshold: f64) -> Vec<PriceLevel> {
        self.levels.iter()
            .filter(|level| level.volume >= min_volume_threshold)
            .copied()
            .collect()
    }

    /// Получить объемный дисбаланс между покупками и продажами (приблизительно)
    pub fn get_volume_imbalance(&self) -> f64 {
        if let Some(poc) = self.get_poc() {
            // Простая эвристика: объем выше POC vs ниже POC
            let above_poc: f64 = self.levels.iter()
                .filter(|level| level.price > poc.price)
                .map(|level| level.volume)
                .sum();
            let below_poc: f64 = self.levels.iter()
                .filter(|level| level.price < poc.price)
                .map(|level| level.volume)
                .sum();
            
            if above_poc + below_poc > 0.0 {
                (above_poc - below_poc) / (above_poc + below_poc)
            } else {
                0.0
            }
        } else {
            0.0
        }
    }
}

/// Статистика Volume Profile
#[derive(Debug, Clone)]
pub struct VolumeProfileStats {
    pub total_volume: f64,
    pub levels_count: usize,
    pub price_range: Option<(f64, f64)>,
    pub average_volume_per_level: f64,
    pub poc: Option<PriceLevel>, // Point of Control
    pub session_bars_count: usize,
    pub session_start_time: i64,
}

/// Конфигурация Volume Profile
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeProfileConfig {
    pub tick_size: f64,
    pub session_duration: i64,
    pub price_multiplier: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Bar;

    #[test]
    fn test_volume_profile_basic() {
        let mut profile = VolumeProfile::new(0.01, 0); // Без автосброса
        
        let bar = Bar {
            time: 1000,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 1000.0,
        };
        // Bar is already the correct type
        
        let updated = profile.update(&bar);
        
        assert!(updated);
        assert!(profile.is_ready());
        assert_eq!(profile.total_volume(), 1000.0);
        assert!(profile.levels_count() > 0);
    }
    
    #[test]
    fn test_poc() {
        let mut profile = VolumeProfile::new(0.01, 0);
        
        // Добавляем бар
        let bar = Bar {
            time: 1000,
            open: 100.0,
            high: 100.0,
            low: 100.0,
            close: 100.0,
            volume: 1000.0,
        };
        // Bar is already the correct type
        profile.update(&bar);
        
        let poc = profile.get_poc();
        assert!(poc.is_some());
        
        let poc_level = poc.unwrap();
        assert_eq!(poc_level.price, 100.0);
        assert!(poc_level.volume > 0.0);
    }
    
    #[test]
    fn test_session_reset() {
        let mut profile = VolumeProfile::new(0.01, 1000); // Сессия 1000 секунд
        
        // Первый бар
        let bar1 = Bar {
            time: 1000,
            open: 100.0,
            high: 101.0,
            low: 99.0,
            close: 100.5,
            volume: 1000.0,
        };
        // Bar is already the correct type
        profile.update(&bar1);
        
        let first_volume = profile.total_volume();
        
        // Бар через 2000 секунд - новая сессия
        let bar2 = Bar {
            time: 3000,
            open: 102.0,
            high: 103.0,
            low: 101.0,
            close: 102.5,
            volume: 500.0,
        };
        // Bar is already the correct type
        profile.update(&bar2);
        
        // Объем должен сброситься
        assert_eq!(profile.total_volume(), 500.0);
        assert_ne!(first_volume, profile.total_volume());
    }
} 






















