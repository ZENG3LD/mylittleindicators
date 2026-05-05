//! Butterworth Filter
//! Фильтр Баттерворта для сглаживания временных рядов
//! Обеспечивает максимально плоскую частотную характеристику в полосе пропускания

use arrayvec::ArrayVec;
use std::f64::consts::PI;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Тип фильтра Баттерворта
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterType {
    LowPass,    // Низкочастотный фильтр
    HighPass,   // Высокочастотный фильтр
    BandPass,   // Полосовой фильтр
    BandStop,   // Режекторный фильтр
}

/// Butterworth Filter
#[derive(Clone)]
pub struct ButterworthFilter {
    // Параметры фильтра
    filter_type: FilterType,
    order: usize,                           // Порядок фильтра (1-8)
    cutoff_frequency: f64,                  // Частота среза (нормализованная)
    low_cutoff: f64,                        // Нижняя частота среза (для полосовых фильтров)
    high_cutoff: f64,                       // Верхняя частота среза (для полосовых фильтров)
    sampling_rate: f64,                     // Частота дискретизации
    
    // Коэффициенты фильтра (IIR)
    a_coefficients: ArrayVec<f64, 16>,      // Коэффициенты знаменателя
    b_coefficients: ArrayVec<f64, 16>,      // Коэффициенты числителя
    
    // Буферы для задержки
    x_buffer: ArrayVec<f64, 16>,            // Входные значения
    y_buffer: ArrayVec<f64, 16>,            // Выходные значения
    
    // Результаты
    filtered_value: f64,                    // Текущее отфильтрованное значение
    filter_response: ArrayVec<f64, 256>,    // Импульсная характеристика
    
    // Состояние
    is_initialized: bool,
}

impl ButterworthFilter {
    pub fn new(filter_type: FilterType, order: usize, cutoff_frequency: f64, sampling_rate: f64) -> Self {
        let order = order.clamp(1, 8);
        let sampling_rate = sampling_rate.max(1.0); // Минимальная частота дискретизации
        
        // Конвертируем частоту среза из Hz в нормализованную частоту [0, 0.5)
        let nyquist_freq = sampling_rate / 2.0;
        let normalized_cutoff = (cutoff_frequency / nyquist_freq).clamp(0.001, 0.499);
        
        let mut filter = Self {
            filter_type,
            order,
            cutoff_frequency: normalized_cutoff,  // Теперь хранится нормализованная частота
            low_cutoff: 0.1,
            high_cutoff: 0.4,
            sampling_rate,
            a_coefficients: ArrayVec::new(),
            b_coefficients: ArrayVec::new(),
            x_buffer: ArrayVec::new(),
            y_buffer: ArrayVec::new(),
            filtered_value: 0.0,
            filter_response: ArrayVec::new(),
            is_initialized: false,
        };
        
        filter.design_filter();
        filter.compute_impulse_response();  // Вычисляем импульсную характеристику
        filter
    }
    
    /// Создать полосовой или режекторный фильтр
    pub fn new_band_filter(
        filter_type: FilterType, 
        order: usize, 
        low_cutoff: f64, 
        high_cutoff: f64,
        sampling_rate: f64
    ) -> Self {
        let order = order.clamp(1, 8);
        let sampling_rate = sampling_rate.max(1.0);
        
        // Конвертируем частоты среза из Hz в нормализованные частоты
        let nyquist_freq = sampling_rate / 2.0;
        let normalized_low = (low_cutoff / nyquist_freq).clamp(0.001, 0.499);
        let normalized_high = (high_cutoff / nyquist_freq).max(normalized_low + 0.001).min(0.499);
        
        let mut filter = Self {
            filter_type,
            order,
            cutoff_frequency: (normalized_low + normalized_high) / 2.0, // Центральная частота
            low_cutoff: normalized_low,
            high_cutoff: normalized_high,
            sampling_rate,
            a_coefficients: ArrayVec::new(),
            b_coefficients: ArrayVec::new(),
            x_buffer: ArrayVec::new(),
            y_buffer: ArrayVec::new(),
            filtered_value: 0.0,
            filter_response: ArrayVec::new(),
            is_initialized: false,
        };
        
        filter.design_filter();
        filter.compute_impulse_response();  // Вычисляем импульсную характеристику
        filter
    }
    
    /// Обновить фильтр новым значением
    pub fn update(&mut self, value: f64) -> f64 {
        if !self.is_initialized {
            // Инициализируем буферы первым значением
            self.initialize_buffers(value);
            self.is_initialized = true;
            self.filtered_value = value;
            return value;
        }
        
        // Добавляем новое входное значение
        if self.x_buffer.len() > self.order {
            self.x_buffer.remove(0);
        }
        if !self.x_buffer.is_full() {
            self.x_buffer.push(value);
        }
        
        // Вычисляем выходное значение по формуле IIR фильтра
        self.filtered_value = self.compute_filter_output();
        
        // Добавляем новое выходное значение в буфер
        if self.y_buffer.len() >= self.order {
            self.y_buffer.remove(0);
        }
        if !self.y_buffer.is_full() {
            self.y_buffer.push(self.filtered_value);
        }
        
        self.filtered_value
    }
    
    /// Инициализация буферов
    fn initialize_buffers(&mut self, initial_value: f64) {
        self.x_buffer.clear();
        self.y_buffer.clear();
        
        // Заполняем буферы начальным значением
        for _ in 0..=self.order {
            if !self.x_buffer.is_full() {
                self.x_buffer.push(initial_value);
            }
        }
        
        for _ in 0..self.order {
            if !self.y_buffer.is_full() {
                self.y_buffer.push(initial_value);
            }
        }
    }
    
    /// Проектирование фильтра
    fn design_filter(&mut self) {
        self.a_coefficients.clear();
        self.b_coefficients.clear();
        
        match self.filter_type {
            FilterType::LowPass => self.design_lowpass(),
            FilterType::HighPass => self.design_highpass(),
            FilterType::BandPass => self.design_bandpass(),
            FilterType::BandStop => self.design_bandstop(),
        }
        
        // Нормализуем коэффициенты
        self.normalize_coefficients();
    }
    
    /// Проектирование низкочастотного фильтра
    fn design_lowpass(&mut self) {
        // Упрощенная реализация для низкочастотного фильтра Баттерворта
        let wc = 2.0 * PI * self.cutoff_frequency; // Угловая частота среза
        let wc_tan = (wc / 2.0).tan(); // Предыскажение
        
        match self.order {
            1 => {
                // Первый порядок: H(s) = 1/(s + 1)
                let k = wc_tan;
                for &val in &[k, k] {
                    if !self.b_coefficients.is_full() { self.b_coefficients.push(val); }
                }
                for &val in &[1.0 + k, k - 1.0] {
                    if !self.a_coefficients.is_full() { self.a_coefficients.push(val); }
                }
            },
            
            2 => {
                // Второй порядок: H(s) = 1/(s² + √2*s + 1)
                let k = wc_tan;
                let k2 = k * k;
                let sqrt2 = 2.0_f64.sqrt();
                let norm = 1.0 + sqrt2 * k + k2;
                
                for &val in &[k2, 2.0 * k2, k2] {
                    if !self.b_coefficients.is_full() { self.b_coefficients.push(val); }
                }
                for &val in &[norm, 2.0 * (k2 - 1.0), 1.0 - sqrt2 * k + k2] {
                    if !self.a_coefficients.is_full() { self.a_coefficients.push(val); }
                }
            },
            
            _ => {
                // Для высших порядков используем каскад секций второго порядка
                self.design_higher_order_lowpass();
            }
        }
    }
    
    /// Проектирование высокочастотного фильтра
    fn design_highpass(&mut self) {
        // Высокочастотный фильтр получается заменой s -> 1/s в низкочастотном
        let wc = 2.0 * PI * self.cutoff_frequency;
        let wc_tan = (wc / 2.0).tan();
        
        match self.order {
            1 => {
                let k = wc_tan;
                for &val in &[1.0, -1.0] {
                    if !self.b_coefficients.is_full() { self.b_coefficients.push(val); }
                }
                for &val in &[1.0 + k, k - 1.0] {
                    if !self.a_coefficients.is_full() { self.a_coefficients.push(val); }
                }
            },
            
            2 => {
                let k = wc_tan;
                let k2 = k * k;
                let sqrt2 = 2.0_f64.sqrt();
                let norm = 1.0 + sqrt2 * k + k2;
                
                self.b_coefficients.push(1.0);
                self.b_coefficients.push(-2.0);
                self.b_coefficients.push(1.0);
                self.a_coefficients.push(norm);
                self.a_coefficients.push(2.0 * (k2 - 1.0));
                self.a_coefficients.push(1.0 - sqrt2 * k + k2);
            },
            
            _ => {
                self.design_higher_order_highpass();
            }
        }
    }
    
    /// Проектирование полосового фильтра
    fn design_bandpass(&mut self) {
        // Упрощенная реализация полосового фильтра
        let w1 = 2.0 * PI * self.low_cutoff;
        let w2 = 2.0 * PI * self.high_cutoff;
        let w0 = (w1 * w2).sqrt(); // Центральная частота
        let bw = w2 - w1; // Ширина полосы
        
        let q = w0 / bw; // Добротность
        let k = (w0 / 2.0).tan();
        let norm = 1.0 + k / q + k * k;
        
                    self.b_coefficients.push(k / q);
            self.b_coefficients.push(0.0);
            self.b_coefficients.push(-k / q);
            self.a_coefficients.push(norm);
            self.a_coefficients.push(2.0 * (k * k - 1.0));
            self.a_coefficients.push(1.0 - k / q + k * k);
    }
    
    /// Проектирование режекторного фильтра
    fn design_bandstop(&mut self) {
        // Режекторный фильтр (обратный полосовому)
        let w1 = 2.0 * PI * self.low_cutoff;
        let w2 = 2.0 * PI * self.high_cutoff;
        let w0 = (w1 * w2).sqrt();
        let bw = w2 - w1;
        
        let q = w0 / bw;
        let k = (w0 / 2.0).tan();
        let norm = 1.0 + k / q + k * k;
        
                    self.b_coefficients.push(1.0 + k * k);
            self.b_coefficients.push(2.0 * (k * k - 1.0));
            self.b_coefficients.push(1.0 + k * k);
            self.a_coefficients.push(norm);
            self.a_coefficients.push(2.0 * (k * k - 1.0));
            self.a_coefficients.push(1.0 - k / q + k * k);
    }
    
    /// Проектирование низкочастотного фильтра высокого порядка
    fn design_higher_order_lowpass(&mut self) {
        // Используем каскад секций второго порядка
        let sections = self.order.div_ceil(2);
        
        // Для простоты используем один эквивалентный фильтр второго порядка
        let wc = 2.0 * PI * self.cutoff_frequency;
        let wc_tan = (wc / 2.0).tan();
        let k = wc_tan;
        let k2 = k * k;
        
        // Коэффициент демпфирования для критического демпфирования
        let zeta = 1.0 / (2.0 * (sections as f64).cos());
        let norm = 1.0 + 2.0 * zeta * k + k2;
        
                    self.b_coefficients.push(k2);
            self.b_coefficients.push(2.0 * k2);
            self.b_coefficients.push(k2);
            self.a_coefficients.push(norm);
            self.a_coefficients.push(2.0 * (k2 - 1.0));
            self.a_coefficients.push(1.0 - 2.0 * zeta * k + k2);
    }
    
    /// Проектирование высокочастотного фильтра высокого порядка
    fn design_higher_order_highpass(&mut self) {
        let sections = self.order.div_ceil(2);
        let wc = 2.0 * PI * self.cutoff_frequency;
        let wc_tan = (wc / 2.0).tan();
        let k = wc_tan;
        let k2 = k * k;
        
        let zeta = 1.0 / (2.0 * (sections as f64).cos());
        let norm = 1.0 + 2.0 * zeta * k + k2;
        
                    self.b_coefficients.push(1.0);
            self.b_coefficients.push(-2.0);
            self.b_coefficients.push(1.0);
            self.a_coefficients.push(norm);
            self.a_coefficients.push(2.0 * (k2 - 1.0));
            self.a_coefficients.push(1.0 - 2.0 * zeta * k + k2);
    }
    
    /// Нормализация коэффициентов фильтра
    fn normalize_coefficients(&mut self) {
        if let Some(&a0) = self.a_coefficients.first() {
            if a0.abs() > 1e-12 {
                // Нормализуем все коэффициенты относительно a[0]
                for coeff in &mut self.a_coefficients {
                    *coeff /= a0;
                }
                for coeff in &mut self.b_coefficients {
                    *coeff /= a0;
                }
            }
        }
    }

    /// Вычисление импульсной характеристики фильтра
    fn compute_impulse_response(&mut self) {
        self.filter_response.clear();
        
        // Сохраняем текущее состояние буферов
        let x_backup = self.x_buffer.clone();
        let y_backup = self.y_buffer.clone();
        
        // Очищаем буферы для чистого расчета
        self.x_buffer.clear();
        self.y_buffer.clear();
        
        // Инициализируем буферы нулями
        for _ in 0..self.order.max(self.a_coefficients.len()).max(self.b_coefficients.len()) {
            if !self.x_buffer.is_full() { self.x_buffer.push(0.0); }
            if !self.y_buffer.is_full() { self.y_buffer.push(0.0); }
        }
        
        // Подаем импульс (1.0 в первый момент, затем нули)
        for i in 0..256.min(self.filter_response.capacity()) {
            let input = if i == 0 { 1.0 } else { 0.0 };
            
            // Сдвигаем входной буфер
            if !self.x_buffer.is_empty() {
                for j in (1..self.x_buffer.len()).rev() {
                    if j < self.x_buffer.len() {
                        self.x_buffer[j] = self.x_buffer[j - 1];
                    }
                }
                self.x_buffer[0] = input;
            }
            
            // Вычисляем выход
            let output = self.compute_filter_output();
            
            // Сдвигаем выходной буфер
            if !self.y_buffer.is_empty() {
                for j in (1..self.y_buffer.len()).rev() {
                    if j < self.y_buffer.len() {
                        self.y_buffer[j] = self.y_buffer[j - 1];
                    }
                }
                self.y_buffer[0] = output;
            }
            
            if !self.filter_response.is_full() {
                self.filter_response.push(output);
            }
        }
        
        // Восстанавливаем исходное состояние буферов
        self.x_buffer = x_backup;
        self.y_buffer = y_backup;
    }
    
    /// Вычисление выходного значения фильтра
    fn compute_filter_output(&self) -> f64 {
        let mut output = 0.0;
        
        // Прямая ветвь (числитель): Σ b[k] * x[n-k]
        for (i, &b_coeff) in self.b_coefficients.iter().enumerate() {
            if i < self.x_buffer.len() {
                let x_idx = self.x_buffer.len() - 1 - i;
                output += b_coeff * self.x_buffer[x_idx];
            }
        }
        
        // Обратная ветвь (знаменатель): -Σ a[k] * y[n-k] (k > 0)
        for (i, &a_coeff) in self.a_coefficients.iter().enumerate().skip(1) {
            let y_idx = i - 1;
            if y_idx < self.y_buffer.len() {
                let y_val_idx = self.y_buffer.len() - 1 - y_idx;
                output -= a_coeff * self.y_buffer[y_val_idx];
            }
        }
        
        output
    }
    
    /// Получить отфильтрованное значение
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.filtered_value)
    }
    
    /// Получить тип фильтра
    pub fn filter_type(&self) -> FilterType {
        self.filter_type
    }
    
    /// Получить порядок фильтра
    pub fn order(&self) -> usize {
        self.order
    }
    
    /// Получить частоту среза
    pub fn cutoff_frequency(&self) -> f64 {
        self.cutoff_frequency
    }
    
    /// Получить коэффициенты фильтра
    pub fn coefficients(&self) -> (&[f64], &[f64]) {
        (&self.a_coefficients, &self.b_coefficients)
    }
    
    /// Установить новую частоту среза
    pub fn set_cutoff_frequency(&mut self, cutoff: f64) {
        self.cutoff_frequency = cutoff.clamp(0.001, 0.499);
        self.design_filter();
        self.compute_impulse_response(); // Пересчитываем импульсную характеристику при изменении частоты среза
    }
    
    /// Проверить готовность фильтра
    pub fn is_ready(&self) -> bool {
        self.is_initialized
    }
    
    /// Сбросить фильтр
    pub fn reset(&mut self) {
        self.x_buffer.clear();
        self.y_buffer.clear();
        self.filtered_value = 0.0;
        self.is_initialized = false;
    }

    /// Получить частоту дискретизации
    pub fn get_sampling_rate(&self) -> f64 {
        self.sampling_rate
    }

    /// Установить новую частоту дискретизации (пересчитывает нормализованные частоты)
    pub fn set_sampling_rate(&mut self, new_sampling_rate: f64) {
        let new_sampling_rate = new_sampling_rate.max(1.0);
        
        // Пересчитываем нормализованные частоты с новой частотой дискретизации
        let old_nyquist = self.sampling_rate / 2.0;
        let new_nyquist = new_sampling_rate / 2.0;
        
        // Конвертируем текущие нормализованные частоты обратно в Hz, затем в новые нормализованные
        let cutoff_hz = self.cutoff_frequency * old_nyquist;
        let low_cutoff_hz = self.low_cutoff * old_nyquist;
        let high_cutoff_hz = self.high_cutoff * old_nyquist;
        
        self.sampling_rate = new_sampling_rate;
        self.cutoff_frequency = (cutoff_hz / new_nyquist).clamp(0.001, 0.499);
        self.low_cutoff = (low_cutoff_hz / new_nyquist).clamp(0.001, 0.499);
        self.high_cutoff = (high_cutoff_hz / new_nyquist).max(self.low_cutoff + 0.001).min(0.499);
        
        // Перепроектируем фильтр с новыми параметрами
        self.design_filter();
        self.compute_impulse_response();
        self.reset(); // Сбрасываем состояние
    }

    /// Получить импульсную характеристику фильтра
    pub fn get_impulse_response(&self) -> &[f64] {
        &self.filter_response
    }

    /// Получить частоту среза в Hz
    pub fn get_cutoff_frequency_hz(&self) -> f64 {
        self.cutoff_frequency * (self.sampling_rate / 2.0)
    }

    /// Установить частоту среза в Hz
    pub fn set_cutoff_frequency_hz(&mut self, cutoff_hz: f64) {
        let nyquist_freq = self.sampling_rate / 2.0;
        let normalized_cutoff = (cutoff_hz / nyquist_freq).clamp(0.001, 0.499);
        self.cutoff_frequency = normalized_cutoff;
        self.design_filter();
        self.compute_impulse_response();
    }

    /// Получить полосовые частоты в Hz
    pub fn get_band_frequencies_hz(&self) -> (f64, f64) {
        let nyquist_freq = self.sampling_rate / 2.0;
        (self.low_cutoff * nyquist_freq, self.high_cutoff * nyquist_freq)
    }

    /// Установить полосовые частоты в Hz
    pub fn set_band_frequencies_hz(&mut self, low_cutoff_hz: f64, high_cutoff_hz: f64) {
        let nyquist_freq = self.sampling_rate / 2.0;
        let normalized_low = (low_cutoff_hz / nyquist_freq).clamp(0.001, 0.499);
        let normalized_high = (high_cutoff_hz / nyquist_freq).max(normalized_low + 0.001).min(0.499);
        
        self.low_cutoff = normalized_low;
        self.high_cutoff = normalized_high;
        self.cutoff_frequency = (normalized_low + normalized_high) / 2.0;
        
        self.design_filter();
        self.compute_impulse_response();
    }

    /// Получить полную конфигурацию фильтра
    pub fn get_config(&self) -> ButterworthConfig {
        ButterworthConfig {
            filter_type: self.filter_type,
            order: self.order,
            cutoff_frequency_hz: self.get_cutoff_frequency_hz(),
            low_cutoff_hz: self.low_cutoff * (self.sampling_rate / 2.0),
            high_cutoff_hz: self.high_cutoff * (self.sampling_rate / 2.0),
            sampling_rate: self.sampling_rate,
        }
    }

    /// Установить новую конфигурацию фильтра
    pub fn set_config(&mut self, config: ButterworthConfig) {
        self.filter_type = config.filter_type;
        self.order = config.order.clamp(1, 8);
        self.sampling_rate = config.sampling_rate.max(1.0);
        
        let nyquist_freq = self.sampling_rate / 2.0;
        self.cutoff_frequency = (config.cutoff_frequency_hz / nyquist_freq).clamp(0.001, 0.499);
        self.low_cutoff = (config.low_cutoff_hz / nyquist_freq).clamp(0.001, 0.499);
        self.high_cutoff = (config.high_cutoff_hz / nyquist_freq).max(self.low_cutoff + 0.001).min(0.499);
        
        self.design_filter();
        self.compute_impulse_response();
        self.reset();
    }

    /// Анализ частотной характеристики фильтра
    pub fn frequency_response(&self, frequency_hz: f64) -> (f64, f64) {
        let omega = 2.0 * PI * frequency_hz / self.sampling_rate;
        let mut magnitude = 0.0;
        let mut phase = 0.0;
        
        if !self.a_coefficients.is_empty() && !self.b_coefficients.is_empty() {
            // Упрощенный расчет частотной характеристики
            let _cos_omega = omega.cos();
            let _sin_omega = omega.sin();
            
            let mut h_real = 0.0;
            let mut h_imag = 0.0;
            
            // Числитель (коэффициенты b)
            for (k, &b_k) in self.b_coefficients.iter().enumerate() {
                h_real += b_k * (k as f64 * omega).cos();
                h_imag -= b_k * (k as f64 * omega).sin();
            }
            
            // Знаменатель (коэффициенты a) - инвертируем
            let mut denom_real = 0.0;
            let mut denom_imag = 0.0;
            
            for (k, &a_k) in self.a_coefficients.iter().enumerate() {
                denom_real += a_k * (k as f64 * omega).cos();
                denom_imag -= a_k * (k as f64 * omega).sin();
            }
            
            // H(ω) = числитель / знаменатель
            let denom_mag_sq = denom_real * denom_real + denom_imag * denom_imag;
            if denom_mag_sq > 1e-12 {
                let final_real = (h_real * denom_real + h_imag * denom_imag) / denom_mag_sq;
                let final_imag = (h_imag * denom_real - h_real * denom_imag) / denom_mag_sq;
                
                magnitude = (final_real * final_real + final_imag * final_imag).sqrt();
                phase = final_imag.atan2(final_real);
            }
        }
        
        (magnitude, phase)
    }
}

/// Конфигурация Butterworth фильтра
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButterworthConfig {
    pub filter_type: FilterType,
    pub order: usize,
    pub cutoff_frequency_hz: f64,
    pub low_cutoff_hz: f64,
    pub high_cutoff_hz: f64,
    pub sampling_rate: f64,
}

impl std::fmt::Debug for ButterworthFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ButterworthFilter")
            .field("filter_type", &self.filter_type)
            .field("order", &self.order)
            .field("cutoff_frequency_hz", &self.get_cutoff_frequency_hz())
            .field("sampling_rate", &self.sampling_rate)
            .field("filtered_value", &self.filtered_value)
            .field("impulse_response_length", &self.filter_response.len())
            .field("is_initialized", &self.is_initialized)
            .field("a_coeffs_count", &self.a_coefficients.len())
            .field("b_coeffs_count", &self.b_coefficients.len())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_butterworth_creation() {
        let filter = ButterworthFilter::new(FilterType::LowPass, 2, 10.0, 100.0);
        assert!(!filter.is_ready());
        assert_eq!(filter.order(), 2);
        assert_eq!(filter.filter_type(), FilterType::LowPass);
    }

    #[test]
    fn test_butterworth_lowpass_smoothing() {
        let mut filter = ButterworthFilter::new(FilterType::LowPass, 2, 5.0, 100.0);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let filtered = filter.update(price);
            assert!(filtered.is_finite(), "Butterworth output should be finite");
        }
        assert!(filter.is_ready());
    }

    #[test]
    fn test_butterworth_highpass() {
        let mut filter = ButterworthFilter::new(FilterType::HighPass, 2, 20.0, 100.0);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            filter.update(price);
        }
        assert!(filter.is_ready());
        assert!(filter.value().main().is_finite());
    }

    #[test]
    fn test_butterworth_reset() {
        let mut filter = ButterworthFilter::new(FilterType::LowPass, 2, 10.0, 100.0);
        for i in 1..=20 {
            filter.update(100.0 + i as f64);
        }
        assert!(filter.is_ready());
        filter.reset();
        assert!(!filter.is_ready());
    }
}






















