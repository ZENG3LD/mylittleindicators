//! Hilbert Transform
//! Преобразование Гильберта для получения аналитического сигнала
//! Позволяет вычислять мгновенную амплитуду, фазу и частоту

use arrayvec::ArrayVec;
use std::f64::consts::PI;

/// Аналитический сигнал (результат преобразования Гильберта)
#[derive(Debug, Clone)]
pub struct AnalyticSignal {
    pub real_part: f64,              // Реальная часть (исходный сигнал)
    pub imaginary_part: f64,         // Мнимая часть (преобразование Гильберта)
    pub instantaneous_amplitude: f64, // Мгновенная амплитуда (огибающая)
    pub instantaneous_phase: f64,     // Мгновенная фаза
    pub instantaneous_frequency: f64, // Мгновенная частота
}

impl Default for AnalyticSignal {
    fn default() -> Self {
        Self::new()
    }
}

impl AnalyticSignal {
    pub fn new() -> Self {
        Self {
            real_part: 0.0,
            imaginary_part: 0.0,
            instantaneous_amplitude: 0.0,
            instantaneous_phase: 0.0,
            instantaneous_frequency: 0.0,
        }
    }
}

/// Hilbert Transform
#[derive(Clone)]
pub struct HilbertTransform {
    // Временные данные
    time_series: ArrayVec<f64, 512>,
    
    // Результаты преобразования
    hilbert_transform: ArrayVec<f64, 512>,      // Преобразование Гильберта
    analytic_signal: AnalyticSignal,            // Текущий аналитический сигнал
    
    // История мгновенных характеристик
    amplitude_history: ArrayVec<f64, 512>,      // История амплитуд
    phase_history: ArrayVec<f64, 512>,          // История фаз
    frequency_history: ArrayVec<f64, 512>,      // История частот
    
    // Фильтр для сглаживания мгновенной частоты
    frequency_filter_length: usize,
    
    // Параметры
    window_size: usize,                         // Размер окна для вычисления
    sampling_rate: f64,                         // Частота дискретизации
    
    // Статистики
    avg_amplitude: f64,                         // Средняя амплитуда
    amplitude_variance: f64,                    // Дисперсия амплитуды
    phase_coherence: f64,                       // Когерентность фазы
    frequency_stability: f64,                   // Стабильность частоты
    
    // Состояние
    is_ready: bool,
    min_samples: usize,
}

impl HilbertTransform {
    pub fn new(window_size: usize, sampling_rate: f64) -> Self {
        let window_size = window_size.clamp(16, 256);
        
        Self {
            time_series: ArrayVec::new(),
            hilbert_transform: ArrayVec::new(),
            analytic_signal: AnalyticSignal::new(),
            amplitude_history: ArrayVec::new(),
            phase_history: ArrayVec::new(),
            frequency_history: ArrayVec::new(),
            frequency_filter_length: 5,
            window_size,
            sampling_rate,
            avg_amplitude: 0.0,
            amplitude_variance: 0.0,
            phase_coherence: 0.0,
            frequency_stability: 0.0,
            is_ready: false,
            min_samples: window_size * 2,
        }
    }
    
    /// Обновить преобразование Гильберта новым значением
    pub fn update(&mut self, value: f64) -> &AnalyticSignal {
        // Добавляем новое значение
        if self.time_series.len() >= 512 {
            self.time_series.remove(0);
        }
        if !self.time_series.is_full() {
            self.time_series.push(value);
        }
        
        // Если достаточно данных, вычисляем преобразование Гильберта
        if self.time_series.len() >= self.min_samples {
            self.compute_hilbert_transform();
            self.compute_analytic_signal();
            self.update_statistics();
            self.is_ready = true;
        }
        
        &self.analytic_signal
    }
    
    /// Вычисление преобразования Гильберта
    fn compute_hilbert_transform(&mut self) {
        self.hilbert_transform.clear();
        
        let n = self.time_series.len();
        
        // Приближенное вычисление преобразования Гильберта через конечные разности
        // H[x(t)] ≈ (1/π) * ∫ x(τ)/(t-τ) dτ
        
        for i in 0..n {
            let mut hilbert_value = 0.0;
            let mut weight_sum = 0.0;
            
            // Вычисляем интеграл численно в окрестности точки i
            let window_start = i.saturating_sub(self.window_size / 2);
            let window_end = (i + self.window_size / 2).min(n);
            
            for j in window_start..window_end {
                if i != j {
                    let tau_diff = i as f64 - j as f64;
                    let weight = 1.0 / (PI * tau_diff);
                    
                    // Применяем окно для уменьшения граничных эффектов
                    let window_coeff = self.hamming_window(j, window_start, window_end);
                    
                    hilbert_value += weight * self.time_series[j] * window_coeff;
                    weight_sum += weight.abs();
                }
            }
            
            // Нормализуем результат
            if weight_sum > 0.0 {
                hilbert_value /= weight_sum;
            }
            
            if !self.hilbert_transform.is_full() {
                self.hilbert_transform.push(hilbert_value);
            }
        }
    }
    
    /// Окно Хэмминга для сглаживания
    fn hamming_window(&self, index: usize, start: usize, end: usize) -> f64 {
        let n = end - start;
        if n <= 1 {
            return 1.0;
        }
        
        let i = index - start;
        0.54 - 0.46 * (2.0 * PI * i as f64 / (n - 1) as f64).cos()
    }
    
    /// Вычисление аналитического сигнала
    fn compute_analytic_signal(&mut self) {
        if self.time_series.is_empty() || self.hilbert_transform.is_empty() {
            return;
        }
        
        let last_idx = self.time_series.len() - 1;
        
        // Реальная и мнимая части
        self.analytic_signal.real_part = self.time_series[last_idx];
        self.analytic_signal.imaginary_part = if last_idx < self.hilbert_transform.len() {
            self.hilbert_transform[last_idx]
        } else {
            0.0
        };
        
        // Мгновенная амплитуда (модуль комплексного числа)
        self.analytic_signal.instantaneous_amplitude = (
            self.analytic_signal.real_part.powi(2) + 
            self.analytic_signal.imaginary_part.powi(2)
        ).sqrt();
        
        // Мгновенная фаза
        self.analytic_signal.instantaneous_phase = self.analytic_signal.imaginary_part
            .atan2(self.analytic_signal.real_part);
        
        // Мгновенная частота (производная фазы)
        self.compute_instantaneous_frequency();
        
        // Сохраняем в историю
        self.save_to_history();
    }
    
    /// Вычисление мгновенной частоты
    fn compute_instantaneous_frequency(&mut self) {
        if self.phase_history.len() < 2 {
            self.analytic_signal.instantaneous_frequency = 0.0;
            return;
        }
        
        // Производная фазы = мгновенная частота
        let current_phase = self.analytic_signal.instantaneous_phase;
        let prev_phase = self.phase_history[self.phase_history.len() - 1];
        
        // Учитываем разрыв фазы (unwrapping)
        let mut phase_diff = current_phase - prev_phase;
        
        // Приводим разность к интервалу [-π, π]
        while phase_diff > PI {
            phase_diff -= 2.0 * PI;
        }
        while phase_diff < -PI {
            phase_diff += 2.0 * PI;
        }
        
        // Мгновенная частота в Гц
        let raw_frequency = phase_diff * self.sampling_rate / (2.0 * PI);
        
        // Сглаживаем частоту для уменьшения шума
        self.analytic_signal.instantaneous_frequency = self.smooth_frequency(raw_frequency);
    }
    
    /// Сглаживание мгновенной частоты
    fn smooth_frequency(&self, new_frequency: f64) -> f64 {
        if self.frequency_history.len() < self.frequency_filter_length {
            return new_frequency;
        }
        
        // Простое скользящее среднее для сглаживания
        let start_idx = self.frequency_history.len().saturating_sub(self.frequency_filter_length);
        let sum: f64 = self.frequency_history[start_idx..].iter().sum();
        let avg = sum / (self.frequency_history.len() - start_idx) as f64;
        
        // Взвешенное среднее: 70% история + 30% новое значение
        0.7 * avg + 0.3 * new_frequency
    }
    
    /// Сохранение в историю
    fn save_to_history(&mut self) {
        // Амплитуда
        if self.amplitude_history.len() >= 512 {
            self.amplitude_history.remove(0);
        }
        if !self.amplitude_history.is_full() {
            self.amplitude_history.push(self.analytic_signal.instantaneous_amplitude);
        }
        
        // Фаза
        if self.phase_history.len() >= 512 {
            self.phase_history.remove(0);
        }
        if !self.phase_history.is_full() {
            self.phase_history.push(self.analytic_signal.instantaneous_phase);
        }
        
        // Частота
        if self.frequency_history.len() >= 512 {
            self.frequency_history.remove(0);
        }
        if !self.frequency_history.is_full() {
            self.frequency_history.push(self.analytic_signal.instantaneous_frequency);
        }
    }
    
    /// Обновление статистик
    fn update_statistics(&mut self) {
        // Средняя амплитуда
        if !self.amplitude_history.is_empty() {
            self.avg_amplitude = self.amplitude_history.iter().sum::<f64>() / self.amplitude_history.len() as f64;
            
            // Дисперсия амплитуды
            let variance_sum: f64 = self.amplitude_history.iter()
                .map(|&a| (a - self.avg_amplitude).powi(2))
                .sum();
            self.amplitude_variance = variance_sum / self.amplitude_history.len() as f64;
        }
        
        // Когерентность фазы (стабильность фазовых переходов)
        self.phase_coherence = self.calculate_phase_coherence();
        
        // Стабильность частоты (обратная дисперсия частоты)
        self.frequency_stability = self.calculate_frequency_stability();
    }
    
    /// Вычисление когерентности фазы
    fn calculate_phase_coherence(&self) -> f64 {
        if self.phase_history.len() < 3 {
            return 0.0;
        }
        
        // Измеряем стабильность фазовых переходов
        let mut phase_diff_variance = 0.0;
        let mut valid_diffs = 0;
        
        for i in 1..self.phase_history.len() {
            let phase_diff = self.phase_history[i] - self.phase_history[i - 1];
            
            // Нормализуем разность фазы
            let normalized_diff = ((phase_diff + PI) % (2.0 * PI)) - PI;
            phase_diff_variance += normalized_diff.powi(2);
            valid_diffs += 1;
        }
        
        if valid_diffs > 0 {
            phase_diff_variance /= valid_diffs as f64;
            // Когерентность = 1 / (1 + variance), чтобы получить значение от 0 до 1
            1.0 / (1.0 + phase_diff_variance)
        } else {
            0.0
        }
    }
    
    /// Вычисление стабильности частоты
    fn calculate_frequency_stability(&self) -> f64 {
        if self.frequency_history.len() < 2 {
            return 0.0;
        }
        
        // Дисперсия частоты
        let mean_freq: f64 = self.frequency_history.iter().sum::<f64>() / self.frequency_history.len() as f64;
        let freq_variance: f64 = self.frequency_history.iter()
            .map(|&f| (f - mean_freq).powi(2))
            .sum::<f64>() / self.frequency_history.len() as f64;
        
        // Стабильность = 1 / (1 + коэффициент вариации)
        if mean_freq.abs() > 1e-10 {
            let cv = freq_variance.sqrt() / mean_freq.abs();
            1.0 / (1.0 + cv)
        } else {
            0.0
        }
    }
    
    /// Получить аналитический сигнал
    pub fn analytic_signal(&self) -> &AnalyticSignal {
        &self.analytic_signal
    }
    
    /// Получить мгновенную амплитуду
    pub fn instantaneous_amplitude(&self) -> f64 {
        self.analytic_signal.instantaneous_amplitude
    }
    
    /// Получить мгновенную фазу
    pub fn instantaneous_phase(&self) -> f64 {
        self.analytic_signal.instantaneous_phase
    }
    
    /// Получить мгновенную частоту
    pub fn instantaneous_frequency(&self) -> f64 {
        self.analytic_signal.instantaneous_frequency
    }
    
    /// Получить среднюю амплитуду
    pub fn average_amplitude(&self) -> f64 {
        self.avg_amplitude
    }
    
    /// Получить дисперсию амплитуды
    pub fn amplitude_variance(&self) -> f64 {
        self.amplitude_variance
    }
    
    /// Получить когерентность фазы
    pub fn phase_coherence(&self) -> f64 {
        self.phase_coherence
    }
    
    /// Получить стабильность частоты
    pub fn frequency_stability(&self) -> f64 {
        self.frequency_stability
    }
    
    /// Получить историю амплитуд
    pub fn amplitude_history(&self) -> &[f64] {
        &self.amplitude_history
    }
    
    /// Получить историю фаз
    pub fn phase_history(&self) -> &[f64] {
        &self.phase_history
    }
    
    /// Получить историю частот
    pub fn frequency_history(&self) -> &[f64] {
        &self.frequency_history
    }
    
    /// Установить длину фильтра частоты
    pub fn set_frequency_filter_length(&mut self, length: usize) {
        self.frequency_filter_length = length.clamp(1, 20);
    }
    
    /// Проверить готовность
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    /// Получить размер окна
    pub fn window_size(&self) -> usize {
        self.window_size
    }
    
    /// Получить частоту дискретизации
    pub fn sampling_rate(&self) -> f64 {
        self.sampling_rate
    }
    
    /// Update with bar data (uses close price)
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) {
        self.update(close);
    }

    /// Get current value as IndicatorValue::Hilbert
    pub fn value(&self) -> crate::bar_indicators::indicator_value::IndicatorValue {
        crate::bar_indicators::indicator_value::IndicatorValue::Hilbert {
            amplitude: self.analytic_signal.instantaneous_amplitude,
            phase: self.analytic_signal.instantaneous_phase,
            frequency: self.analytic_signal.instantaneous_frequency,
        }
    }

    /// Сбросить преобразование
    pub fn reset(&mut self) {
        self.time_series.clear();
        self.hilbert_transform.clear();
        self.analytic_signal = AnalyticSignal::new();
        self.amplitude_history.clear();
        self.phase_history.clear();
        self.frequency_history.clear();
        self.avg_amplitude = 0.0;
        self.amplitude_variance = 0.0;
        self.phase_coherence = 0.0;
        self.frequency_stability = 0.0;
        self.is_ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hilbert_creation() {
        let ht = HilbertTransform::new(32, 100.0);
        assert!(!ht.is_ready());
        assert_eq!(ht.window_size(), 32);
        assert_eq!(ht.sampling_rate(), 100.0);
    }

    #[test]
    fn test_hilbert_update() {
        let mut ht = HilbertTransform::new(32, 100.0);
        for i in 0..150 {
            let value = (i as f64 * 0.1).sin() * 10.0 + 100.0;
            ht.update(value);
        }
        assert!(ht.is_ready());
        assert!(ht.instantaneous_amplitude().is_finite());
        assert!(ht.instantaneous_phase().is_finite());
    }

    #[test]
    fn test_hilbert_analytic_signal() {
        let mut ht = HilbertTransform::new(32, 100.0);
        for i in 0..150 {
            ht.update(100.0 + (i as f64 * 0.2).sin() * 5.0);
        }
        let sig = ht.analytic_signal();
        assert!(sig.real_part.is_finite());
        assert!(sig.imaginary_part.is_finite());
    }

    #[test]
    fn test_hilbert_reset() {
        let mut ht = HilbertTransform::new(32, 100.0);
        for i in 0..150 {
            ht.update(100.0 + i as f64);
        }
        assert!(ht.is_ready());
        ht.reset();
        assert!(!ht.is_ready());
    }
}






















