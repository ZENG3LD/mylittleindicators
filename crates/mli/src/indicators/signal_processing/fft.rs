//! Fast Fourier Transform (FFT)
//! Быстрое преобразование Фурье для анализа частотного спектра временных рядов
//! Позволяет выявлять циклические паттерны и доминирующие частоты

use std::f64::consts::PI;

use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};

/// Fixed number of frequency bins in the spectrum VECTOR (= FFT_WINDOW/2 max, capped here).
const FFT_BINS: u16 = 256;

/// Комплексное число для FFT вычислений
#[derive(Debug, Clone, Copy)]
pub struct Complex {
    pub real: f64,
    pub imag: f64,
}

impl Complex {
    pub fn new(real: f64, imag: f64) -> Self {
        Self { real, imag }
    }
    
    pub fn zero() -> Self {
        Self { real: 0.0, imag: 0.0 }
    }
    
    pub fn magnitude(&self) -> f64 {
        (self.real * self.real + self.imag * self.imag).sqrt()
    }
    
    pub fn phase(&self) -> f64 {
        self.imag.atan2(self.real)
    }
    
    pub fn add(&self, other: &Complex) -> Complex {
        Complex {
            real: self.real + other.real,
            imag: self.imag + other.imag,
        }
    }
    
    pub fn sub(&self, other: &Complex) -> Complex {
        Complex {
            real: self.real - other.real,
            imag: self.imag - other.imag,
        }
    }
    
    pub fn mul(&self, other: &Complex) -> Complex {
        Complex {
            real: self.real * other.real - self.imag * other.imag,
            imag: self.real * other.imag + self.imag * other.real,
        }
    }
}

/// Частотная область
#[derive(Debug, Clone)]
pub struct FrequencyDomain {
    pub frequencies: Vec<f64>,        // Частоты
    pub magnitudes: Vec<f64>,         // Амплитуды
    pub phases: Vec<f64>,             // Фазы
    pub power_spectrum: Vec<f64>,     // Спектр мощности
    pub dominant_frequency: f64,                // Доминирующая частота
    pub dominant_period: f64,                   // Доминирующий период
    pub spectral_centroid: f64,                 // Спектральный центроид
    pub spectral_bandwidth: f64,                // Спектральная ширина
}

impl Default for FrequencyDomain {
    fn default() -> Self {
        Self::new()
    }
}

impl FrequencyDomain {
    pub fn new() -> Self {
        Self {
            frequencies: Vec::with_capacity(256),
            magnitudes: Vec::with_capacity(256),
            phases: Vec::with_capacity(256),
            power_spectrum: Vec::with_capacity(256),
            dominant_frequency: 0.0,
            dominant_period: 0.0,
            spectral_centroid: 0.0,
            spectral_bandwidth: 0.0,
        }
    }
}

/// Fast Fourier Transform
#[derive(Clone)]
pub struct FastFourierTransform {
    // Временные данные
    time_series: Vec<f64>,

    // Результаты FFT
    fft_result: Vec<Complex>,
    frequency_domain: FrequencyDomain,

    // Overlapped windowing
    prev_window_results: Vec<FrequencyDomain>,  // Хранение предыдущих результатов
    hop_size: usize,                                    // Размер шага между окнами

    // Параметры
    window_size: usize,                         // Размер окна для FFT (степень 2)
    sampling_rate: f64,                         // Частота дискретизации
    overlap_ratio: f64,                         // Коэффициент перекрытия окон

    // Оконные функции
    window_function: WindowType,
    window_coefficients: Vec<f64>,
    
    // Сглаживание спектра
    spectral_smoothing: bool,
    smoothing_factor: f64,
    
    // Состояние
    is_ready: bool,
    min_samples: usize,

    /// Per-frequency-bin magnitude spectrum as a `FFT_BINS × 1` VECTOR.
    /// Row = frequency bin index (0..N/2), value = magnitude. The matrix output behind
    /// `IndicatorOutputId::FftSpectrumGrid`.
    grid: MatrixGrid,
}

/// Типы оконных функций
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowType {
    Rectangular,    // Прямоугольное окно
    Hamming,       // Окно Хэмминга
    Hanning,       // Окно Ханнинга
    Blackman,      // Окно Блэкмана
    Kaiser,        // Окно Кайзера
}

impl FastFourierTransform {
    pub fn new(window_size: usize, sampling_rate: f64) -> Self {
        // Округляем размер окна до ближайшей степени 2
        let window_size = Self::next_power_of_2(window_size).min(256);
        let overlap_ratio = 0.5;  // 50% перекрытие по умолчанию
        let hop_size = (window_size as f64 * (1.0 - overlap_ratio)) as usize;
        
        let mut fft = Self {
            time_series: Vec::with_capacity(512),
            fft_result: Vec::with_capacity(256),
            frequency_domain: FrequencyDomain::new(),
            prev_window_results: Vec::with_capacity(8),
            hop_size,
            window_size,
            sampling_rate,
            overlap_ratio,
            window_function: WindowType::Hamming,
            window_coefficients: Vec::with_capacity(256),
            spectral_smoothing: true,
            smoothing_factor: 0.8,
            is_ready: false,
            min_samples: window_size * 2,
            grid: MatrixGrid::new(FFT_BINS, 1, false)
                .with_labels(AxisLabels::Ordinal("freq_bin"), AxisLabels::Bars),
        };
        
        fft.generate_window_coefficients();
        fft
    }
    
    /// Обновить FFT новым значением
    pub fn update(&mut self, value: f64) -> &FrequencyDomain {
        // Добавляем новое значение
        if self.time_series.len() >= 512 {
            self.time_series.remove(0);
        }
        self.time_series.push(value);
        
        // Если достаточно данных, вычисляем FFT
        if self.time_series.len() >= self.min_samples {
            self.compute_fft();
            self.is_ready = true;
        }
        
        &self.frequency_domain
    }
    
    /// Найти ближайшую степень 2
    fn next_power_of_2(n: usize) -> usize {
        let mut power = 1;
        while power < n {
            power *= 2;
        }
        power
    }
    
    /// Генерация коэффициентов оконной функции
    fn generate_window_coefficients(&mut self) {
        self.window_coefficients.clear();
        
        for i in 0..self.window_size {
            let coeff = match self.window_function {
                WindowType::Rectangular => 1.0,
                
                WindowType::Hamming => {
                    0.54 - 0.46 * (2.0 * PI * i as f64 / (self.window_size - 1) as f64).cos()
                },
                
                WindowType::Hanning => {
                    0.5 * (1.0 - (2.0 * PI * i as f64 / (self.window_size - 1) as f64).cos())
                },
                
                WindowType::Blackman => {
                    let n = (self.window_size - 1) as f64;
                    let i_f64 = i as f64;
                    0.42 - 0.5 * (2.0 * PI * i_f64 / n).cos() + 0.08 * (4.0 * PI * i_f64 / n).cos()
                },
                
                WindowType::Kaiser => {
                    // Упрощенная версия окна Кайзера (β = 5.0)
                    let beta = 5.0;
                    let n = (self.window_size - 1) as f64;
                    let i_f64 = i as f64;
                    let x = 2.0 * i_f64 / n - 1.0;
                    let bessel_i0_beta = Self::bessel_i0(beta);
                    let bessel_i0_arg = Self::bessel_i0(beta * (1.0 - x * x).sqrt());
                    bessel_i0_arg / bessel_i0_beta
                },
            };
            
            self.window_coefficients.push(coeff);
        }
    }
    
    /// Приближенная функция Бесселя I0 для окна Кайзера
    fn bessel_i0(x: f64) -> f64 {
        let mut sum = 1.0;
        let mut term = 1.0;
        let x_half_sq = (x / 2.0).powi(2);
        
        for k in 1..=20 {
            term *= x_half_sq / (k as f64).powi(2);
            sum += term;
            if term < 1e-10 {
                break;
            }
        }
        
        sum
    }
    
    /// Вычисление FFT
    fn compute_fft(&mut self) {
        if self.time_series.len() < self.window_size {
            return;
        }
        
        // Проверяем, пора ли вычислять новое окно (используем hop_size)
        let samples_since_last = if self.prev_window_results.is_empty() {
            self.window_size // Первое окно
        } else {
            self.hop_size
        };
        
        if self.time_series.len() < samples_since_last {
            return;
        }
        
        // Берем данные с учетом перекрытия (overlap_ratio)
        let start_idx = self.time_series.len() - self.window_size;
        let mut input_data: Vec<Complex> = Vec::with_capacity(256);
        
        // Применяем оконную функцию и преобразуем в комплексные числа
        for i in 0..self.window_size {
            let time_idx = start_idx + i;
            let windowed_value = if time_idx < self.time_series.len() {
                let window_coeff = if i < self.window_coefficients.len() {
                    self.window_coefficients[i]
                } else {
                    1.0
                };
                self.time_series[time_idx] * window_coeff
            } else {
                0.0
            };
            
            input_data.push(Complex::new(windowed_value, 0.0));
        }
        
        // Выполняем FFT
        self.fft_result = self.fft_radix2(&input_data);
        
        // Анализируем частотную область
        self.analyze_frequency_domain();
        
        // Сохраняем результат для усреднения с перекрытием
        if self.prev_window_results.len() >= 8 {
            self.prev_window_results.remove(0);
        }
        self.prev_window_results.push(self.frequency_domain.clone());
        
        // Применяем overlapped averaging для сглаживания
        self.apply_overlapped_averaging();

        // Populate the spectrum vector grid with per-bin magnitudes.
        self.fill_spectrum();
    }

    /// Populate `grid` with per-frequency-bin magnitude values from the current FFT result.
    /// Copies magnitudes into a local vec first to avoid borrow conflicts with `self.grid`.
    fn fill_spectrum(&mut self) {
        self.grid.reset();
        // Snapshot magnitudes to avoid simultaneous borrow of self.frequency_domain + self.grid.
        let mags: Vec<f64> = self.frequency_domain.magnitudes.clone();
        let num_bins = mags.len().min(FFT_BINS as usize);
        for b in 0..num_bins {
            self.grid.set_direction(MatrixCell::new(b as u16, 0), mags[b].abs());
        }
    }
    
    /// Упрощенная реализация FFT radix-2
    fn fft_radix2(&self, input: &Vec<Complex>) -> Vec<Complex> {
        let n = input.len();
        if n <= 1 {
            return input.clone();
        }
        
        // Проверяем, что n - степень 2
        if n & (n - 1) != 0 {
            return input.clone();
        }
        
        let mut result = input.clone();
        
        // Bit-reversal permutation
        let mut j = 0;
        for i in 1..n {
            let mut bit = n >> 1;
            while j & bit != 0 {
                j ^= bit;
                bit >>= 1;
            }
            j ^= bit;
            
            if i < j && i < result.len() && j < result.len() {
                let temp = result[i];
                result[i] = result[j];
                result[j] = temp;
            }
        }
        
        // Cooley-Tukey FFT
        let mut length = 2;
        while length <= n {
            let angle = -2.0 * PI / length as f64;
            let wlen = Complex::new(angle.cos(), angle.sin());
            
            for i in (0..n).step_by(length) {
                let mut w = Complex::new(1.0, 0.0);
                
                for j in 0..length/2 {
                    let u_idx = i + j;
                    let v_idx = i + j + length / 2;
                    
                    if u_idx < result.len() && v_idx < result.len() {
                        let u = result[u_idx];
                        let v = result[v_idx].mul(&w);
                        
                        result[u_idx] = u.add(&v);
                        result[v_idx] = u.sub(&v);
                    }
                    
                    w = w.mul(&wlen);
                }
            }
            
            length *= 2;
        }
        
        result
    }
    
    /// Анализ частотной области
    fn analyze_frequency_domain(&mut self) {
        self.frequency_domain.frequencies.clear();
        self.frequency_domain.magnitudes.clear();
        self.frequency_domain.phases.clear();
        self.frequency_domain.power_spectrum.clear();
        
        let n = self.fft_result.len();
        let nyquist_freq = self.sampling_rate / 2.0;
        
        // Анализируем только первую половину спектра (из-за симметрии)
        let half_n = n / 2;
        
        let mut max_magnitude = 0.0;
        let mut dominant_freq_idx = 0;
        let mut spectral_sum = 0.0;
        let mut weighted_freq_sum = 0.0;
        
        for i in 0..half_n {
            if i >= self.fft_result.len() {
                break;
            }
            
            // Частота
            let frequency = (i as f64 * nyquist_freq) / half_n as f64;
            
            // Амплитуда и фаза
            let complex_val = &self.fft_result[i];
            let magnitude = complex_val.magnitude();
            let phase = complex_val.phase();
            let power = magnitude * magnitude;
            
            // Сглаживание спектра
            let smoothed_magnitude = if self.spectral_smoothing && i > 0 && 
                i < self.frequency_domain.magnitudes.len() {
                let prev_mag = self.frequency_domain.magnitudes[i - 1];
                self.smoothing_factor * prev_mag + (1.0 - self.smoothing_factor) * magnitude
            } else {
                magnitude
            };
            
            // Сохраняем результаты
            self.frequency_domain.frequencies.push(frequency);
            self.frequency_domain.magnitudes.push(smoothed_magnitude);
            self.frequency_domain.phases.push(phase);
            self.frequency_domain.power_spectrum.push(power);
            
            // Ищем доминирующую частоту (исключаем DC компоненту)
            if i > 0 && smoothed_magnitude > max_magnitude {
                max_magnitude = smoothed_magnitude;
                dominant_freq_idx = i;
            }
            
            // Для спектрального центроида
            spectral_sum += power;
            weighted_freq_sum += frequency * power;
        }
        
        // Доминирующая частота и период
        if dominant_freq_idx < self.frequency_domain.frequencies.len() {
            self.frequency_domain.dominant_frequency = self.frequency_domain.frequencies[dominant_freq_idx];
            self.frequency_domain.dominant_period = if self.frequency_domain.dominant_frequency > 0.0 {
                1.0 / self.frequency_domain.dominant_frequency
            } else {
                0.0
            };
        }
        
        // Спектральный центроид
        self.frequency_domain.spectral_centroid = if spectral_sum > 0.0 {
            weighted_freq_sum / spectral_sum
        } else {
            0.0
        };
        
        // Спектральная ширина (упрощенная версия)
        let mut bandwidth_sum = 0.0;
        for (i, &freq) in self.frequency_domain.frequencies.iter().enumerate() {
            if i < self.frequency_domain.power_spectrum.len() {
                let power = self.frequency_domain.power_spectrum[i];
                let freq_diff = freq - self.frequency_domain.spectral_centroid;
                bandwidth_sum += freq_diff * freq_diff * power;
            }
        }
        
        self.frequency_domain.spectral_bandwidth = if spectral_sum > 0.0 {
            (bandwidth_sum / spectral_sum).sqrt()
        } else {
            0.0
        };
    }
    
    /// Применяет overlapped averaging для сглаживания спектра
    fn apply_overlapped_averaging(&mut self) {
        // Skip averaging if no previous results to average with
        if self.prev_window_results.is_empty() {
            return;
        }

        // Get the spectrum length from current result
        let spectrum_len = self.frequency_domain.magnitudes.len();
        if spectrum_len == 0 {
            return;
        }

        let mut averaged_magnitudes: Vec<f64> = Vec::with_capacity(256);
        let mut averaged_phases: Vec<f64> = Vec::with_capacity(256);
        let mut averaged_power_spectrum: Vec<f64> = Vec::with_capacity(256);

        // Average each frequency bin across all previous windows
        for bin_idx in 0..spectrum_len {
            let mut sum_mag = 0.0;
            let mut sum_phase = 0.0;
            let mut sum_power = 0.0;
            let mut count = 0;

            for prev_result in self.prev_window_results.iter() {
                if bin_idx < prev_result.magnitudes.len() {
                    sum_mag += prev_result.magnitudes[bin_idx];
                    sum_phase += prev_result.phases[bin_idx];
                    sum_power += prev_result.power_spectrum[bin_idx];
                    count += 1;
                }
            }

            if count > 0 {
                averaged_magnitudes.push(sum_mag / count as f64);
                averaged_phases.push(sum_phase / count as f64);
                averaged_power_spectrum.push(sum_power / count as f64);
            }
        }

        // Only update if we have valid averaged data
        if !averaged_magnitudes.is_empty() {
            self.frequency_domain.magnitudes = averaged_magnitudes;
            self.frequency_domain.phases = averaged_phases;
            self.frequency_domain.power_spectrum = averaged_power_spectrum;
            // Keep frequencies unchanged - they don't need averaging
        }
    }
    
    /// The per-frequency-bin magnitude spectrum as a `FFT_BINS × 1` VECTOR.
    /// Row = bin index (0..N/2), value = magnitude. The matrix output behind
    /// `IndicatorOutputId::FftSpectrumGrid`.
    pub fn spectrum_grid(&self) -> &MatrixGrid {
        &self.grid
    }

    /// Получить частотную область
    pub fn frequency_domain(&self) -> &FrequencyDomain {
        &self.frequency_domain
    }
    
    /// Получить доминирующую частоту
    pub fn dominant_frequency(&self) -> f64 {
        self.frequency_domain.dominant_frequency
    }
    
    /// Получить доминирующий период
    pub fn dominant_period(&self) -> f64 {
        self.frequency_domain.dominant_period
    }
    
    /// Получить спектральный центроид
    pub fn spectral_centroid(&self) -> f64 {
        self.frequency_domain.spectral_centroid
    }
    
    /// Установить тип оконной функции
    pub fn set_window_type(&mut self, window_type: WindowType) {
        self.window_function = window_type;
        self.generate_window_coefficients();
    }
    
    /// Установить параметры сглаживания
    pub fn set_spectral_smoothing(&mut self, enabled: bool, factor: f64) {
        self.spectral_smoothing = enabled;
        self.smoothing_factor = factor.clamp(0.0, 1.0);
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

    /// Установить коэффициент перекрытия окон
    pub fn set_overlap_ratio(&mut self, ratio: f64) {
        self.overlap_ratio = ratio.clamp(0.0, 1.0);
        self.hop_size = (self.window_size as f64 * (1.0 - self.overlap_ratio)) as usize;
    }

    /// Получить коэффициент перекрытия окон
    pub fn overlap_ratio(&self) -> f64 {
        self.overlap_ratio
    }
    
    /// Сбросить FFT
    pub fn reset(&mut self) {
        self.time_series.clear();
        self.fft_result.clear();
        self.frequency_domain = FrequencyDomain::new();
        self.prev_window_results.clear();  // Очищаем overlapped результаты
        self.is_ready = false;
        self.grid.reset();
    }

    pub fn value(&self) -> f64 {
        // Return dominant period as the main value
        self.frequency_domain.dominant_period
    }

    /// Получить размер шага между окнами
    pub fn hop_size(&self) -> usize {
        self.hop_size
    }

    /// Получить полную конфигурацию FFT
    pub fn get_config(&self) -> FFTConfig {
        FFTConfig {
            window_size: self.window_size,
            sampling_rate: self.sampling_rate,
            overlap_ratio: self.overlap_ratio,
            hop_size: self.hop_size,
            window_function: self.window_function,
            spectral_smoothing: self.spectral_smoothing,
            smoothing_factor: self.smoothing_factor,
        }
    }

    /// Установить новую конфигурацию FFT
    pub fn set_config(&mut self, config: FFTConfig) {
        self.window_size = Self::next_power_of_2(config.window_size).min(256);
        self.sampling_rate = config.sampling_rate;
        self.set_overlap_ratio(config.overlap_ratio);
        self.window_function = config.window_function;
        self.spectral_smoothing = config.spectral_smoothing;
        self.smoothing_factor = config.smoothing_factor;
        
        // Пересоздаем оконные коэффициенты
        self.generate_window_coefficients();
        self.reset();
    }

    /// Получить количество обработанных окон
    pub fn processed_windows_count(&self) -> usize {
        self.prev_window_results.len()
    }

    /// Feed one pre-extracted scalar (the factory resolves the configured price field).
    pub fn feed(&mut self, v: f64) -> f64 {
        self.update(v);
        self.frequency_domain.dominant_period
    }
}

/// Конфигурация FFT
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FFTConfig {
    pub window_size: usize,
    pub sampling_rate: f64,
    pub overlap_ratio: f64,
    pub hop_size: usize,
    pub window_function: WindowType,
    pub spectral_smoothing: bool,
    pub smoothing_factor: f64,
}

impl std::fmt::Debug for FastFourierTransform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FastFourierTransform")
            .field("window_size", &self.window_size)
            .field("sampling_rate", &self.sampling_rate)
            .field("overlap_ratio", &self.overlap_ratio)
            .field("hop_size", &self.hop_size)
            .field("window_function", &self.window_function)
            .field("spectral_smoothing", &self.spectral_smoothing)
            .field("smoothing_factor", &self.smoothing_factor)
            .field("dominant_frequency", &self.frequency_domain.dominant_frequency)
            .field("dominant_period", &self.frequency_domain.dominant_period)
            .field("spectral_centroid", &self.frequency_domain.spectral_centroid)
            .field("is_ready", &self.is_ready)
            .field("processed_windows", &self.prev_window_results.len())
            .finish()
    }
}

impl Default for FastFourierTransform {
    /// Factory default: window=128, sampling_rate=1.0.
    fn default() -> Self {
        Self::new(128, 1.0)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity, ValueDomain};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`FastFourierTransform`]: window size and sampling rate.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FftConfig {
    pub window_size: Param<usize>,
    pub sampling_rate: Param<f64>,
}

impl Indicator for FastFourierTransform {
    const ID: IndicatorId = IndicatorId::Fft;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(window * log(window)) per bar: overlapping radix-2 FFT.
    /// One large rolling Vec (512 max) + result Vec.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::fixed(StoreKind::Vec, 512), Store::fixed(StoreKind::Vec, 256)],
    );
    const OUTPUTS: &'static [Output] = &[
        crate::contract::Output::count(IndicatorOutputId::Fft),
        Output::matrix(IndicatorOutputId::FftSpectrumGrid, ValueDomain::Magnitude),
    ];
    type Config = FftConfig;
    type Runtime = FastFourierTransform;

    fn create(cfg: FftConfig) -> FastFourierTransform {
        FastFourierTransform::new(cfg.window_size.resolved(), cfg.sampling_rate.resolved())
    }

    fn source_fields(cfg: &FftConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for FftConfig {
    fn defaults() -> Self {
        FftConfig { window_size: Param::Solo(128), sampling_rate: Param::Solo(1.0) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // window_size: Class A usize → auto range(2,4048,1)
        // sampling_rate: PIN (bar rate Hz) — f64 auto leaves Solo; do not touch
        Self::machine_defaults_auto()
    }
}


impl Render for FastFourierTransform {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Fft, "FFT Dominant Period", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_creation() {
        let fft = FastFourierTransform::new(64, 100.0);
        assert!(!fft.is_ready());
        assert_eq!(fft.window_size(), 64);
        assert_eq!(fft.sampling_rate(), 100.0);
    }

    #[test]
    fn test_fft_update() {
        let mut fft = FastFourierTransform::new(32, 100.0);
        for i in 0..200 {
            let value = (i as f64 * 0.1).sin() * 10.0 + 100.0;
            fft.update(value);
        }
        assert!(fft.is_ready());
        assert!(fft.dominant_frequency().is_finite());
    }

    #[test]
    fn test_fft_complex() {
        let c1 = Complex::new(3.0, 4.0);
        assert!((c1.magnitude() - 5.0).abs() < 1e-9);
        let c2 = Complex::new(1.0, 2.0);
        let sum = c1.add(&c2);
        assert!((sum.real - 4.0).abs() < 1e-9);
        assert!((sum.imag - 6.0).abs() < 1e-9);
    }

    #[test]
    fn factory_feeds_resolved_fft() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<FastFourierTransform as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Fft(cfg).build_solo().unwrap();
        for i in 0..300 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite());
    }

    #[test]
    fn test_fft_reset() {
        let mut fft = FastFourierTransform::new(32, 100.0);
        for i in 0..200 {
            fft.update(100.0 + (i as f64 * 0.1).sin() * 10.0);
        }
        assert!(fft.is_ready());
        fft.reset();
        assert!(!fft.is_ready());
    }

    #[test]
    fn spectrum_grid_row_label() {
        use crate::engine::matrix_grid::Label;
        let fft = FastFourierTransform::new(64, 100.0);
        let g = fft.spectrum_grid();
        assert_eq!(g.row_label(5), Label::Index(5));
    }

    #[test]
    fn spectrum_grid_populates() {
        // Feed a sine wave: window=64, need min_samples=128 → feed 200 samples to be safe.
        let mut fft = FastFourierTransform::new(64, 100.0);
        for i in 0..200u32 {
            let v = 100.0 + (i as f64 * 2.0 * std::f64::consts::PI / 32.0).sin() * 10.0;
            fft.update(v);
        }
        assert!(fft.is_ready(), "FFT should be ready after 200 samples");

        let g = fft.spectrum_grid();
        assert_eq!(g.cols(), 1, "spectrum is a Nx1 vector");
        assert_eq!(g.rows(), FFT_BINS, "spectrum has FFT_BINS rows");

        // All stored magnitudes must be non-negative.
        for b in 0..g.rows() {
            let mag = g.read_direction(MatrixCell::new(b, 0));
            assert!(mag >= 0.0, "negative magnitude at bin {b}: {mag}");
        }

        // At least one bin should be non-zero (we fed a non-trivial sine wave).
        let has_nonzero = (0..g.rows()).any(|b| g.read_direction(MatrixCell::new(b, 0)) != 0.0);
        assert!(has_nonzero, "spectrum grid should have at least one non-zero bin");
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::engine::indicator_id::IndicatorId;
    use crate::contract::MarketSample;

    #[test]
    fn factory_exposes_fft_spectrum_vector() {
        let cfg = <<FastFourierTransform as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Fft(cfg).build_solo().unwrap();
        // Feed enough bars to make FFT ready (window=128, min_samples=256 → 300 to be safe).
        for i in 0..300u32 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1.0,
            });
        }
        let g = f
            .grid(IndicatorOutputId::FftSpectrumGrid)
            .expect("Fft emits the FftSpectrumGrid vector");
        assert_eq!(g.rows(), FFT_BINS, "spectrum has FFT_BINS rows");
        assert_eq!(g.cols(), 1, "spectrum is a Nx1 vector");

        // A scalar-only producer returns None for a foreign matrix id.
        let sma = IndicatorOrder::from_defaults(IndicatorId::Sma).unwrap().build_solo().unwrap();
        assert!(sma.grid(IndicatorOutputId::FftSpectrumGrid).is_none());
    }
}






















