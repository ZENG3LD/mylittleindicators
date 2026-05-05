//! Savitzky-Golay Filter
//! Фильтр Савицкого-Голея для сглаживания и вычисления производных
//! Использует локальную полиномиальную аппроксимацию для сохранения особенностей сигнала

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DerivativeOrder {
    Smoothing,       // Только сглаживание (0-я производная)
    FirstDerivative, // Первая производная
    SecondDerivative, // Вторая производная
    ThirdDerivative, // Третья производная
}

/// Savitzky-Golay Filter
#[derive(Clone)]
pub struct SavitzkyGolayFilter {
    // Параметры фильтра
    window_size: usize,                         // Размер окна (должен быть нечетным)
    polynomial_order: usize,                    // Порядок полинома
    derivative_order: DerivativeOrder,          // Порядок производной
    
    // Данные
    values: ArrayVec<f64, 63>,                  // Окно данных (максимум 63 для полинома 6-го порядка)
    
    // Коэффициенты
    coefficients: ArrayVec<f64, 63>,            // Коэффициенты фильтра
    
    // Результаты
    filtered_value: f64,                        // Отфильтрованное значение
    confidence: f64,                            // Доверительный интервал
    polynomial_fit: ArrayVec<f64, 7>,           // Коэффициенты полинома
    
    // Статистики качества
    residual_sum_squares: f64,                  // Сумма квадратов остатков
    correlation_coefficient: f64,               // Коэффициент корреляции
    
    // Состояние
    is_ready: bool,
    half_window: usize,
}

impl SavitzkyGolayFilter {
    pub fn new(window_size: usize, polynomial_order: usize, derivative_order: DerivativeOrder) -> Self {
        // Проверяем и корректируем параметры
        let window_size = Self::validate_window_size(window_size);
        let polynomial_order = Self::validate_polynomial_order(polynomial_order, window_size);
        let half_window = window_size / 2;
        
        let mut filter = Self {
            window_size,
            polynomial_order,
            derivative_order,
            values: ArrayVec::new(),
            coefficients: ArrayVec::new(),
            filtered_value: 0.0,
            confidence: 0.0,
            polynomial_fit: ArrayVec::new(),
            residual_sum_squares: 0.0,
            correlation_coefficient: 0.0,
            is_ready: false,
            half_window,
        };
        
        filter.calculate_coefficients();
        filter
    }
    
    /// Обновить фильтр новым значением
    pub fn update(&mut self, value: f64) -> f64 {
        // Добавляем новое значение
        if self.values.len() >= self.window_size {
            self.values.remove(0);
        }
        if !self.values.is_full() {
            self.values.push(value);
        }
        
        if self.values.len() == self.window_size {
            self.filtered_value = self.apply_filter();
            self.calculate_statistics();
            self.is_ready = true;
        } else {
            self.filtered_value = value;
        }
        
        self.filtered_value
    }
    
    /// Проверка размера окна
    fn validate_window_size(size: usize) -> usize {
        let size = if size.is_multiple_of(2) { size + 1 } else { size }; // Должен быть нечетным
        size.clamp(5, 63) // От 5 до 63
    }
    
    /// Проверка порядка полинома
    fn validate_polynomial_order(order: usize, window_size: usize) -> usize {
        order.clamp(1, 6).min(window_size - 1) // Порядок должен быть меньше размера окна
    }
    
    /// Вычисление коэффициентов Савицкого-Голея
    fn calculate_coefficients(&mut self) {
        self.coefficients.clear();
        
        let n = self.window_size;
        let m = self.half_window;
        let degree = self.polynomial_order;
        
        // Создаем матрицу Вандермонда
        let mut matrix = vec![vec![0.0; degree + 1]; n];
        let mut rhs = vec![0.0; degree + 1];
        
        // Заполняем матрицу
        for (i, row) in matrix.iter_mut().enumerate() {
            let x = (i as f64) - (m as f64); // Центрируем относительно середины окна
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = x.powi(j as i32);
            }
        }
        
        // Создаем правую часть для нужной производной
        match self.derivative_order {
            DerivativeOrder::Smoothing => rhs[0] = 1.0,
            DerivativeOrder::FirstDerivative => {
                if degree >= 1 { rhs[1] = 1.0; }
            },
            DerivativeOrder::SecondDerivative => {
                if degree >= 2 { rhs[2] = 2.0; } // 2! = 2
            },
            DerivativeOrder::ThirdDerivative => {
                if degree >= 3 { rhs[3] = 6.0; } // 3! = 6
            },
        }
        
        // Решаем систему нормальных уравнений: (A^T * A) * c = A^T * rhs
        let coeffs = self.solve_normal_equations(&matrix, &rhs);
        
        // Вычисляем коэффициенты фильтра
        for i in 0..n {
            let mut coeff = 0.0;
            for j in 0..=degree {
                if j < coeffs.len() {
                    let x = (i as f64) - (m as f64);
                    coeff += coeffs[j] * x.powi(j as i32);
                }
            }
            if !self.coefficients.is_full() {
                self.coefficients.push(coeff);
            }
        }
        
        // Сохраняем коэффициенты полинома для анализа
        self.polynomial_fit.clear();
        for &coeff in &coeffs {
            if !self.polynomial_fit.is_full() {
                self.polynomial_fit.push(coeff);
            }
        }
    }
    
    /// Решение системы нормальных уравнений методом Гаусса
    fn solve_normal_equations(&self, matrix: &[Vec<f64>], rhs: &[f64]) -> Vec<f64> {
        let n = matrix.len();
        let p = matrix[0].len();
        
        // Вычисляем A^T * A
        let mut ata = vec![vec![0.0; p]; p];
        for i in 0..p {
            for j in 0..p {
                for row in &matrix[..n] {
                    ata[i][j] += row[i] * row[j];
                }
            }
        }

        // Вычисляем A^T * rhs
        let mut atr = vec![0.0; p];
        for i in 0..p {
            for row in &matrix[..n] {
                atr[i] += row[i] * rhs[i];
            }
        }
        
        // Решаем систему методом Гаусса
        self.gaussian_elimination(&ata, &atr)
    }
    
    /// Метод Гаусса для решения системы линейных уравнений
    fn gaussian_elimination(&self, matrix: &[Vec<f64>], rhs: &[f64]) -> Vec<f64> {
        let n = matrix.len();
        let mut a = matrix.to_vec();
        let mut b = rhs.to_vec();
        
        // Прямой ход
        for i in 0..n {
            // Поиск главного элемента
            let mut max_row = i;
            for k in (i + 1)..n {
                if a[k][i].abs() > a[max_row][i].abs() {
                    max_row = k;
                }
            }
            
            // Перестановка строк
            if max_row != i {
                a.swap(i, max_row);
                b.swap(i, max_row);
            }
            
            // Исключение
            for k in (i + 1)..n {
                if a[i][i].abs() > 1e-12 {
                    let factor = a[k][i] / a[i][i];
                    // i < k always holds here, so split_at_mut is safe
                    let (left, right) = a.split_at_mut(k);
                    for (akj, &aij) in right[0][i..].iter_mut().zip(left[i][i..].iter()) {
                        *akj -= factor * aij;
                    }
                    b[k] -= factor * b[i];
                }
            }
        }
        
        // Обратный ход
        let mut x = vec![0.0; n];
        for i in (0..n).rev() {
            x[i] = b[i];
            for j in (i + 1)..n {
                x[i] -= a[i][j] * x[j];
            }
            if a[i][i].abs() > 1e-12 {
                x[i] /= a[i][i];
            }
        }
        
        x
    }
    
    /// Применение фильтра
    fn apply_filter(&self) -> f64 {
        if self.values.len() != self.coefficients.len() {
            return self.values.last().copied().unwrap_or(0.0);
        }
        
        let mut result = 0.0;
        for (i, &value) in self.values.iter().enumerate() {
            if i < self.coefficients.len() {
                result += value * self.coefficients[i];
            }
        }
        
        result
    }
    
    /// Вычисление статистик качества фильтрации
    fn calculate_statistics(&mut self) {
        if self.values.len() != self.window_size || self.polynomial_fit.is_empty() {
            return;
        }
        
        let mut sum_squares = 0.0;
        let mut sum_values = 0.0;
        let mut sum_fitted = 0.0;
        let mut sum_values_sq = 0.0;
        let mut sum_fitted_sq = 0.0;
        let mut sum_cross = 0.0;
        
        // Вычисляем подогнанные значения и остатки
        for (i, &value) in self.values.iter().enumerate() {
            let x = (i as f64) - (self.half_window as f64);
            let mut fitted = 0.0;
            
            for (j, &coeff) in self.polynomial_fit.iter().enumerate() {
                fitted += coeff * x.powi(j as i32);
            }
            
            let residual = value - fitted;
            sum_squares += residual * residual;
            
            sum_values += value;
            sum_fitted += fitted;
            sum_values_sq += value * value;
            sum_fitted_sq += fitted * fitted;
            sum_cross += value * fitted;
        }
        
        self.residual_sum_squares = sum_squares;
        
        // Коэффициент корреляции
        let n = self.values.len() as f64;
        let numerator = n * sum_cross - sum_values * sum_fitted;
        let denominator = ((n * sum_values_sq - sum_values * sum_values) * 
                          (n * sum_fitted_sq - sum_fitted * sum_fitted)).sqrt();
        
        self.correlation_coefficient = if denominator.abs() > 1e-12 {
            numerator / denominator
        } else {
            0.0
        };
        
        // Доверительный интервал (упрощенная оценка)
        let mse = sum_squares / (n - (self.polynomial_order + 1) as f64).max(1.0);
        self.confidence = mse.sqrt();
    }
    
    // Публичные методы
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.filtered_value)
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }
    
    pub fn window_size(&self) -> usize {
        self.window_size
    }
    
    pub fn polynomial_order(&self) -> usize {
        self.polynomial_order
    }
    
    pub fn derivative_order(&self) -> DerivativeOrder {
        self.derivative_order
    }
    
    pub fn confidence_interval(&self) -> f64 {
        self.confidence
    }
    
    pub fn correlation_coefficient(&self) -> f64 {
        self.correlation_coefficient
    }
    
    pub fn residual_sum_squares(&self) -> f64 {
        self.residual_sum_squares
    }
    
    pub fn polynomial_coefficients(&self) -> &[f64] {
        &self.polynomial_fit
    }
    
    pub fn filter_coefficients(&self) -> &[f64] {
        &self.coefficients
    }
    
    pub fn reset(&mut self) {
        self.values.clear();
        self.filtered_value = 0.0;
        self.confidence = 0.0;
        self.residual_sum_squares = 0.0;
        self.correlation_coefficient = 0.0;
        self.is_ready = false;
    }
    
    /// Изменить параметры фильтра
    pub fn reconfigure(&mut self, window_size: usize, polynomial_order: usize, derivative_order: DerivativeOrder) {
        self.window_size = Self::validate_window_size(window_size);
        self.polynomial_order = Self::validate_polynomial_order(polynomial_order, self.window_size);
        self.derivative_order = derivative_order;
        self.half_window = self.window_size / 2;

        self.calculate_coefficients();
        self.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_savitzky_golay_creation() {
        let sg = SavitzkyGolayFilter::new(11, 3, DerivativeOrder::Smoothing);
        assert!(!sg.is_ready());
        assert_eq!(sg.value().main(), 0.0);
        assert_eq!(sg.window_size(), 11);
        assert_eq!(sg.polynomial_order(), 3);
        assert_eq!(sg.derivative_order(), DerivativeOrder::Smoothing);
    }

    #[test]
    fn test_savitzky_golay_warmup() {
        let mut sg = SavitzkyGolayFilter::new(11, 3, DerivativeOrder::Smoothing);
        for i in 0..11 {
            sg.update(100.0 + i as f64);
        }
        assert!(sg.is_ready());
    }

    #[test]
    fn test_savitzky_golay_finite() {
        let mut sg = SavitzkyGolayFilter::new(5, 2, DerivativeOrder::Smoothing);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 5.0;
            let value = sg.update(price);
            assert!(value.is_finite(), "SG filter should always return finite values");
        }
    }

    #[test]
    fn test_savitzky_golay_reset() {
        let mut sg = SavitzkyGolayFilter::new(11, 3, DerivativeOrder::Smoothing);
        for i in 0..20 {
            sg.update(100.0 + i as f64);
        }
        sg.reset();
        assert!(!sg.is_ready());
        assert_eq!(sg.value().main(), 0.0);
    }
} 






















