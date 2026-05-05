//! Hilbert Transform Dominant Cycle - индикатор доминирующего цикла по методу Джона Эхлерса
//! 
//! Использует преобразование Гильберта для определения доминирующего периода цикла в ценовых данных.
//! Основан на работах John Ehlers "Rocket Science for Traders" и "Cycle Analytics for Traders".
//! 
//! Алгоритм:
//! 1. Применяется Hilbert Transform для получения квадратурных компонент
//! 2. Рассчитывается мгновенная фаза и период
//! 3. Сглаживается для получения доминирующего цикла

use arrayvec::ArrayVec;
use std::f64::consts::PI;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Результат анализа доминирующего цикла
#[derive(Debug, Clone, Copy)]
pub struct DominantCycleResult {
    pub period: f64,              // Доминирующий период в барах
    pub phase: f64,               // Мгновенная фаза (-π до π)
    pub amplitude: f64,           // Амплитуда цикла
    pub cycle_position: f64,      // Позиция в цикле (0.0 до 1.0)
    pub trend_strength: f64,      // Сила тренда (0.0 до 1.0)
}

impl DominantCycleResult {
    pub fn empty() -> Self {
        Self {
            period: 20.0,
            phase: 0.0,
            amplitude: 0.0,
            cycle_position: 0.0,
            trend_strength: 0.5,
        }
    }
    
    /// Получить позицию в цикле как строку
    pub fn cycle_position_name(&self) -> &'static str {
        match self.cycle_position {
            x if x < 0.125 => "Дно цикла",
            x if x < 0.375 => "Восходящая фаза",
            x if x < 0.625 => "Пик цикла", 
            x if x < 0.875 => "Нисходящая фаза",
            _ => "Дно цикла",
        }
    }
    
    /// Определить силу тренда
    pub fn trend_strength_name(&self) -> &'static str {
        match self.trend_strength {
            x if x < 0.2 => "Очень слабый",
            x if x < 0.4 => "Слабый",
            x if x < 0.6 => "Умеренный",
            x if x < 0.8 => "Сильный",
            _ => "Очень сильный",
        }
    }
}

/// Hilbert Transform Dominant Cycle индикатор
#[derive(Clone)]
pub struct HilbertDominantCycle {
    // Буферы для Hilbert Transform (требуется минимум 7 значений)
    prices: ArrayVec<f64, 32>,
    
    // Компоненты Hilbert Transform
    in_phase: ArrayVec<f64, 32>,      // I компонента
    quadrature: ArrayVec<f64, 32>,    // Q компонента
    
    // Мгновенные значения
    inst_period: ArrayVec<f64, 32>,   // Мгновенный период
    inst_phase: ArrayVec<f64, 32>,    // Мгновенная фаза
    
    // Сглаженные значения
    smooth_period: f64,               // Сглаженный доминирующий период
    smooth_phase: f64,                // Сглаженная фаза
    
    // Дополнительные параметры
    min_period: f64,                  // Минимальный период для анализа
    max_period: f64,                  // Максимальный период для анализа
    
    // Результат
    current_result: DominantCycleResult,
    
    // Состояние
    is_ready: bool,
    update_count: usize,
}

impl HilbertDominantCycle {
    /// Создать новый индикатор с параметрами по умолчанию
    pub fn new() -> Self {
        Self::with_period_range(8.0, 50.0)
    }
    
    /// Создать новый индикатор с заданным диапазоном периодов
    pub fn with_period_range(min_period: f64, max_period: f64) -> Self {
        assert!(min_period > 0.0 && max_period > min_period, 
                "Invalid period range: min_period must be > 0 and max_period > min_period");
        
        Self {
            prices: ArrayVec::new(),
            in_phase: ArrayVec::new(),
            quadrature: ArrayVec::new(),
            inst_period: ArrayVec::new(),
            inst_phase: ArrayVec::new(),
            smooth_period: (min_period + max_period) / 2.0,
            smooth_phase: 0.0,
            min_period,
            max_period,
            current_result: DominantCycleResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }
    
    /// Обновить индикатор новым баром
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> DominantCycleResult {
        self.update_price(close)
    }
    
    /// Обновить индикатор новой ценой
    pub fn update_price(&mut self, price: f64) -> DominantCycleResult {
        // Добавляем цену в буфер
        if self.prices.len() >= 32 {
            self.prices.remove(0);
        }
        self.prices.push(price);
        
        // Нужно минимум 7 значений для Hilbert Transform
        if self.prices.len() >= 7 {
            self.calculate_hilbert_transform();
            self.calculate_dominant_cycle();
            self.is_ready = true;
        }
        
        self.update_count += 1;
        self.current_result
    }
    
    /// Рассчитать компоненты Hilbert Transform
    fn calculate_hilbert_transform(&mut self) {
        let len = self.prices.len();
        if len < 7 {
            return;
        }
        
        // Получаем последние 7 значений для расчета
        let idx = len - 1;
        
        // Hilbert Transform для I компоненты (In-Phase)
        // I = (Price[i-3] + Price[i-2] + Price[i-1] + Price[i]) / 4
        let i_component = if idx >= 3 {
            (self.prices[idx-3] + self.prices[idx-2] + self.prices[idx-1] + self.prices[idx]) / 4.0
        } else {
            self.prices[idx]
        };
        
        // Hilbert Transform для Q компоненты (Quadrature)  
        // Q = (Price[i-6] + 2*Price[i-4] + 3*Price[i-2] + 3*Price[i] + 2*Price[i+2] + Price[i+4]) / 12
        // Упрощенная версия для реального времени:
        let q_component = if idx >= 6 {
            (self.prices[idx-6] + 2.0*self.prices[idx-4] + 3.0*self.prices[idx-2] + 3.0*self.prices[idx]) / 9.0
        } else if idx >= 2 {
            (self.prices[idx-2] + self.prices[idx]) / 2.0
        } else {
            self.prices[idx]
        };
        
        // Сохраняем компоненты
        if self.in_phase.len() >= 32 {
            self.in_phase.remove(0);
        }
        self.in_phase.push(i_component);
        
        if self.quadrature.len() >= 32 {
            self.quadrature.remove(0);
        }
        self.quadrature.push(q_component);
    }
    
    /// Рассчитать доминирующий цикл
    fn calculate_dominant_cycle(&mut self) {
        if self.in_phase.len() < 2 || self.quadrature.len() < 2 {
            return;
        }
        
        let i_len = self.in_phase.len();
        let q_len = self.quadrature.len();
        
        // Получаем текущие и предыдущие значения
        let i_curr = self.in_phase[i_len - 1];
        let _i_prev = self.in_phase[i_len - 2];
        let q_curr = self.quadrature[q_len - 1];
        let _q_prev = self.quadrature[q_len - 2];
        
        // Рассчитываем мгновенную фазу
        let phase = if i_curr != 0.0 {
            (q_curr / i_curr).atan()
        } else {
            PI / 2.0 * q_curr.signum()
        };
        
        // Рассчитываем изменение фазы
        let mut delta_phase = phase - if self.inst_phase.is_empty() { 0.0 } else { self.inst_phase[self.inst_phase.len() - 1] };
        
        // Нормализуем изменение фазы
        if delta_phase < -PI {
            delta_phase += 2.0 * PI;
        } else if delta_phase > PI {
            delta_phase -= 2.0 * PI;
        }
        
        // Рассчитываем мгновенный период
        let inst_period = if delta_phase.abs() > 0.01 {
            let period = 2.0 * PI / delta_phase.abs();
            // Ограничиваем период заданными пределами
            period.max(self.min_period).min(self.max_period)
        } else {
            self.smooth_period // Используем предыдущее значение
        };
        
        // Сохраняем мгновенные значения
        if self.inst_period.len() >= 32 {
            self.inst_period.remove(0);
        }
        self.inst_period.push(inst_period);
        
        if self.inst_phase.len() >= 32 {
            self.inst_phase.remove(0);
        }
        self.inst_phase.push(phase);
        
        // Сглаживаем период (EMA с альфа = 0.2)
        let alpha = 0.2;
        self.smooth_period = alpha * inst_period + (1.0 - alpha) * self.smooth_period;
        self.smooth_phase = alpha * phase + (1.0 - alpha) * self.smooth_phase;
        
        // Рассчитываем амплитуду
        let amplitude = (i_curr * i_curr + q_curr * q_curr).sqrt();
        
        // Рассчитываем позицию в цикле (0.0 до 1.0)
        let normalized_phase = (phase + PI) / (2.0 * PI);
        let cycle_position = normalized_phase.fract();
        
        // Рассчитываем силу тренда на основе стабильности периода
        let trend_strength = self.calculate_trend_strength();
        
        // Обновляем результат
        self.current_result = DominantCycleResult {
            period: self.smooth_period,
            phase: self.smooth_phase,
            amplitude,
            cycle_position,
            trend_strength,
        };
    }
    
    /// Рассчитать силу тренда на основе стабильности периода
    fn calculate_trend_strength(&self) -> f64 {
        if self.inst_period.len() < 5 {
            return 0.5;
        }
        
        // Берем последние 5 значений периода
        let start_idx = self.inst_period.len() - 5;
        let periods = &self.inst_period[start_idx..];
        
        // Рассчитываем стандартное отклонение
        let mean: f64 = periods.iter().sum::<f64>() / periods.len() as f64;
        let variance: f64 = periods.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / periods.len() as f64;
        let std_dev = variance.sqrt();
        
        // Чем меньше отклонение, тем сильнее тренд
        
        1.0 - (std_dev / mean).min(1.0)
    }
    
    /// Получить текущий результат
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_result.period)
    }
    
    /// Получить полный результат анализа
    pub fn result(&self) -> DominantCycleResult {
        self.current_result
    }
    
    /// Проверить, готов ли индикатор
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Сбросить состояние индикатора
    pub fn reset(&mut self) {
        self.prices.clear();
        self.in_phase.clear();
        self.quadrature.clear();
        self.inst_period.clear();
        self.inst_phase.clear();
        self.smooth_period = (self.min_period + self.max_period) / 2.0;
        self.smooth_phase = 0.0;
        self.current_result = DominantCycleResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }
    
    /// Получить период
    pub fn period(&self) -> usize {
        self.smooth_period as usize
    }
    
    /// Генерировать торговый сигнал на основе позиции в цикле
    /// Возвращает: -1 (продажа), 0 (нет сигнала), 1 (покупка)
    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        
        let pos = self.current_result.cycle_position;
        let strength = self.current_result.trend_strength;
        
        // Сигналы только при достаточной силе тренда
        if strength < 0.3 {
            return 0;
        }
        
        // Покупка в районе дна цикла (0.0-0.25)
        if pos < 0.25 && strength > 0.5 {
            return 1;
        }
        
        // Продажа в районе пика цикла (0.5-0.75)
        if pos > 0.5 && pos < 0.75 && strength > 0.5 {
            return -1;
        }
        
        0
    }
    
    /// Предсказать следующие значения цикла
    pub fn forecast(&self, bars_ahead: usize) -> Vec<f64> {
        if !self.is_ready {
            return vec![];
        }
        
        let mut forecast = Vec::with_capacity(bars_ahead);
        let period = self.current_result.period;
        let amplitude = self.current_result.amplitude;
        let current_phase = self.current_result.phase;
        
        for i in 1..=bars_ahead {
            let future_phase = current_phase + (2.0 * PI * i as f64 / period);
            let forecast_value = amplitude * future_phase.sin();
            forecast.push(forecast_value);
        }
        
        forecast
    }
    
    /// Получить информацию о текущем состоянии
    pub fn info(&self) -> String {
        let result = self.current_result;
        format!(
            "Hilbert Dominant Cycle: Период={:.1}, Фаза={:.3}, Амплитуда={:.4}, Позиция={} ({:.1}%), Сила тренда={}",
            result.period,
            result.phase,
            result.amplitude,
            result.cycle_position_name(),
            result.cycle_position * 100.0,
            result.trend_strength_name()
        )
    }
    
    /// Получить дополнительные значения
    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        values.insert("period".to_string(), self.current_result.period);
        values.insert("phase".to_string(), self.current_result.phase);
        values.insert("amplitude".to_string(), self.current_result.amplitude);
        values.insert("cycle_position".to_string(), self.current_result.cycle_position);
        values.insert("trend_strength".to_string(), self.current_result.trend_strength);
        values.insert("smooth_period".to_string(), self.smooth_period);
        values.insert("smooth_phase".to_string(), self.smooth_phase);
        values
    }
    
    /// Получить количество обновлений
    pub fn update_count(&self) -> usize {
        self.update_count
    }
    
    /// Получить диапазон периодов
    pub fn period_range(&self) -> (f64, f64) {
        (self.min_period, self.max_period)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hilbert_dominant_cycle_creation() {
        let hdc = HilbertDominantCycle::new();
        assert!(!hdc.is_ready());
        assert_eq!(hdc.value().main(), 20.0); // Среднее между 8 и 50
    }
    
    #[test]
    fn test_hilbert_dominant_cycle_with_range() {
        let hdc = HilbertDominantCycle::with_period_range(10.0, 30.0);
        assert_eq!(hdc.period_range(), (10.0, 30.0));
        assert_eq!(hdc.value().main(), 20.0); // Среднее между 10 и 30
    }
    
    #[test]
    fn test_hilbert_update() {
        let mut hdc = HilbertDominantCycle::new();
        
        // Добавляем синусоидальные данные с известным периодом
        let period = 20.0;
        let amplitude = 10.0;
        let base_price = 100.0;
        
        for i in 0..50 {
            let phase = 2.0 * PI * i as f64 / period;
            let price = base_price + amplitude * phase.sin();
            let result = hdc.update_price(price);
            
            if i > 10 { // После нескольких обновлений
                assert!(hdc.is_ready());
                assert!(result.period > 0.0);
                assert!(result.amplitude >= 0.0);
                assert!(result.cycle_position >= 0.0 && result.cycle_position <= 1.0);
            }
        }
        
        // Период должен быть близок к 20
        let detected_period = hdc.value().main();
        assert!(detected_period > 15.0 && detected_period < 25.0);
    }
    
    #[test]
    fn test_trading_signals() {
        let mut hdc = HilbertDominantCycle::new();
        
        // Добавляем данные для получения сигналов
        for i in 0..30 {
            let price = 100.0 + 10.0 * (i as f64 * 0.3).sin();
            let _result = hdc.update_price(price);
        }
        
        if hdc.is_ready() {
            let signal = hdc.trading_signal();
            assert!(signal >= -1 && signal <= 1);
        }
    }
    
    #[test]
    fn test_forecast() {
        let mut hdc = HilbertDominantCycle::new();
        
        // Добавляем синусоидальные данные
        for i in 0..30 {
            let price = 100.0 + 10.0 * (i as f64 * 0.3).sin();
            let _result = hdc.update_price(price);
        }
        
        if hdc.is_ready() {
            let forecast = hdc.forecast(5);
            assert_eq!(forecast.len(), 5);
            
            // Все прогнозы должны быть конечными числами
            for value in forecast {
                assert!(value.is_finite());
            }
        }
    }
    
    #[test]
    fn test_cycle_position() {
        let result = DominantCycleResult {
            period: 20.0,
            phase: 0.0,
            amplitude: 10.0,
            cycle_position: 0.1,
            trend_strength: 0.7,
        };
        
        assert_eq!(result.cycle_position_name(), "Дно цикла");
        assert_eq!(result.trend_strength_name(), "Сильный");
    }
} 






















