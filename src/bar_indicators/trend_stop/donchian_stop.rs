//! Donchian Stop - индикатор динамических уровней на основе каналов Дончиана
//! 
//! Вычисляет уровни на основе каналов Дончиана:
//! - Верхняя линия: максимальный High за N периодов
//! - Нижняя линия: минимальный Low за N периодов  
//! - Средняя линия: (верхняя + нижняя) / 2
//! 
//! Может использовать разные периоды для верхней и нижней полос,
//! а также добавлять отступы для более консервативного подхода.
//! 
//! Индикатор НЕ содержит логику стопов - только возвращает граници каналов.
//! Логика остановки позиций реализуется в стратегиях на основе этих уровней.

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Donchian Stop индикатор - уровни на основе каналов Дончиана
#[derive(Debug, Clone)]
pub struct DonchianStop {
    upper_period: usize,    // Период для верхней полосы (highs)
    lower_period: usize,    // Период для нижней полосы (lows)
    offset: f64,            // Отступ от границ канала
    use_percentage: bool,   // Использовать процентный отступ
    
    // Буферы данных
    highs: ArrayVec<f64, 512>,
    lows: ArrayVec<f64, 512>,
    
    // Текущие уровни
    upper_line: f64,        // Максимальный high
    lower_line: f64,        // Минимальный low  
    middle_line: f64,       // Средняя линия
    
    // Уровни с отступами
    upper_stop: f64,        // Верхняя граница с отступом
    lower_stop: f64,        // Нижняя граница с отступом
    
    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl DonchianStop {
    /// Создать Donchian Stop с параметрами по умолчанию 
    /// (period=20, offset=0.0, absolute)
    pub fn new() -> Self {
        Self::with_params(20, 0.0, false)
    }
    
    /// Создать Donchian Stop с одинаковым периодом для обеих полос
    /// * `period` - период для поиска экстремумов
    /// * `offset` - отступ от границ (пункты или %)
    /// * `use_percentage` - использовать процентный отступ
    pub fn with_params(period: usize, offset: f64, use_percentage: bool) -> Self {
        Self::with_different_periods(period, period, offset, use_percentage)
    }
    
    /// Создать Donchian Stop с разными периодами для верхней и нижней полос
    /// * `upper_period` - период для поиска максимумов
    /// * `lower_period` - период для поиска минимумов  
    /// * `offset` - отступ от границ (пункты или %)
    /// * `use_percentage` - использовать процентный отступ
    pub fn with_different_periods(
        upper_period: usize,
        lower_period: usize,
        offset: f64,
        use_percentage: bool,
    ) -> Self {
        assert!(upper_period > 0, "Upper period must be greater than 0");
        assert!(lower_period > 0, "Lower period must be greater than 0");
        
        let _max_period = upper_period.max(lower_period);
        
        Self {
            upper_period,
            lower_period,
            offset,
            use_percentage,
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            upper_line: 0.0,
            lower_line: 0.0,
            middle_line: 0.0,
            upper_stop: 0.0,
            lower_stop: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Создать для оптимизации с широким диапазоном параметров
    pub fn for_optimization(
        upper_period: usize,
        lower_period: usize,
        offset: f64,
        use_percentage: bool,
    ) -> Self {
        Self::with_different_periods(upper_period, lower_period, offset, use_percentage)
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, _close: f64, _volume: f64) -> (f64, f64, f64) {
        self.bars_count += 1;
        
        // Определяем максимальный размер буфера
        let max_period = self.upper_period.max(self.lower_period);
        
        // Добавляем в буферы
        if self.highs.len() >= max_period {
            self.highs.remove(0);
            self.lows.remove(0);
        }
        self.highs.push(high);
        self.lows.push(low);
        
        // Вычисляем верхнюю линию (максимальный high за upper_period)
        if self.highs.len() >= self.upper_period {
            let start_idx = self.highs.len().saturating_sub(self.upper_period);
            self.upper_line = self.highs[start_idx..]
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
        }
        
        // Вычисляем нижнюю линию (минимальный low за lower_period)
        if self.lows.len() >= self.lower_period {
            let start_idx = self.lows.len().saturating_sub(self.lower_period);
            self.lower_line = self.lows[start_idx..]
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min);
        }
        
        // Вычисляем среднюю линию
        if self.upper_line > 0.0 && self.lower_line < f64::INFINITY {
            self.middle_line = (self.upper_line + self.lower_line) / 2.0;
        }
        
        // Вычисляем уровни с отступами
        self.calculate_stop_levels();
        
        // Готов когда есть достаточно данных для обеих полос
        let min_period = self.upper_period.min(self.lower_period);
        self.is_ready = self.bars_count >= min_period && 
                       self.upper_line > 0.0 && 
                       self.lower_line < f64::INFINITY;
        
        (self.lower_stop, self.upper_stop, self.middle_line)
    }
    
    /// Вычислить уровни стопов с отступами
    fn calculate_stop_levels(&mut self) {
        if self.upper_line > 0.0 {
            if self.use_percentage {
                self.upper_stop = self.upper_line * (1.0 + self.offset / 100.0);
            } else {
                self.upper_stop = self.upper_line + self.offset;
            }
        }
        
        if self.lower_line < f64::INFINITY {
            if self.use_percentage {
                self.lower_stop = self.lower_line * (1.0 - self.offset / 100.0);
            } else {
                self.lower_stop = self.lower_line - self.offset;
            }
        }
    }
    
    /// Получить верхнюю линию (максимальный high)
    pub fn upper_line(&self) -> f64 {
        self.upper_line
    }
    
    /// Получить нижнюю линию (минимальный low)
    pub fn lower_line(&self) -> f64 {
        if self.lower_line == f64::INFINITY { 0.0 } else { self.lower_line }
    }
    
    /// Получить среднюю линию
    pub fn middle_line(&self) -> f64 {
        self.middle_line
    }
    
    /// Получить верхний стоп с отступом
    pub fn upper_stop(&self) -> f64 {
        self.upper_stop
    }
    
    /// Получить нижний стоп с отступом  
    pub fn lower_stop(&self) -> f64 {
        self.lower_stop
    }
    
    /// Получить основной уровень (нижний стоп по умолчанию)
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.lower_stop)
    }
    
    /// Получить все уровни как кортеж (lower_stop, upper_stop, middle)
    pub fn levels(&self) -> (f64, f64, f64) {
        (self.lower_stop, self.upper_stop, self.middle_line)
    }
    
    /// Получить ширину канала
    pub fn channel_width(&self) -> f64 {
        if self.upper_line > 0.0 && self.lower_line < f64::INFINITY {
            self.upper_line - self.lower_line
        } else {
            0.0
        }
    }
    
    /// Получить позицию цены в канале (0.0 = нижняя граница, 1.0 = верхняя)
    pub fn price_position(&self, price: f64) -> f64 {
        let width = self.channel_width();
        if width == 0.0 {
            return 0.5;
        }
        (price - self.lower_line) / width
    }
    
    /// Проверить пробой верхней границы
    pub fn is_above_upper(&self, price: f64) -> bool {
        price > self.upper_line
    }
    
    /// Проверить пробой нижней границы
    pub fn is_below_lower(&self, price: f64) -> bool {
        price < self.lower_line
    }
    
    /// Проверить нахождение внутри канала
    pub fn is_inside_channel(&self, price: f64) -> bool {
        price >= self.lower_line && price <= self.upper_line
    }
    
    /// Проверить готовность индикатора
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить индикатор
    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.upper_line = 0.0;
        self.lower_line = 0.0;
        self.middle_line = 0.0;
        self.upper_stop = 0.0;
        self.lower_stop = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
    
    /// Получить параметры индикатора
    pub fn params(&self) -> (usize, usize, f64, bool) {
        (self.upper_period, self.lower_period, self.offset, self.use_percentage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_donchian_stop_creation() {
        let ind = DonchianStop::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_donchian_stop_with_params() {
        let ind = DonchianStop::with_params(20, 0.5, false);
        assert!(!ind.is_ready());
        assert_eq!(ind.params(), (20, 20, 0.5, false));
    }

    #[test]
    fn test_donchian_stop_warmup() {
        let mut ind = DonchianStop::with_params(10, 0.0, false);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_donchian_stop_values_finite() {
        let mut ind = DonchianStop::with_params(10, 0.0, false);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (lower, upper, mid) = ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(lower.is_finite());
            assert!(upper.is_finite());
            assert!(mid.is_finite());
        }
    }

    #[test]
    fn test_donchian_stop_reset() {
        let mut ind = DonchianStop::with_params(10, 0.0, false);
        for i in 0..20 {
            ind.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
} 