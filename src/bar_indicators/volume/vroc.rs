//! Volume Rate of Change (VROC) - скорость изменения объема
//! VROC = (Current Volume - Volume N periods ago) / Volume N periods ago * 100
//! Показывает процентное изменение объема по сравнению с предыдущими периодами
//! Используется для анализа активности торговли и подтверждения ценовых движений

use arrayvec::ArrayVec;
use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Volume Rate of Change индикатор
#[derive(Clone)]
pub struct VolumeRateOfChange {
    period: usize,
    signal_period: usize,
    
    // Буферы для объемов и значений
    volume_buffer: ArrayVec<f64, 512>,
    vroc_values: ArrayVec<f64, 512>,
    
    // Сигнальная линия (MA от VROC)
    signal_ma: MovingAverageProvider,
    
    // Текущие значения
    vroc_value: f64,
    signal_value: f64,
    
    // Состояние
    bars_count: usize,
    is_ready: bool,
}

impl VolumeRateOfChange {
    /// Создать новый VROC с параметрами по умолчанию (14, 9)
    pub fn new() -> Self {
        Self::with_params(14, 9)
    }
    
    /// Создать новый VROC с настраиваемыми параметрами
    pub fn with_params(period: usize, signal_period: usize) -> Self {
        assert!(period > 0, "Period must be greater than 0");
        assert!(signal_period > 0, "Signal period must be greater than 0");
        
        Self {
            period,
            signal_period,
            volume_buffer: ArrayVec::new(),
            vroc_values: ArrayVec::new(),
            signal_ma: MovingAverageProvider::new(MovingAverageType::SMA, signal_period),
            vroc_value: 0.0,
            signal_value: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, _close: f64, volume: f64) -> f64 {
        self.bars_count += 1;
        
        // Добавляем объем в буфер
        if self.volume_buffer.len() >= 512 {
            self.volume_buffer.remove(0);
        }
        self.volume_buffer.push(volume);
        
        // Проверяем, можем ли рассчитать VROC
        if self.volume_buffer.len() > self.period {
            let current_volume = volume;
            let past_volume = self.volume_buffer[self.volume_buffer.len() - self.period - 1];
            
            // Рассчитываем VROC
            self.vroc_value = if past_volume.abs() > 1e-12 {
                (current_volume - past_volume) / past_volume * 100.0
            } else {
                0.0
            };
        }
        
        // Добавляем в буфер значений
        if self.vroc_values.len() >= 512 {
            self.vroc_values.remove(0);
        }
        self.vroc_values.push(self.vroc_value);
        
        // Рассчитываем сигнальную линию
        self.signal_value = self.signal_ma.update_bar(self.vroc_value, self.vroc_value, self.vroc_value, self.vroc_value, 1.0);
        
        // Проверяем готовность
        if self.bars_count >= self.period + self.signal_period {
            self.is_ready = true;
        }
        
        self.vroc_value
    }
    
    /// Получить значение VROC
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.vroc_value)
    }
    
    /// Получить значение сигнальной линии
    pub fn signal_value(&self) -> f64 {
        self.signal_value
    }
    
    /// Получить разность между VROC и сигнальной линией
    pub fn histogram(&self) -> f64 {
        self.vroc_value - self.signal_value
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить периоды индикатора
    pub fn periods(&self) -> (usize, usize) {
        (self.period, self.signal_period)
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.volume_buffer.clear();
        self.vroc_values.clear();
        self.signal_ma.reset();
        self.vroc_value = 0.0;
        self.signal_value = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
    
    /// Определить состояние объемной активности
    pub fn volume_activity(&self) -> &'static str {
        match self.vroc_value {
            v if v > 50.0 => "Very High Volume",
            v if v > 20.0 => "High Volume",
            v if v > 0.0 => "Above Average Volume",
            v if v > -20.0 => "Below Average Volume",
            v if v > -50.0 => "Low Volume",
            _ => "Very Low Volume"
        }
    }
    
    /// Получить торговый сигнал
    /// 1 = покупка, -1 = продажа, 0 = нейтрально
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready() {
            return 0;
        }
        
        // Сигнал на основе пересечения сигнальной линии
        if self.vroc_value > self.signal_value && self.vroc_value > 0.0 {
            1  // Покупка - растущий объем выше средней
        } else if self.vroc_value < self.signal_value && self.vroc_value < 0.0 {
            -1 // Продажа - падающий объем ниже средней
        } else {
            0  // Нейтрально
        }
    }
    
    /// Получить информацию о состоянии индикатора
    pub fn info(&self) -> String {
        format!(
            "VROC: {:.2}%, Signal: {:.2}%, Activity: {}",
            self.vroc_value,
            self.signal_value,
            self.volume_activity()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vroc_creation() {
        let vroc = VolumeRateOfChange::new();
        assert!(!vroc.is_ready());
        assert_eq!(vroc.value().main(), 0.0);
    }

    #[test]
    fn test_vroc_with_params() {
        let vroc = VolumeRateOfChange::with_params(10, 5);
        assert!(!vroc.is_ready());
        assert_eq!(vroc.periods(), (10, 5));
    }

    #[test]
    fn test_vroc_warmup() {
        let mut vroc = VolumeRateOfChange::with_params(10, 5);
        for i in 0..20 {
            let volume = 1000.0 + (i as f64 * 0.1).sin() * 100.0;
            vroc.update_bar(100.0, 101.0, 99.0, 100.0, volume);
        }
        assert!(vroc.is_ready());
    }

    #[test]
    fn test_vroc_values_finite() {
        let mut vroc = VolumeRateOfChange::new();
        for i in 0..30 {
            let volume = 1000.0 + i as f64 * 50.0;
            let value = vroc.update_bar(100.0, 101.0, 99.0, 100.0, volume);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_vroc_reset() {
        let mut vroc = VolumeRateOfChange::new();
        for i in 0..30 {
            vroc.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0 + i as f64 * 10.0);
        }
        vroc.reset();
        assert!(!vroc.is_ready());
        assert_eq!(vroc.value().main(), 0.0);
    }
} 






















