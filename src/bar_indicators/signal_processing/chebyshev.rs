//! Chebyshev Filter
//! Фильтр Чебышева для обработки сигналов
//! Обеспечивает крутые переходы с контролируемой пульсацией в полосе пропускания

use arrayvec::ArrayVec;
use std::f64::consts::PI;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ChebyshevType {
    Type1,  // Чебышев I типа (пульсации в полосе пропускания)
    Type2,  // Чебышев II типа (пульсации в полосе задержания)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FilterType {
    LowPass,    // Низкочастотный фильтр
    HighPass,   // Высокочастотный фильтр  
    BandPass,   // Полосовой фильтр
    BandStop,   // Режекторный фильтр
}

/// Chebyshev Filter
#[derive(Clone)]
pub struct ChebyshevFilter {
    // Параметры фильтра
    chebyshev_type: ChebyshevType,
    filter_type: FilterType,
    order: usize,                           // Порядок фильтра (1-8)
    cutoff_frequency: f64,                  // Частота среза (нормализованная 0-0.5)
    low_cutoff: f64,                        // Нижняя частота среза (для полосовых)
    high_cutoff: f64,                       // Верхняя частота среза (для полосовых)
    ripple_db: f64,                         // Пульсации в дБ
    
    // Коэффициенты IIR фильтра
    a_coefficients: ArrayVec<f64, 16>,      // Коэффициенты знаменателя
    b_coefficients: ArrayVec<f64, 16>,      // Коэффициенты числителя
    
    // Буферы задержки
    x_buffer: ArrayVec<f64, 16>,            // Входные значения
    y_buffer: ArrayVec<f64, 16>,            // Выходные значения
    
    // Результаты
    filtered_value: f64,                    // Отфильтрованное значение
    group_delay: f64,                       // Групповая задержка
    phase_response: f64,                    // Фазовая характеристика
    
    // Каскады биквадратных секций для высоких порядков
    biquad_sections: ArrayVec<BiquadSection, 4>,
    
    // Состояние
    is_initialized: bool,
}

/// Биквадратная секция для каскадной реализации
#[derive(Debug, Clone)]
struct BiquadSection {
    // Коэффициенты секции
    b0: f64, b1: f64, b2: f64,
    a1: f64, a2: f64,
    
    // Память секции
    x1: f64, x2: f64,
    y1: f64, y2: f64,
}

impl BiquadSection {
    fn new() -> Self {
        Self {
            b0: 1.0, b1: 0.0, b2: 0.0,
            a1: 0.0, a2: 0.0,
            x1: 0.0, x2: 0.0,
            y1: 0.0, y2: 0.0,
        }
    }
    
    fn process(&mut self, input: f64) -> f64 {
        // Прямая форма II
        let w = input - self.a1 * self.y1 - self.a2 * self.y2;
        let output = self.b0 * w + self.b1 * self.y1 + self.b2 * self.y2;
        
        // Обновляем память
        self.y2 = self.y1;
        self.y1 = output;
        
        output
    }
    
    fn reset(&mut self) {
        self.x1 = 0.0; self.x2 = 0.0;
        self.y1 = 0.0; self.y2 = 0.0;
    }
}

impl ChebyshevFilter {
    pub fn new(
        chebyshev_type: ChebyshevType, 
        filter_type: FilterType, 
        order: usize, 
        cutoff_frequency: f64, 
        ripple_db: f64
    ) -> Self {
        let order = order.clamp(1, 8);
        let cutoff_frequency = cutoff_frequency.clamp(0.001, 0.499);
        let ripple_db = ripple_db.clamp(0.1, 60.0);
        
        let mut filter = Self {
            chebyshev_type,
            filter_type,
            order,
            cutoff_frequency,
            low_cutoff: 0.1,
            high_cutoff: 0.4,
            ripple_db,
            a_coefficients: ArrayVec::new(),
            b_coefficients: ArrayVec::new(),
            x_buffer: ArrayVec::new(),
            y_buffer: ArrayVec::new(),
            filtered_value: 0.0,
            group_delay: 0.0,
            phase_response: 0.0,
            biquad_sections: ArrayVec::new(),
            is_initialized: false,
        };
        
        filter.design_filter();
        filter
    }
    
    /// Создать полосовой или режекторный фильтр
    pub fn new_band_filter(
        chebyshev_type: ChebyshevType,
        filter_type: FilterType,
        order: usize,
        low_cutoff: f64,
        high_cutoff: f64,
        ripple_db: f64
    ) -> Self {
        let order = order.clamp(1, 8);
        let low_cutoff = low_cutoff.clamp(0.001, 0.499);
        let high_cutoff = high_cutoff.max(low_cutoff + 0.001).min(0.499);
        let ripple_db = ripple_db.clamp(0.1, 60.0);
        
        let mut filter = Self {
            chebyshev_type,
            filter_type,
            order,
            cutoff_frequency: (low_cutoff + high_cutoff) / 2.0,
            low_cutoff,
            high_cutoff,
            ripple_db,
            a_coefficients: ArrayVec::new(),
            b_coefficients: ArrayVec::new(),
            x_buffer: ArrayVec::new(),
            y_buffer: ArrayVec::new(),
            filtered_value: 0.0,
            group_delay: 0.0,
            phase_response: 0.0,
            biquad_sections: ArrayVec::new(),
            is_initialized: false,
        };
        
        filter.design_filter();
        filter
    }
    
    /// Обновить фильтр новым значением
    pub fn update(&mut self, value: f64) -> f64 {
        if !self.is_initialized {
            self.initialize_buffers(value);
            self.is_initialized = true;
            self.filtered_value = value;
            return value;
        }
        
        // Обработка через каскад биквадратных секций (для высоких порядков)
        if !self.biquad_sections.is_empty() {
            let mut output = value;
            for section in &mut self.biquad_sections {
                output = section.process(output);
            }
            self.filtered_value = output;
        } else {
            // Прямая IIR фильтрация для низких порядков
            self.filtered_value = self.direct_form_filter(value);
        }
        
        self.filtered_value
    }
    
    /// Инициализация буферов
    fn initialize_buffers(&mut self, initial_value: f64) {
        self.x_buffer.clear();
        self.y_buffer.clear();
        
        // Заполняем буферы начальным значением для устойчивости
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
        
        // Инициализируем биквадратные секции
        for section in &mut self.biquad_sections {
            section.reset();
        }
    }
    
    /// Прямая IIR фильтрация
    fn direct_form_filter(&mut self, input: f64) -> f64 {
        // Добавляем новое входное значение
        if self.x_buffer.len() > self.order {
            self.x_buffer.remove(0);
        }
        if !self.x_buffer.is_full() {
            self.x_buffer.push(input);
        }
        
        // Вычисляем выходное значение
        let mut output = 0.0;
        
        // Числитель (b коэффициенты)
        for (i, &x_val) in self.x_buffer.iter().enumerate() {
            if i < self.b_coefficients.len() {
                output += self.b_coefficients[i] * x_val;
            }
        }
        
        // Знаменатель (a коэффициенты, кроме a[0])
        for (i, &y_val) in self.y_buffer.iter().enumerate() {
            if i + 1 < self.a_coefficients.len() {
                output -= self.a_coefficients[i + 1] * y_val;
            }
        }
        
        // Нормализуем на a[0]
        if !self.a_coefficients.is_empty() && self.a_coefficients[0].abs() > 1e-12 {
            output /= self.a_coefficients[0];
        }
        
        // Добавляем выходное значение в буфер
        if self.y_buffer.len() >= self.order {
            self.y_buffer.remove(0);
        }
        if !self.y_buffer.is_full() {
            self.y_buffer.push(output);
        }
        
        output
    }
    
    /// Проектирование фильтра Чебышева
    fn design_filter(&mut self) {
        self.a_coefficients.clear();
        self.b_coefficients.clear();
        self.biquad_sections.clear();
        
        match self.filter_type {
            FilterType::LowPass => self.design_lowpass(),
            FilterType::HighPass => self.design_highpass(),
            FilterType::BandPass => self.design_bandpass(),
            FilterType::BandStop => self.design_bandstop(),
        }
        
        // Для порядков выше 4 используем каскад биквадратных секций
        if self.order > 4 {
            self.create_biquad_cascade();
        }
    }
    
    /// Проектирование низкочастотного фильтра
    fn design_lowpass(&mut self) {
        let _wp = 2.0 * self.cutoff_frequency; // Предварительно исказим частоту
        let wa = (2.0 * PI * self.cutoff_frequency / 2.0).tan(); // Билинейное преобразование
        
        match self.chebyshev_type {
            ChebyshevType::Type1 => {
                self.design_chebyshev_type1_lowpass(wa);
            },
            ChebyshevType::Type2 => {
                self.design_chebyshev_type2_lowpass(wa);
            },
        }
    }
    
    /// Чебышев I типа (пульсации в полосе пропускания)
    fn design_chebyshev_type1_lowpass(&mut self, wa: f64) {
        let epsilon = (10.0_f64.powf(self.ripple_db / 10.0) - 1.0).sqrt();
        let n = self.order as f64;
        
        // Полюса аналогового прототипа
        let mut poles = Vec::new();
        for k in 0..self.order {
            let angle = PI * (2.0 * k as f64 + 1.0) / (2.0 * n);
            let sigma = -wa * (1.0 / epsilon).asinh() / n * angle.sin();
            let omega = wa * angle.cos();
            poles.push((sigma, omega));
        }
        
        // Преобразуем в цифровые коэффициенты
        self.analog_to_digital_lowpass(&poles, wa);
    }
    
    /// Чебышев II типа (пульсации в полосе задержания)
    fn design_chebyshev_type2_lowpass(&mut self, wa: f64) {
        let epsilon = 1.0 / (10.0_f64.powf(self.ripple_db / 10.0) - 1.0).sqrt();
        let n = self.order as f64;
        
        // Полюса и нули аналогового прототипа
        let mut poles = Vec::new();
        for k in 0..self.order {
            let angle = PI * (2.0 * k as f64 + 1.0) / (2.0 * n);
            // Инвертированные полюса Чебышева I
            let sigma = -wa / ((1.0 / epsilon).asinh() / n * angle.sin());
            let omega = wa / angle.cos();
            poles.push((sigma, omega));
        }
        
        self.analog_to_digital_lowpass(&poles, wa);
    }
    
    /// Преобразование из аналогового в цифровой фильтр (билинейное преобразование)
    fn analog_to_digital_lowpass(&mut self, poles: &[(f64, f64)], wa: f64) {
        // Для простоты реализуем только первые два порядка напрямую
        match self.order {
            1 => {
                let (sigma, _) = poles[0];
                let alpha = wa / (wa - sigma);
                self.b_coefficients.push(alpha);
                self.b_coefficients.push(alpha);
                self.a_coefficients.push(1.0);
                self.a_coefficients.push(alpha - 1.0);
            },
            
            2 => {
                // Биквадратная секция
                let (s1, w1) = poles[0];
                let (_s2, _w2) = if poles.len() > 1 { poles[1] } else { poles[0] };
                
                let wn = (s1 * s1 + w1 * w1).sqrt();
                let q = wn / (-2.0 * s1);
                let k = wa / wn;
                let k2 = k * k;
                let k_q = k / q;
                let norm = 1.0 + k_q + k2;
                
                // Коэффициенты числителя
                self.b_coefficients.push(k2 / norm);
                self.b_coefficients.push(2.0 * k2 / norm);
                self.b_coefficients.push(k2 / norm);
                
                // Коэффициенты знаменателя
                self.a_coefficients.push(1.0);
                self.a_coefficients.push(2.0 * (k2 - 1.0) / norm);
                self.a_coefficients.push((1.0 - k_q + k2) / norm);
            },
            
            _ => {
                // Для высоких порядков используем упрощенную аппроксимацию
                let alpha = wa / (wa + 1.0);
                for i in 0..=self.order {
                    let coeff = if i == 0 || i == self.order { alpha } else { 2.0 * alpha };
                    if !self.b_coefficients.is_full() {
                        self.b_coefficients.push(coeff);
                    }
                }
                
                self.a_coefficients.push(1.0);
                for i in 1..=self.order {
                    let coeff = if i % 2 == 1 { alpha - 1.0 } else { 1.0 - alpha };
                    if !self.a_coefficients.is_full() {
                        self.a_coefficients.push(coeff);
                    }
                }
            }
        }
    }
    
    /// Проектирование высокочастотного фильтра
    fn design_highpass(&mut self) {
        // Сначала проектируем НЧ фильтр, затем применяем высокочастотное преобразование
        self.design_lowpass();
        self.transform_lowpass_to_highpass();
    }
    
    /// Преобразование НЧ в ВЧ фильтр
    fn transform_lowpass_to_highpass(&mut self) {
        // Замена z -> -z^(-1) в передаточной функции
        for i in 0..self.b_coefficients.len() {
            if i % 2 == 1 {
                self.b_coefficients[i] = -self.b_coefficients[i];
            }
        }
        
        for i in 0..self.a_coefficients.len() {
            if i % 2 == 1 {
                self.a_coefficients[i] = -self.a_coefficients[i];
            }
        }
        
        // Реверсируем коэффициенты
        self.b_coefficients.reverse();
    }
    
    /// Проектирование полосового фильтра (упрощенная версия)
    fn design_bandpass(&mut self) {
        let center_freq = (self.low_cutoff + self.high_cutoff) / 2.0;
        let bandwidth = self.high_cutoff - self.low_cutoff;
        
        // Используем упрощенную биквадратную секцию для полосового фильтра
        let w0 = 2.0 * PI * center_freq;
        let q = center_freq / bandwidth;
        let alpha = w0.sin() / (2.0 * q);
        
        let norm = 1.0 + alpha;
        
        self.b_coefficients.push(alpha / norm);
        self.b_coefficients.push(0.0);
        self.b_coefficients.push(-alpha / norm);
        
        self.a_coefficients.push(1.0);
        self.a_coefficients.push(-2.0 * w0.cos() / norm);
        self.a_coefficients.push((1.0 - alpha) / norm);
    }
    
    /// Проектирование режекторного фильтра
    fn design_bandstop(&mut self) {
        let center_freq = (self.low_cutoff + self.high_cutoff) / 2.0;
        let bandwidth = self.high_cutoff - self.low_cutoff;
        
        let w0 = 2.0 * PI * center_freq;
        let q = center_freq / bandwidth;
        let alpha = w0.sin() / (2.0 * q);
        
        let norm = 1.0 + alpha;
        let cos_w0 = w0.cos();
        
        self.b_coefficients.push(1.0 / norm);
        self.b_coefficients.push(-2.0 * cos_w0 / norm);
        self.b_coefficients.push(1.0 / norm);
        
        self.a_coefficients.push(1.0);
        self.a_coefficients.push(-2.0 * cos_w0 / norm);
        self.a_coefficients.push((1.0 - alpha) / norm);
    }
    
    /// Создание каскада биквадратных секций для высоких порядков
    fn create_biquad_cascade(&mut self) {
        let sections = self.order.div_ceil(2); // Количество биквадратных секций
        
        for i in 0..sections {
            let mut section = BiquadSection::new();
            
            // Упрощенное распределение коэффициентов по секциям
            let section_index = i * 3;
            if section_index < self.b_coefficients.len() {
                section.b0 = self.b_coefficients[section_index];
            }
            if section_index + 1 < self.b_coefficients.len() {
                section.b1 = self.b_coefficients[section_index + 1];
            }
            if section_index + 2 < self.b_coefficients.len() {
                section.b2 = self.b_coefficients[section_index + 2];
            }
            
            if section_index + 1 < self.a_coefficients.len() {
                section.a1 = self.a_coefficients[section_index + 1];
            }
            if section_index + 2 < self.a_coefficients.len() {
                section.a2 = self.a_coefficients[section_index + 2];
            }
            
            if !self.biquad_sections.is_full() {
                self.biquad_sections.push(section);
            }
        }
    }
    
    // Публичные методы
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.filtered_value)
    }
    
    pub fn chebyshev_type(&self) -> ChebyshevType {
        self.chebyshev_type
    }
    
    pub fn filter_type(&self) -> FilterType {
        self.filter_type
    }
    
    pub fn order(&self) -> usize {
        self.order
    }
    
    pub fn cutoff_frequency(&self) -> f64 {
        self.cutoff_frequency
    }
    
    pub fn ripple_db(&self) -> f64 {
        self.ripple_db
    }
    
    pub fn group_delay(&self) -> f64 {
        self.group_delay
    }
    
    pub fn phase_response(&self) -> f64 {
        self.phase_response
    }
    
    pub fn coefficients(&self) -> (&[f64], &[f64]) {
        (&self.b_coefficients, &self.a_coefficients)
    }
    
    pub fn set_cutoff_frequency(&mut self, cutoff: f64) {
        self.cutoff_frequency = cutoff.clamp(0.001, 0.499);
        self.design_filter();
        self.reset();
    }
    
    pub fn set_ripple(&mut self, ripple_db: f64) {
        self.ripple_db = ripple_db.clamp(0.1, 60.0);
        self.design_filter();
        self.reset();
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_initialized
    }
    
    pub fn reset(&mut self) {
        self.x_buffer.clear();
        self.y_buffer.clear();
        self.filtered_value = 0.0;
        self.is_initialized = false;

        for section in &mut self.biquad_sections {
            section.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chebyshev_creation() {
        let filter = ChebyshevFilter::new(ChebyshevType::Type1, FilterType::LowPass, 2, 0.2, 1.0);
        assert!(!filter.is_ready());
        assert_eq!(filter.order(), 2);
        assert_eq!(filter.chebyshev_type(), ChebyshevType::Type1);
    }

    #[test]
    fn test_chebyshev_lowpass() {
        let mut filter = ChebyshevFilter::new(ChebyshevType::Type1, FilterType::LowPass, 2, 0.2, 1.0);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let filtered = filter.update(price);
            assert!(filtered.is_finite(), "Chebyshev output should be finite");
        }
        assert!(filter.is_ready());
    }

    #[test]
    fn test_chebyshev_type2() {
        let mut filter = ChebyshevFilter::new(ChebyshevType::Type2, FilterType::LowPass, 2, 0.3, 3.0);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            filter.update(price);
        }
        assert!(filter.is_ready());
        assert!(filter.value().main().is_finite());
    }

    #[test]
    fn test_chebyshev_reset() {
        let mut filter = ChebyshevFilter::new(ChebyshevType::Type1, FilterType::LowPass, 2, 0.2, 1.0);
        for i in 1..=20 {
            filter.update(100.0 + i as f64);
        }
        assert!(filter.is_ready());
        filter.reset();
        assert!(!filter.is_ready());
    }
}






















