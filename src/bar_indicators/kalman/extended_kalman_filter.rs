//! Extended Kalman Filter (EKF)
//! Расширенный фильтр Калмана для нелинейных систем
//! Использует линеаризацию через якобианы для обработки нелинейностей

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;


/// Нелинейная функция состояния
pub trait StateFunction: Send + Sync {
    fn apply(&self, state: &[f64; 2], dt: f64) -> [f64; 2];
    fn jacobian(&self, state: &[f64; 2], dt: f64) -> [[f64; 2]; 2];
    fn clone_box(&self) -> Box<dyn StateFunction>;
}

/// Нелинейная функция наблюдения
pub trait ObservationFunction: Send + Sync {
    fn apply(&self, state: &[f64; 2]) -> f64;
    fn jacobian(&self, state: &[f64; 2]) -> [f64; 2];
    fn clone_box(&self) -> Box<dyn ObservationFunction>;
}

/// Модель постоянной скорости с нелинейными эффектами
#[derive(Clone)]
pub struct NonlinearMotionModel {
    pub friction_coefficient: f64,  // Коэффициент трения
    pub acceleration_noise: f64,    // Шум ускорения
}

impl StateFunction for NonlinearMotionModel {
    fn clone_box(&self) -> Box<dyn StateFunction> {
        Box::new(self.clone())
    }

    fn apply(&self, state: &[f64; 2], dt: f64) -> [f64; 2] {
        let position = state[0];
        let velocity = state[1];
        
        // Нелинейная модель с трением: v' = v * (1 - friction * |v|)
        let friction_effect = 1.0 - self.friction_coefficient * velocity.abs();
        let new_velocity = velocity * friction_effect.max(0.1);
        let new_position = position + new_velocity * dt;
        
        [new_position, new_velocity]
    }
    
    fn jacobian(&self, state: &[f64; 2], dt: f64) -> [[f64; 2]; 2] {
        let velocity = state[1];
        
        // ∂f/∂position
        let df_dpos = [1.0, 0.0];
        
        // ∂f/∂velocity (учитываем нелинейность трения)
        let friction_deriv = if velocity >= 0.0 {
            1.0 - 2.0 * self.friction_coefficient * velocity
        } else {
            1.0 + 2.0 * self.friction_coefficient * velocity
        };
        
        let df_dvel = [dt * friction_deriv, friction_deriv];
        
        [df_dpos, df_dvel]
    }
}

/// Нелинейная модель наблюдения (например, с логарифмическим преобразованием)
#[derive(Clone)]
pub struct NonlinearObservationModel {
    pub observation_type: ObservationType,
}

#[derive(Debug, Clone, Copy)]
pub enum ObservationType {
    Linear,         // Прямое наблюдение
    Logarithmic,    // Логарифмическое преобразование
    Square,         // Квадратичное преобразование  
    Sigmoid,        // Сигмоидальное преобразование
}

impl ObservationFunction for NonlinearObservationModel {
    fn clone_box(&self) -> Box<dyn ObservationFunction> {
        Box::new(self.clone())
    }

    fn apply(&self, state: &[f64; 2]) -> f64 {
        let position = state[0];
        
        match self.observation_type {
            ObservationType::Linear => position,
            ObservationType::Logarithmic => {
                if position > 0.0 {
                    position.ln()
                } else {
                    -(-position).ln()
                }
            },
            ObservationType::Square => position * position,
            ObservationType::Sigmoid => {
                1.0 / (1.0 + (-position).exp())
            },
        }
    }
    
    fn jacobian(&self, state: &[f64; 2]) -> [f64; 2] {
        let position = state[0];
        
        let h_pos = match self.observation_type {
            ObservationType::Linear => 1.0,
            ObservationType::Logarithmic => {
                if position.abs() > 1e-12 {
                    1.0 / position
                } else {
                    1e12
                }
            },
            ObservationType::Square => 2.0 * position,
            ObservationType::Sigmoid => {
                let exp_neg_pos = (-position).exp();
                exp_neg_pos / (1.0 + exp_neg_pos).powi(2)
            },
        };
        
        [h_pos, 0.0] // Наблюдаем только позицию
    }
}

/// Матрица 2x2 для EKF вычислений
#[derive(Debug, Clone, Copy)]
pub struct Matrix2x2 {
    pub data: [[f64; 2]; 2],
}

impl Matrix2x2 {
    pub fn new(data: [[f64; 2]; 2]) -> Self {
        Self { data }
    }
    
    pub fn identity() -> Self {
        Self {
            data: [[1.0, 0.0], [0.0, 1.0]]
        }
    }
    
    pub fn from_jacobian(jacobian: [[f64; 2]; 2]) -> Self {
        Self { data: jacobian }
    }
    
    pub fn multiply(&self, other: &Matrix2x2) -> Matrix2x2 {
        let mut result = [[0.0; 2]; 2];
        for (i, row) in result.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                for k in 0..2 {
                    *cell += self.data[i][k] * other.data[k][j];
                }
            }
        }
        Matrix2x2::new(result)
    }
    
    pub fn transpose(&self) -> Matrix2x2 {
        Matrix2x2::new([
            [self.data[0][0], self.data[1][0]],
            [self.data[0][1], self.data[1][1]]
        ])
    }
    
    pub fn inverse(&self) -> Option<Matrix2x2> {
        let det = self.data[0][0] * self.data[1][1] - self.data[0][1] * self.data[1][0];
        if det.abs() < 1e-12 {
            return None;
        }
        
        Some(Matrix2x2::new([
            [self.data[1][1] / det, -self.data[0][1] / det],
            [-self.data[1][0] / det, self.data[0][0] / det]
        ]))
    }
    
    pub fn add(&self, other: &Matrix2x2) -> Matrix2x2 {
        Matrix2x2::new([
            [self.data[0][0] + other.data[0][0], self.data[0][1] + other.data[0][1]],
            [self.data[1][0] + other.data[1][0], self.data[1][1] + other.data[1][1]]
        ])
    }
    
    pub fn subtract(&self, other: &Matrix2x2) -> Matrix2x2 {
        Matrix2x2::new([
            [self.data[0][0] - other.data[0][0], self.data[0][1] - other.data[0][1]],
            [self.data[1][0] - other.data[1][0], self.data[1][1] - other.data[1][1]]
        ])
    }
    
    pub fn multiply_vector(&self, vector: &[f64; 2]) -> [f64; 2] {
        [
            self.data[0][0] * vector[0] + self.data[0][1] * vector[1],
            self.data[1][0] * vector[0] + self.data[1][1] * vector[1]
        ]
    }
    
    pub fn scale(&self, scalar: f64) -> Matrix2x2 {
        Matrix2x2::new([
            [self.data[0][0] * scalar, self.data[0][1] * scalar],
            [self.data[1][0] * scalar, self.data[1][1] * scalar]
        ])
    }
}

/// Результат EKF фильтрации
#[derive(Debug, Clone)]
pub struct EkfResult {
    pub filtered_value: f64,        // Отфильтрованное значение
    pub predicted_value: f64,       // Предсказанное значение
    pub velocity: f64,              // Скорость
    pub uncertainty: f64,           // Неопределенность
    pub innovation: f64,            // Инновация
    pub linearization_error: f64,   // Ошибка линеаризации
    pub condition_number: f64,      // Число обусловленности
    pub nonlinearity_measure: f64,  // Мера нелинейности
}

impl Default for EkfResult {
    fn default() -> Self {
        Self::new()
    }
}

impl EkfResult {
    pub fn new() -> Self {
        Self {
            filtered_value: 0.0,
            predicted_value: 0.0,
            velocity: 0.0,
            uncertainty: 0.0,
            innovation: 0.0,
            linearization_error: 0.0,
            condition_number: 1.0,
            nonlinearity_measure: 0.0,
        }
    }
}

/// Extended Kalman Filter
pub struct ExtendedKalmanFilter {
    // Модели системы
    state_function: Box<dyn StateFunction>,
    observation_function: Box<dyn ObservationFunction>,
    
    // Параметры фильтра
    dt: f64,
    process_noise: Matrix2x2,
    measurement_noise: f64,
    
    // Состояние фильтра
    state: [f64; 2],                    // [position, velocity]
    covariance: Matrix2x2,              // Ковариационная матрица
    
    // Результаты
    current_result: EkfResult,
    
    // История для анализа
    state_history: ArrayVec<[f64; 2], 100>,
    innovation_history: ArrayVec<f64, 100>,
    linearization_errors: ArrayVec<f64, 50>,
    
    // Адаптивность
    adaptive_noise: bool,
    #[allow(dead_code)]
    innovation_variance: f64,
    
    // Диагностика
    jacobian_condition_numbers: ArrayVec<f64, 50>,
    nonlinearity_measures: ArrayVec<f64, 50>,
    
    // Состояние
    is_initialized: bool,
}

impl ExtendedKalmanFilter {
    pub fn new(
        dt: f64,
        process_noise_std: f64,
        measurement_noise_std: f64,
        friction_coefficient: f64,
        observation_type: ObservationType
    ) -> Self {
        let state_function = Box::new(NonlinearMotionModel {
            friction_coefficient,
            acceleration_noise: process_noise_std,
        });
        
        let observation_function = Box::new(NonlinearObservationModel {
            observation_type,
        });
        
        // Матрица шума процесса
        let q = process_noise_std * process_noise_std;
        let process_noise = Matrix2x2::new([
            [q * dt * dt * dt / 3.0, q * dt * dt / 2.0],
            [q * dt * dt / 2.0, q * dt]
        ]);
        
        Self {
            state_function,
            observation_function,
            dt,
            process_noise,
            measurement_noise: measurement_noise_std * measurement_noise_std,
            state: [0.0, 0.0],
            covariance: Matrix2x2::new([[1000.0, 0.0], [0.0, 100.0]]),
            current_result: EkfResult::new(),
            state_history: ArrayVec::new(),
            innovation_history: ArrayVec::new(),
            linearization_errors: ArrayVec::new(),
            adaptive_noise: false,
            innovation_variance: 0.0,
            jacobian_condition_numbers: ArrayVec::new(),
            nonlinearity_measures: ArrayVec::new(),
            is_initialized: false,
        }
    }
    
    /// Включить адаптивную настройку шума
    pub fn enable_adaptive_noise(&mut self) {
        self.adaptive_noise = true;
    }
    
    /// Обновить фильтр новым наблюдением
    pub fn update(&mut self, observation: f64) -> &EkfResult {
        if !self.is_initialized {
            self.initialize(observation);
            self.is_initialized = true;
            self.current_result.filtered_value = observation;
            return &self.current_result;
        }
        
        // Шаг предсказания
        self.predict();
        
        // Шаг коррекции
        self.correct(observation);
        
        // Диагностика нелинейности
        self.analyze_nonlinearity();
        
        // Адаптация (если включена)
        if self.adaptive_noise {
            self.adapt_noise();
        }
        
        // Сохранение истории
        self.save_history();
        
        &self.current_result
    }
    
    /// Инициализация фильтра
    fn initialize(&mut self, observation: f64) {
        // Инвертируем функцию наблюдения для получения начального состояния
        // Инвертируем функцию наблюдения для получения начального состояния
        // Используем простое линейное предположение для инициализации
        self.state[0] = observation;
        self.state[1] = 0.0; // Начальная скорость
        
        self.covariance = Matrix2x2::new([
            [1000.0, 0.0],
            [0.0, 100.0]
        ]);
    }
    
    /// Шаг предсказания
    fn predict(&mut self) {
        // Нелинейное предсказание состояния
        let predicted_state = self.state_function.apply(&self.state, self.dt);
        
        // Якобиан функции состояния
        let f_jacobian = Matrix2x2::from_jacobian(
            self.state_function.jacobian(&self.state, self.dt)
        );
        
        // Предсказание ковариации: P = F * P * F^T + Q
        let predicted_covariance = f_jacobian
            .multiply(&self.covariance)
            .multiply(&f_jacobian.transpose())
            .add(&self.process_noise);
        
        // Обновляем состояние
        self.state = predicted_state;
        self.covariance = predicted_covariance;
        
        // Сохраняем предсказанное значение
        self.current_result.predicted_value = self.observation_function.apply(&self.state);
        self.current_result.velocity = self.state[1];
    }
    
    /// Шаг коррекции
    fn correct(&mut self, observation: f64) {
        // Предсказанное наблюдение
        let predicted_observation = self.observation_function.apply(&self.state);
        let innovation = observation - predicted_observation;
        
        // Якобиан функции наблюдения
        let h_jacobian = self.observation_function.jacobian(&self.state);
        
        // Ковариация инновации: S = H * P * H^T + R
        let h_p = [
            h_jacobian[0] * self.covariance.data[0][0] + h_jacobian[1] * self.covariance.data[1][0],
            h_jacobian[0] * self.covariance.data[0][1] + h_jacobian[1] * self.covariance.data[1][1]
        ];
        
        let innovation_variance = h_jacobian[0] * h_p[0] + h_jacobian[1] * h_p[1] + self.measurement_noise;
        
        if innovation_variance > 1e-12 {
            // Коэффициент усиления Калмана: K = P * H^T / S
            let kalman_gain = [
                h_p[0] / innovation_variance,
                h_p[1] / innovation_variance
            ];
            
            // Коррекция состояния: x = x + K * innovation
            self.state[0] += kalman_gain[0] * innovation;
            self.state[1] += kalman_gain[1] * innovation;
            
            // Коррекция ковариации: P = (I - K * H) * P
            let kh = Matrix2x2::new([
                [kalman_gain[0] * h_jacobian[0], kalman_gain[0] * h_jacobian[1]],
                [kalman_gain[1] * h_jacobian[0], kalman_gain[1] * h_jacobian[1]]
            ]);
            
            let identity = Matrix2x2::identity();
            self.covariance = identity.subtract(&kh).multiply(&self.covariance);
            
            // Сохраняем результаты
            self.current_result.filtered_value = self.observation_function.apply(&self.state);
            self.current_result.innovation = innovation;
            self.current_result.uncertainty = self.covariance.data[0][0].sqrt();
            
            // Вычисляем ошибку линеаризации
            self.estimate_linearization_error(observation);
        }
    }
    
    /// Оценка ошибки линеаризации
    fn estimate_linearization_error(&mut self, _observation: f64) {
        // Сравниваем линейную аппроксимацию с истинной нелинейной функцией
        let h_jacobian = self.observation_function.jacobian(&self.state);
        let linear_prediction = self.observation_function.apply(&self.state) +
            h_jacobian[0] * 0.1 + h_jacobian[1] * 0.1; // Малое возмущение
        
        let true_prediction = {
            let perturbed_state = [self.state[0] + 0.1, self.state[1] + 0.1];
            self.observation_function.apply(&perturbed_state)
        };
        
        let linearization_error = (linear_prediction - true_prediction).abs();
        self.current_result.linearization_error = linearization_error;
        
        // Сохраняем в историю
        if self.linearization_errors.len() >= 50 {
            self.linearization_errors.remove(0);
        }
        if !self.linearization_errors.is_full() {
            self.linearization_errors.push(linearization_error);
        }
    }
    
    /// Анализ нелинейности системы
    fn analyze_nonlinearity(&mut self) {
        // Число обусловленности якобиана
        let f_jacobian = Matrix2x2::from_jacobian(
            self.state_function.jacobian(&self.state, self.dt)
        );
        
        let condition_number = self.compute_condition_number(&f_jacobian);
        self.current_result.condition_number = condition_number;
        
        // Мера нелинейности на основе изменения якобиана
        let nonlinearity_measure = self.compute_nonlinearity_measure();
        self.current_result.nonlinearity_measure = nonlinearity_measure;
        
        // Сохраняем в историю
        if self.jacobian_condition_numbers.len() >= 50 {
            self.jacobian_condition_numbers.remove(0);
        }
        if !self.jacobian_condition_numbers.is_full() {
            self.jacobian_condition_numbers.push(condition_number);
        }
        
        if self.nonlinearity_measures.len() >= 50 {
            self.nonlinearity_measures.remove(0);
        }
        if !self.nonlinearity_measures.is_full() {
            self.nonlinearity_measures.push(nonlinearity_measure);
        }
    }
    
    /// Вычисление числа обусловленности матрицы
    fn compute_condition_number(&self, matrix: &Matrix2x2) -> f64 {
        let det = matrix.data[0][0] * matrix.data[1][1] - matrix.data[0][1] * matrix.data[1][0];
        let _trace = matrix.data[0][0] + matrix.data[1][1];
        let frobenius_norm = (matrix.data[0][0].powi(2) + matrix.data[0][1].powi(2) +
                             matrix.data[1][0].powi(2) + matrix.data[1][1].powi(2)).sqrt();
        
        if det.abs() > 1e-12 {
            frobenius_norm / det.abs()
        } else {
            1e12
        }
    }
    
    /// Вычисление меры нелинейности
    fn compute_nonlinearity_measure(&self) -> f64 {
        if self.state_history.len() < 2 {
            return 0.0;
        }
        
        // Сравниваем якобианы в текущей и предыдущей точках
        let current_jacobian = self.state_function.jacobian(&self.state, self.dt);
        let prev_state = self.state_history[self.state_history.len() - 1];
        let prev_jacobian = self.state_function.jacobian(&prev_state, self.dt);
        
        let mut diff_norm = 0.0;
        for i in 0..2 {
            for j in 0..2 {
                let diff = current_jacobian[i][j] - prev_jacobian[i][j];
                diff_norm += diff * diff;
            }
        }
        
        diff_norm.sqrt()
    }
    
    /// Адаптация шума на основе инноваций
    fn adapt_noise(&mut self) {
        let innovation = self.current_result.innovation;
        
        // Простая адаптация на основе размера инноваций
        if innovation.abs() > 2.0 * self.current_result.uncertainty {
            // Увеличиваем шум процесса если инновации большие
            self.process_noise = self.process_noise.scale(1.1);
        } else if innovation.abs() < 0.5 * self.current_result.uncertainty {
            // Уменьшаем шум процесса если инновации маленькие
            self.process_noise = self.process_noise.scale(0.95);
        }
    }
    
    /// Сохранение истории
    fn save_history(&mut self) {
        if self.state_history.len() >= 100 {
            self.state_history.remove(0);
        }
        if !self.state_history.is_full() {
            self.state_history.push(self.state);
        }
        
        if self.innovation_history.len() >= 100 {
            self.innovation_history.remove(0);
        }
        if !self.innovation_history.is_full() {
            self.innovation_history.push(self.current_result.innovation);
        }
    }
    
    // Публичные методы доступа
    pub fn filtered_value(&self) -> f64 {
        self.current_result.filtered_value
    }
    
    pub fn predicted_value(&self) -> f64 {
        self.current_result.predicted_value
    }
    
    pub fn velocity(&self) -> f64 {
        self.current_result.velocity
    }
    
    pub fn uncertainty(&self) -> f64 {
        self.current_result.uncertainty
    }
    
    pub fn innovation(&self) -> f64 {
        self.current_result.innovation
    }
    
    pub fn linearization_error(&self) -> f64 {
        self.current_result.linearization_error
    }
    
    pub fn condition_number(&self) -> f64 {
        self.current_result.condition_number
    }
    
    pub fn nonlinearity_measure(&self) -> f64 {
        self.current_result.nonlinearity_measure
    }
    
    pub fn state(&self) -> [f64; 2] {
        self.state
    }
    
    pub fn covariance_matrix(&self) -> &Matrix2x2 {
        &self.covariance
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_initialized
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_result.filtered_value)
    }

    pub fn linearization_error_history(&self) -> &[f64] {
        &self.linearization_errors
    }
    
    pub fn condition_number_history(&self) -> &[f64] {
        &self.jacobian_condition_numbers
    }
    
    pub fn reset(&mut self) {
        self.state = [0.0, 0.0];
        self.covariance = Matrix2x2::new([[1000.0, 0.0], [0.0, 100.0]]);
        self.current_result = EkfResult::new();
        self.state_history.clear();
        self.innovation_history.clear();
        self.linearization_errors.clear();
        self.jacobian_condition_numbers.clear();
        self.nonlinearity_measures.clear();
        self.is_initialized = false;
    }
}

impl Clone for ExtendedKalmanFilter {
    fn clone(&self) -> Self {
        Self {
            state_function: self.state_function.clone_box(),
            observation_function: self.observation_function.clone_box(),
            dt: self.dt,
            process_noise: self.process_noise,
            measurement_noise: self.measurement_noise,
            state: self.state,
            covariance: self.covariance,
            current_result: self.current_result.clone(),
            state_history: self.state_history.clone(),
            innovation_history: self.innovation_history.clone(),
            linearization_errors: self.linearization_errors.clone(),
            adaptive_noise: self.adaptive_noise,
            innovation_variance: self.innovation_variance,
            jacobian_condition_numbers: self.jacobian_condition_numbers.clone(),
            nonlinearity_measures: self.nonlinearity_measures.clone(),
            is_initialized: self.is_initialized,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extended_kalman_filter_creation() {
        let ekf = ExtendedKalmanFilter::new(1.0, 0.1, 1.0, 0.01, ObservationType::Linear);
        assert!(!ekf.is_ready());
        assert_eq!(ekf.filtered_value(), 0.0);
    }

    #[test]
    fn test_extended_kalman_filter_warmup() {
        let mut ekf = ExtendedKalmanFilter::new(1.0, 0.1, 1.0, 0.01, ObservationType::Linear);
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ekf.update(price);
        }
        assert!(ekf.is_ready());
    }

    #[test]
    fn test_extended_kalman_filter_values_finite() {
        let mut ekf = ExtendedKalmanFilter::new(1.0, 0.1, 1.0, 0.01, ObservationType::Linear);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let result = ekf.update(price);
            assert!(result.filtered_value.is_finite());
            assert!(result.velocity.is_finite());
        }
    }

    #[test]
    fn test_extended_kalman_filter_reset() {
        let mut ekf = ExtendedKalmanFilter::new(1.0, 0.1, 1.0, 0.01, ObservationType::Linear);
        for i in 0..10 {
            ekf.update(100.0 + i as f64);
        }
        ekf.reset();
        assert!(!ekf.is_ready());
        assert_eq!(ekf.filtered_value(), 0.0);
    }
} 






















