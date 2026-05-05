//! Wavelet Transform
//! Вейвлет-преобразование для многомасштабного анализа временных рядов
//! Позволяет анализировать сигнал одновременно во времени и частоте

use arrayvec::ArrayVec;

/// Типы вейвлетов
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WaveletType {
    Haar,           // Вейвлет Хаара (простейший)
    Daubechies4,    // Добеши-4
    Daubechies6,    // Добеши-6
    Morlet,         // Вейвлет Морле (комплексный)
    Mexican,        // Мексиканская шляпа
    Biorthogonal,   // Биортогональный
}

/// Результат вейвлет-преобразования
#[derive(Debug, Clone)]
pub struct WaveletCoefficients {
    pub scales: ArrayVec<f64, 32>,                      // Масштабы
    pub coefficients: ArrayVec<ArrayVec<f64, 256>, 32>, // Коэффициенты для каждого масштаба
    pub approximation: ArrayVec<f64, 256>,              // Аппроксимирующие коэффициенты
    pub details: ArrayVec<ArrayVec<f64, 256>, 8>,       // Детализирующие коэффициенты
    pub energy: ArrayVec<f64, 32>,                      // Энергия на каждом масштабе
    pub dominant_scale: f64,                            // Доминирующий масштаб
    pub wavelet_entropy: f64,                           // Вейвлет-энтропия
}

impl Default for WaveletCoefficients {
    fn default() -> Self {
        Self::new()
    }
}

impl WaveletCoefficients {
    pub fn new() -> Self {
        Self {
            scales: ArrayVec::new(),
            coefficients: ArrayVec::new(),
            approximation: ArrayVec::new(),
            details: ArrayVec::new(),
            energy: ArrayVec::new(),
            dominant_scale: 0.0,
            wavelet_entropy: 0.0,
        }
    }
}

/// Wavelet Transform
#[derive(Clone)]
pub struct WaveletTransform {
    // Временные данные
    time_series: ArrayVec<f64, 512>,
    
    // Результаты преобразования
    wavelet_coeffs: WaveletCoefficients,
    
    // Параметры
    wavelet_type: WaveletType,
    max_scales: usize,                  // Максимальное количество масштабов
    min_scale: f64,                     // Минимальный масштаб
    max_scale: f64,                     // Максимальный масштаб
    
    // Фильтры для дискретного вейвлет-преобразования
    low_pass_filter: ArrayVec<f64, 16>, // Низкочастотный фильтр
    high_pass_filter: ArrayVec<f64, 16>, // Высокочастотный фильтр
    
    // Параметры непрерывного вейвлет-преобразования
    morlet_omega: f64,                  // Параметр ω для вейвлета Морле
    
    // Состояние
    is_ready: bool,
    min_samples: usize,
}

impl WaveletTransform {
    pub fn new(wavelet_type: WaveletType, max_scales: usize) -> Self {
        let max_scales = max_scales.min(32);
        
        let mut transform = Self {
            time_series: ArrayVec::new(),
            wavelet_coeffs: WaveletCoefficients::new(),
            wavelet_type,
            max_scales,
            min_scale: 1.0,
            max_scale: 64.0,
            low_pass_filter: ArrayVec::new(),
            high_pass_filter: ArrayVec::new(),
            morlet_omega: 6.0,
            is_ready: false,
            min_samples: 32,
        };
        
        transform.initialize_filters();
        transform
    }
    
    /// Обновить вейвлет-преобразование новым значением
    pub fn update(&mut self, value: f64) -> &WaveletCoefficients {
        // Добавляем новое значение
        if self.time_series.len() >= 512 {
            self.time_series.remove(0);
        }
        if !self.time_series.is_full() {
            self.time_series.push(value);
        }
        
        // Если достаточно данных, вычисляем вейвлет-преобразование
        if self.time_series.len() >= self.min_samples {
            match self.wavelet_type {
                WaveletType::Morlet | WaveletType::Mexican => {
                    self.continuous_wavelet_transform();
                },
                _ => {
                    self.discrete_wavelet_transform();
                }
            }
            self.calculate_energy_and_entropy();
            self.is_ready = true;
        }
        
        &self.wavelet_coeffs
    }
    
    /// Инициализация фильтров для дискретного вейвлет-преобразования
    fn initialize_filters(&mut self) {
        self.low_pass_filter.clear();
        self.high_pass_filter.clear();
        
        match self.wavelet_type {
            WaveletType::Haar => {
                // Фильтры Хаара
                self.low_pass_filter.push(std::f64::consts::FRAC_1_SQRT_2);
                self.low_pass_filter.push(std::f64::consts::FRAC_1_SQRT_2);
                self.high_pass_filter.push(-std::f64::consts::FRAC_1_SQRT_2);
                self.high_pass_filter.push(std::f64::consts::FRAC_1_SQRT_2);
            },
            
            WaveletType::Daubechies4 => {
                // Коэффициенты Добеши-4
                let h = [
                    0.6830127018922193,
                    1.1830127018922193,
                    0.3169872981077807,
                    -0.1830127018922193,
                ];
                for &coeff in &h {
                    self.low_pass_filter.push(coeff);
                }
                
                // Высокочастотный фильтр (альтернирующие знаки)
                for (i, &coeff) in h.iter().enumerate() {
                    let sign = if i % 2 == 0 { -1.0 } else { 1.0 };
                    if !self.high_pass_filter.is_full() {
                        self.high_pass_filter.push(sign * coeff);
                    }
                }
                self.high_pass_filter.reverse();
            },
            
            WaveletType::Daubechies6 => {
                // Коэффициенты Добеши-6
                let h = [
                    0.47046721,
                    1.14111692,
                    0.650365,
                    -0.19093442,
                    -0.12083221,
                    0.0498175,
                ];
                for &coeff in &h {
                    self.low_pass_filter.push(coeff);
                }
                
                for (i, &coeff) in h.iter().enumerate() {
                    let sign = if i % 2 == 0 { -1.0 } else { 1.0 };
                    if !self.high_pass_filter.is_full() {
                        self.high_pass_filter.push(sign * coeff);
                    }
                }
                self.high_pass_filter.reverse();
            },
            
            WaveletType::Biorthogonal => {
                // Упрощенные биортогональные фильтры
                let h = [
                    -0.125, 0.25, 0.75, 0.25, -0.125
                ];
                for &coeff in &h {
                    self.low_pass_filter.push(coeff);
                }
                
                let g = [
                    0.5, 1.0, 0.5
                ];
                for &coeff in &g {
                    self.high_pass_filter.push(coeff);
                }
            },
            
            _ => {
                // По умолчанию используем Хаара
                self.low_pass_filter.push(std::f64::consts::FRAC_1_SQRT_2);
                self.low_pass_filter.push(std::f64::consts::FRAC_1_SQRT_2);
                self.high_pass_filter.push(-std::f64::consts::FRAC_1_SQRT_2);
                self.high_pass_filter.push(std::f64::consts::FRAC_1_SQRT_2);
            }
        }
    }
    
    /// Дискретное вейвлет-преобразование
    fn discrete_wavelet_transform(&mut self) {
        self.wavelet_coeffs.approximation.clear();
        self.wavelet_coeffs.details.clear();
        
        // Начинаем с исходного сигнала
        let mut current_signal = self.time_series.clone();
        
        // Выполняем разложение на несколько уровней
        for _level in 0..self.max_scales.min(8) {
            if current_signal.len() < 4 {
                break; // Недостаточно данных для дальнейшего разложения
            }
            
            let (approximation, detail) = self.single_level_dwt(&current_signal);
            
            // Сохраняем детализирующие коэффициенты (приводим к нужному размеру)
            if !self.wavelet_coeffs.details.is_full() {
                let mut detail_256 = ArrayVec::<f64, 256>::new();
                for (i, &val) in detail.iter().enumerate() {
                    if i >= 256 { break; }
                    detail_256.push(val);
                }
                self.wavelet_coeffs.details.push(detail_256);
            }
            
            // Переходим к следующему уровню с аппроксимацией
            current_signal = approximation;
        }
        
        // Последняя аппроксимация (приводим к нужному размеру)
        self.wavelet_coeffs.approximation.clear();
        for (i, &val) in current_signal.iter().enumerate() {
            if i >= 256 { break; }
            self.wavelet_coeffs.approximation.push(val);
        }
    }
    
    /// Одноуровневое дискретное вейвлет-преобразование
    fn single_level_dwt(&self, signal: &ArrayVec<f64, 512>) -> (ArrayVec<f64, 512>, ArrayVec<f64, 512>) {
        let mut approximation = ArrayVec::new();
        let mut detail = ArrayVec::new();
        
        let n = signal.len();
        
        // Свертка с низкочастотным фильтром (аппроксимация)
        for i in (0..n).step_by(2) {
            let mut approx_sum = 0.0;
            let mut detail_sum = 0.0;
            
            for (j, &h_coeff) in self.low_pass_filter.iter().enumerate() {
                let signal_idx = (i + j) % n; // Циклическое расширение
                approx_sum += h_coeff * signal[signal_idx];
            }
            
            for (j, &g_coeff) in self.high_pass_filter.iter().enumerate() {
                let signal_idx = (i + j) % n;
                detail_sum += g_coeff * signal[signal_idx];
            }
            
            if !approximation.is_full() {
                approximation.push(approx_sum);
            }
            if !detail.is_full() {
                detail.push(detail_sum);
            }
        }
        
        (approximation, detail)
    }
    
    /// Непрерывное вейвлет-преобразование
    fn continuous_wavelet_transform(&mut self) {
        self.wavelet_coeffs.scales.clear();
        self.wavelet_coeffs.coefficients.clear();
        
        let n = self.time_series.len();
        
        // Генерируем масштабы логарифмически
        for i in 0..self.max_scales {
            let scale = self.min_scale * (self.max_scale / self.min_scale).powf(i as f64 / (self.max_scales - 1) as f64);
            
            if !self.wavelet_coeffs.scales.is_full() {
                self.wavelet_coeffs.scales.push(scale);
            }
            
            let mut scale_coefficients = ArrayVec::new();
            
            // Вычисляем коэффициенты для текущего масштаба
            for t in 0..n {
                let coeff = self.compute_wavelet_coefficient(t, scale);
                if !scale_coefficients.is_full() {
                    scale_coefficients.push(coeff);
                }
            }
            
            if !self.wavelet_coeffs.coefficients.is_full() {
                self.wavelet_coeffs.coefficients.push(scale_coefficients);
            }
        }
    }
    
    /// Вычисление одного вейвлет-коэффициента
    fn compute_wavelet_coefficient(&self, position: usize, scale: f64) -> f64 {
        let mut coefficient = 0.0;
        let n = self.time_series.len();
        
        for k in 0..n {
            let t = (k as f64 - position as f64) / scale;
            let wavelet_value = self.evaluate_wavelet(t);
            coefficient += self.time_series[k] * wavelet_value;
        }
        
        coefficient / scale.sqrt()
    }
    
    /// Вычисление значения вейвлет-функции
    fn evaluate_wavelet(&self, t: f64) -> f64 {
        match self.wavelet_type {
            WaveletType::Morlet => {
                // Вейвлет Морле: e^(iωt) * e^(-t²/2)
                
                (self.morlet_omega * t).cos() * (-t * t / 2.0).exp()
            },
            
            WaveletType::Mexican => {
                // Мексиканская шляпа: (1 - t²) * e^(-t²/2)
                (1.0 - t * t) * (-t * t / 2.0).exp()
            },
            
            _ => {
                // Для дискретных вейвлетов возвращаем 0 (не используется в CWT)
                0.0
            }
        }
    }
    
    /// Вычисление энергии и энтропии
    fn calculate_energy_and_entropy(&mut self) {
        self.wavelet_coeffs.energy.clear();

        let mut total_energy = 0.0;
        let mut max_energy = 0.0;
        let mut dominant_scale_idx = 0;

        // For CWT: use coefficients array
        // For DWT: use details array
        let use_cwt = !self.wavelet_coeffs.coefficients.is_empty();

        if use_cwt {
            // CWT: энергия для каждого масштаба из coefficients
            for (i, coeffs) in self.wavelet_coeffs.coefficients.iter().enumerate() {
                let energy: f64 = coeffs.iter().map(|&c| c * c).sum();

                if !self.wavelet_coeffs.energy.is_full() {
                    self.wavelet_coeffs.energy.push(energy);
                }

                total_energy += energy;

                if energy > max_energy {
                    max_energy = energy;
                    dominant_scale_idx = i;
                }
            }

            // Доминирующий масштаб
            if dominant_scale_idx < self.wavelet_coeffs.scales.len() {
                self.wavelet_coeffs.dominant_scale = self.wavelet_coeffs.scales[dominant_scale_idx];
            }
        } else {
            // DWT: энергия для каждого уровня из details
            for (i, detail_coeffs) in self.wavelet_coeffs.details.iter().enumerate() {
                let energy: f64 = detail_coeffs.iter().map(|&c| c * c).sum();

                if !self.wavelet_coeffs.energy.is_full() {
                    self.wavelet_coeffs.energy.push(energy);
                }

                total_energy += energy;

                if energy > max_energy {
                    max_energy = energy;
                    dominant_scale_idx = i;
                }
            }

            // Доминирующий масштаб = 2^level для DWT
            self.wavelet_coeffs.dominant_scale = (1 << dominant_scale_idx) as f64;
        }

        // Вейвлет-энтропия
        self.wavelet_coeffs.wavelet_entropy = 0.0;
        if total_energy > 0.0 {
            for &energy in &self.wavelet_coeffs.energy {
                let p = energy / total_energy;
                if p > 0.0 {
                    self.wavelet_coeffs.wavelet_entropy -= p * p.ln();
                }
            }
        }
    }
    
    /// Получить вейвлет-коэффициенты
    pub fn coefficients(&self) -> &WaveletCoefficients {
        &self.wavelet_coeffs
    }
    
    /// Получить доминирующий масштаб
    pub fn dominant_scale(&self) -> f64 {
        self.wavelet_coeffs.dominant_scale
    }
    
    /// Получить вейвлет-энтропию
    pub fn wavelet_entropy(&self) -> f64 {
        self.wavelet_coeffs.wavelet_entropy
    }
    
    /// Получить энергию на масштабе
    pub fn energy_at_scale(&self, scale_index: usize) -> f64 {
        if scale_index < self.wavelet_coeffs.energy.len() {
            self.wavelet_coeffs.energy[scale_index]
        } else {
            0.0
        }
    }
    
    /// Установить параметры
    pub fn set_scale_range(&mut self, min_scale: f64, max_scale: f64) {
        self.min_scale = min_scale.max(0.1);
        self.max_scale = max_scale.max(self.min_scale);
    }
    
    /// Установить параметр Морле
    pub fn set_morlet_omega(&mut self, omega: f64) {
        self.morlet_omega = omega.max(1.0);
    }
    
    /// Проверить готовность
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    /// Update with OHLCV bar - uses close price
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> crate::bar_indicators::indicator_value::IndicatorValue {
        self.update(close);
        self.value()
    }

    /// Get current indicator value - returns wavelet entropy as main signal
    pub fn value(&self) -> crate::bar_indicators::indicator_value::IndicatorValue {
        crate::bar_indicators::indicator_value::IndicatorValue::Single(self.wavelet_coeffs.wavelet_entropy)
    }
    
    /// Получить тип вейвлета
    pub fn wavelet_type(&self) -> WaveletType {
        self.wavelet_type
    }
    
    /// Сбросить преобразование
    pub fn reset(&mut self) {
        self.time_series.clear();
        self.wavelet_coeffs = WaveletCoefficients::new();
        self.is_ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wavelet_creation() {
        let wt = WaveletTransform::new(WaveletType::Haar, 8);
        assert!(!wt.is_ready());
        assert_eq!(wt.wavelet_type(), WaveletType::Haar);
    }

    #[test]
    fn test_wavelet_types() {
        let types = [
            WaveletType::Haar,
            WaveletType::Daubechies4,
            WaveletType::Daubechies6,
            WaveletType::Morlet,
            WaveletType::Mexican,
            WaveletType::Biorthogonal,
        ];

        for wtype in types {
            let wt = WaveletTransform::new(wtype, 4);
            assert_eq!(wt.wavelet_type(), wtype);
        }
    }

    #[test]
    fn test_wavelet_warmup() {
        let mut wt = WaveletTransform::new(WaveletType::Haar, 8);
        for i in 0..50 {
            let value = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            wt.update(value);
        }
        assert!(wt.is_ready());
    }

    #[test]
    fn test_wavelet_coefficients() {
        let mut wt = WaveletTransform::new(WaveletType::Morlet, 8);
        for i in 0..50 {
            let value = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            wt.update(value);
        }

        if wt.is_ready() {
            let coeffs = wt.coefficients();
            assert!(coeffs.dominant_scale >= 0.0);
            assert!(coeffs.wavelet_entropy.is_finite());
        }
    }

    #[test]
    fn test_wavelet_reset() {
        let mut wt = WaveletTransform::new(WaveletType::Haar, 8);
        for i in 0..50 {
            wt.update(100.0 + i as f64);
        }
        wt.reset();
        assert!(!wt.is_ready());
    }
} 






















