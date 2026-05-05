//! Basic Kalman Filter
//! Базовый фильтр Калмана для отслеживания трендов и прогнозирования цен
//! Оптимально сочетает предсказания модели с зашумленными наблюдениями

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Матрица 2x2 для простых фильтров Калмана
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
    
    pub fn zeros() -> Self {
        Self {
            data: [[0.0, 0.0], [0.0, 0.0]]
        }
    }
    
    /// Умножение матриц
    pub fn multiply(&self, other: &Matrix2x2) -> Matrix2x2 {
        let mut result = Matrix2x2::zeros();
        for i in 0..2 {
            for j in 0..2 {
                for k in 0..2 {
                    result.data[i][j] += self.data[i][k] * other.data[k][j];
                }
            }
        }
        result
    }
    
    /// Транспонирование
    pub fn transpose(&self) -> Matrix2x2 {
        Matrix2x2::new([
            [self.data[0][0], self.data[1][0]],
            [self.data[0][1], self.data[1][1]]
        ])
    }
    
    /// Обращение матрицы 2x2
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
    
    /// Сложение матриц
    pub fn add(&self, other: &Matrix2x2) -> Matrix2x2 {
        Matrix2x2::new([
            [self.data[0][0] + other.data[0][0], self.data[0][1] + other.data[0][1]],
            [self.data[1][0] + other.data[1][0], self.data[1][1] + other.data[1][1]]
        ])
    }
    
    /// Вычитание матриц
    pub fn subtract(&self, other: &Matrix2x2) -> Matrix2x2 {
        Matrix2x2::new([
            [self.data[0][0] - other.data[0][0], self.data[0][1] - other.data[0][1]],
            [self.data[1][0] - other.data[1][0], self.data[1][1] - other.data[1][1]]
        ])
    }
    
    /// Умножение на скаляр
    pub fn scale(&self, scalar: f64) -> Matrix2x2 {
        Matrix2x2::new([
            [self.data[0][0] * scalar, self.data[0][1] * scalar],
            [self.data[1][0] * scalar, self.data[1][1] * scalar]
        ])
    }
    
    /// Умножение матрицы на вектор
    pub fn multiply_vector(&self, vector: &[f64; 2]) -> [f64; 2] {
        [
            self.data[0][0] * vector[0] + self.data[0][1] * vector[1],
            self.data[1][0] * vector[0] + self.data[1][1] * vector[1]
        ]
    }
    
    /// След матрицы
    pub fn trace(&self) -> f64 {
        self.data[0][0] + self.data[1][1]
    }
    
    /// Определитель
    pub fn determinant(&self) -> f64 {
        self.data[0][0] * self.data[1][1] - self.data[0][1] * self.data[1][0]
    }
}

/// Состояние фильтра Калмана
#[derive(Debug, Clone)]
pub struct KalmanState {
    pub position: f64,      // Текущая позиция (цена)
    pub velocity: f64,      // Скорость изменения (тренд)
    pub covariance: Matrix2x2, // Ковариационная матрица ошибок
}

impl KalmanState {
    pub fn new(initial_position: f64, initial_velocity: f64, initial_uncertainty: f64) -> Self {
        Self {
            position: initial_position,
            velocity: initial_velocity,
            covariance: Matrix2x2::new([
                [initial_uncertainty, 0.0],
                [0.0, initial_uncertainty]
            ]),
        }
    }
}

/// Результат фильтрации
#[derive(Debug, Clone)]
pub struct FilterResult {
    pub filtered_value: f64,        // Отфильтрованное значение
    pub predicted_value: f64,       // Предсказанное значение на следующий шаг
    pub velocity: f64,              // Скорость (тренд)
    pub acceleration: f64,          // Ускорение (изменение тренда)
    pub uncertainty: f64,           // Неопределенность оценки
    pub innovation: f64,            // Инновация (разность между наблюдением и предсказанием)
    pub kalman_gain: f64,           // Коэффициент усиления Калмана
    pub likelihood: f64,            // Вероятность наблюдения
}

impl Default for FilterResult {
    fn default() -> Self {
        Self::new()
    }
}

impl FilterResult {
    pub fn new() -> Self {
        Self {
            filtered_value: 0.0,
            predicted_value: 0.0,
            velocity: 0.0,
            acceleration: 0.0,
            uncertainty: 0.0,
            innovation: 0.0,
            kalman_gain: 0.0,
            likelihood: 0.0,
        }
    }
}

/// Basic Kalman Filter

#[derive(Clone)]
pub struct BasicKalmanFilter {
    // Параметры модели
    dt: f64,                        // Временной шаг
    process_noise: f64,             // Шум процесса (Q)
    measurement_noise: f64,         // Шум измерений (R)
    
    // Матрицы системы
    state_transition: Matrix2x2,    // Матрица перехода состояния (F)
    observation_matrix: Matrix2x2,   // Матрица наблюдений (H)
    process_noise_matrix: Matrix2x2, // Матрица шума процесса (Q)
    measurement_noise_matrix: Matrix2x2, // Матрица шума измерений (R)
    
    // Текущее состояние
    state: KalmanState,             // Текущее состояние фильтра
    
    // История для анализа
    state_history: ArrayVec<KalmanState, 100>,
    innovation_history: ArrayVec<f64, 100>,
    gain_history: ArrayVec<f64, 100>,
    
    // Результаты
    current_result: FilterResult,
    
    // Адаптивность
    adaptive_noise: bool,           // Адаптивная настройка шума
    innovation_variance: f64,       // Дисперсия инноваций
    innovation_window: ArrayVec<f64, 10>,
    
    // Статистика
    log_likelihood: f64,            // Логарифм правдоподобия
    aic: f64,                       // Информационный критерий Акаике
    
    // Состояние
    is_initialized: bool,
}

impl BasicKalmanFilter {
    pub fn new(dt: f64, process_noise: f64, measurement_noise: f64) -> Self {
        let dt = dt.max(1e-6);
        let process_noise = process_noise.max(1e-12);
        let measurement_noise = measurement_noise.max(1e-12);
        
        // Матрица перехода состояния для модели постоянной скорости
        let state_transition = Matrix2x2::new([
            [1.0, dt],      // position = position + velocity * dt
            [0.0, 1.0]      // velocity = velocity (постоянная скорость)
        ]);
        
        // Матрица наблюдений (наблюдаем только позицию)
        let observation_matrix = Matrix2x2::new([
            [1.0, 0.0],     // Наблюдаем позицию
            [0.0, 0.0]      // Не наблюдаем скорость
        ]);
        
        // Матрица шума процесса
        let q = process_noise;
        let process_noise_matrix = Matrix2x2::new([
            [q * dt * dt * dt / 3.0, q * dt * dt / 2.0],
            [q * dt * dt / 2.0, q * dt]
        ]);
        
        // Матрица шума измерений
        let measurement_noise_matrix = Matrix2x2::new([
            [measurement_noise, 0.0],
            [0.0, 1e12]  // Большая неопределенность для ненаблюдаемой скорости
        ]);
        
        Self {
            dt,
            process_noise,
            measurement_noise,
            state_transition,
            observation_matrix,
            process_noise_matrix,
            measurement_noise_matrix,
            state: KalmanState::new(0.0, 0.0, 1000.0),
            state_history: ArrayVec::new(),
            innovation_history: ArrayVec::new(),
            gain_history: ArrayVec::new(),
            current_result: FilterResult::new(),
            adaptive_noise: false,
            innovation_variance: 0.0,
            innovation_window: ArrayVec::new(),
            log_likelihood: 0.0,
            aic: 0.0,
            is_initialized: false,
        }
    }
    
    /// Создать адаптивный фильтр Калмана
    pub fn new_adaptive(dt: f64, initial_process_noise: f64, initial_measurement_noise: f64) -> Self {
        let mut filter = Self::new(dt, initial_process_noise, initial_measurement_noise);
        filter.adaptive_noise = true;
        filter
    }
    
    /// Обновить фильтр новым наблюдением
    pub fn update(&mut self, observation: f64) -> &FilterResult {
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
        
        // Адаптация шума (если включена)
        if self.adaptive_noise {
            self.adapt_noise();
        }
        
        // Сохранение истории
        self.save_history();
        
        // Обновление статистики
        self.update_statistics();
        
        &self.current_result
    }
    
    /// Инициализация фильтра первым наблюдением
    fn initialize(&mut self, observation: f64) {
        self.state.position = observation;
        self.state.velocity = 0.0;
        
        // Инициализируем с большой неопределенностью
        self.state.covariance = Matrix2x2::new([
            [1000.0, 0.0],
            [0.0, 100.0]
        ]);
    }
    
    /// Шаг предсказания
    fn predict(&mut self) {
        // Предсказание состояния: x_k|k-1 = F * x_k-1|k-1
        let state_vector = [self.state.position, self.state.velocity];
        let predicted_state = self.state_transition.multiply_vector(&state_vector);
        
        self.state.position = predicted_state[0];
        self.state.velocity = predicted_state[1];
        
        // Предсказание ковариации: P_k|k-1 = F * P_k-1|k-1 * F^T + Q
        let predicted_covariance = self.state_transition
            .multiply(&self.state.covariance)
            .multiply(&self.state_transition.transpose())
            .add(&self.process_noise_matrix);
            
        self.state.covariance = predicted_covariance;
        
        // Сохраняем предсказанное значение
        self.current_result.predicted_value = self.state.position + self.state.velocity * self.dt;
        self.current_result.velocity = self.state.velocity;
    }
    
    /// Шаг коррекции
    fn correct(&mut self, observation: f64) {
        // Инновация: y = z - H * x_k|k-1
        let predicted_observation = self.observation_matrix.multiply_vector(&[self.state.position, self.state.velocity])[0];
        let innovation = observation - predicted_observation;
        
        // Ковариация инновации: S = H * P_k|k-1 * H^T + R
        let innovation_covariance = self.observation_matrix
            .multiply(&self.state.covariance)
            .multiply(&self.observation_matrix.transpose())
            .add(&self.measurement_noise_matrix);
        
        // Коэффициент усиления Калмана: K = P_k|k-1 * H^T * S^-1
        if let Some(innovation_covariance_inv) = innovation_covariance.inverse() {
            let kalman_gain_matrix = self.state.covariance
                .multiply(&self.observation_matrix.transpose())
                .multiply(&innovation_covariance_inv);
            
            // Коррекция состояния: x_k|k = x_k|k-1 + K * y
            let gain_vector = [kalman_gain_matrix.data[0][0], kalman_gain_matrix.data[1][0]];
            self.state.position += gain_vector[0] * innovation;
            self.state.velocity += gain_vector[1] * innovation;
            
            // Коррекция ковариации: P_k|k = (I - K * H) * P_k|k-1
            let identity = Matrix2x2::identity();
            let kh = kalman_gain_matrix.multiply(&self.observation_matrix);
            let corrected_covariance = identity.subtract(&kh).multiply(&self.state.covariance);
            self.state.covariance = corrected_covariance;
            
            // Сохраняем результаты
            self.current_result.filtered_value = self.state.position;
            self.current_result.innovation = innovation;
            self.current_result.kalman_gain = gain_vector[0];
            self.current_result.uncertainty = self.state.covariance.data[0][0].sqrt();
            
            // Вычисляем правдоподобие
            let det = innovation_covariance.determinant();
            if det > 0.0 {
                let mahalanobis_distance = innovation * innovation / det;
                self.current_result.likelihood = (-0.5 * mahalanobis_distance - 0.5 * det.ln()).exp();
            }
        }
        
        // Вычисляем ускорение (изменение скорости)
        if let Some(prev_velocity) = self.state_history.last().map(|s| s.velocity) {
            self.current_result.acceleration = (self.state.velocity - prev_velocity) / self.dt;
        }
    }
    
    /// Адаптация шума на основе инноваций
    fn adapt_noise(&mut self) {
        let innovation = self.current_result.innovation;
        
        // Добавляем инновацию в окно
        if self.innovation_window.len() >= 10 {
            self.innovation_window.remove(0);
        }
        if !self.innovation_window.is_full() {
            self.innovation_window.push(innovation);
        }
        
        if self.innovation_window.len() >= 5 {
            // Вычисляем дисперсию инноваций
            let mean = self.innovation_window.iter().sum::<f64>() / self.innovation_window.len() as f64;
            let variance = self.innovation_window.iter()
                .map(|&x| (x - mean) * (x - mean))
                .sum::<f64>() / (self.innovation_window.len() - 1) as f64;
            
            self.innovation_variance = variance;
            
            // Адаптируем шум измерений
            let expected_innovation_variance = self.measurement_noise;
            if variance > expected_innovation_variance * 2.0 {
                // Увеличиваем шум измерений если инновации слишком большие
                self.measurement_noise = (self.measurement_noise * 1.1).min(variance);
                self.measurement_noise_matrix.data[0][0] = self.measurement_noise;
            } else if variance < expected_innovation_variance * 0.5 {
                // Уменьшаем шум измерений если инновации слишком маленькие
                self.measurement_noise = (self.measurement_noise * 0.9).max(1e-12);
                self.measurement_noise_matrix.data[0][0] = self.measurement_noise;
            }
        }
    }
    
    /// Сохранение истории состояний
    fn save_history(&mut self) {
        if self.state_history.len() >= 100 {
            self.state_history.remove(0);
        }
        if !self.state_history.is_full() {
            self.state_history.push(self.state.clone());
        }
        
        if self.innovation_history.len() >= 100 {
            self.innovation_history.remove(0);
        }
        if !self.innovation_history.is_full() {
            self.innovation_history.push(self.current_result.innovation);
        }
        
        if self.gain_history.len() >= 100 {
            self.gain_history.remove(0);
        }
        if !self.gain_history.is_full() {
            self.gain_history.push(self.current_result.kalman_gain);
        }
    }
    
    /// Обновление статистики
    fn update_statistics(&mut self) {
        if self.current_result.likelihood > 0.0 {
            self.log_likelihood += self.current_result.likelihood.ln();
        }
        
        // AIC = 2k - 2ln(L), где k - количество параметров
        let num_parameters = 4.0; // process_noise, measurement_noise, initial_position, initial_velocity
        self.aic = 2.0 * num_parameters - 2.0 * self.log_likelihood;
    }
    
    /// Получить прогноз на n шагов вперед
    pub fn forecast(&self, steps: usize) -> Vec<f64> {
        let mut forecast = Vec::new();
        let mut state = [self.state.position, self.state.velocity];
        
        for _ in 0..steps {
            state = self.state_transition.multiply_vector(&state);
            forecast.push(state[0]);
        }
        
        forecast
    }
    
    /// Сброс фильтра
    pub fn reset(&mut self) {
        self.state = KalmanState::new(0.0, 0.0, 1000.0);
        self.state_history.clear();
        self.innovation_history.clear();
        self.gain_history.clear();
        self.current_result = FilterResult::new();
        self.innovation_window.clear();
        self.log_likelihood = 0.0;
        self.aic = 0.0;
        self.is_initialized = false;
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
    
    pub fn acceleration(&self) -> f64 {
        self.current_result.acceleration
    }
    
    pub fn uncertainty(&self) -> f64 {
        self.current_result.uncertainty
    }
    
    pub fn innovation(&self) -> f64 {
        self.current_result.innovation
    }
    
    pub fn kalman_gain(&self) -> f64 {
        self.current_result.kalman_gain
    }
    
    pub fn likelihood(&self) -> f64 {
        self.current_result.likelihood
    }
    
    pub fn log_likelihood(&self) -> f64 {
        self.log_likelihood
    }
    
    pub fn aic(&self) -> f64 {
        self.aic
    }
    
    pub fn innovation_variance(&self) -> f64 {
        self.innovation_variance
    }
    
    pub fn current_noise_parameters(&self) -> (f64, f64) {
        (self.process_noise, self.measurement_noise)
    }
    
    pub fn state_history(&self) -> &[KalmanState] {
        &self.state_history
    }
    
    pub fn innovation_history(&self) -> &[f64] {
        &self.innovation_history
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_initialized
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_result.filtered_value)
    }

    pub fn set_process_noise(&mut self, noise: f64) {
        self.process_noise = noise.max(1e-12);
        let q = self.process_noise;
        let dt = self.dt;
        self.process_noise_matrix = Matrix2x2::new([
            [q * dt * dt * dt / 3.0, q * dt * dt / 2.0],
            [q * dt * dt / 2.0, q * dt]
        ]);
    }
    
    pub fn set_measurement_noise(&mut self, noise: f64) {
        self.measurement_noise = noise.max(1e-12);
        self.measurement_noise_matrix.data[0][0] = self.measurement_noise;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_kalman_filter_creation() {
        let kf = BasicKalmanFilter::new(1.0, 0.1, 1.0);
        assert!(!kf.is_ready());
        assert_eq!(kf.filtered_value(), 0.0);
    }

    #[test]
    fn test_basic_kalman_filter_warmup() {
        let mut kf = BasicKalmanFilter::new(1.0, 0.1, 1.0);
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kf.update(price);
        }
        assert!(kf.is_ready());
    }

    #[test]
    fn test_basic_kalman_filter_values_finite() {
        let mut kf = BasicKalmanFilter::new(1.0, 0.1, 1.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let result = kf.update(price);
            assert!(result.filtered_value.is_finite());
            assert!(result.velocity.is_finite());
        }
    }

    #[test]
    fn test_basic_kalman_filter_reset() {
        let mut kf = BasicKalmanFilter::new(1.0, 0.1, 1.0);
        for i in 0..10 {
            kf.update(100.0 + i as f64);
        }
        kf.reset();
        assert!(!kf.is_ready());
        assert_eq!(kf.filtered_value(), 0.0);
    }

    #[test]
    fn test_basic_kalman_filter_forecast() {
        let mut kf = BasicKalmanFilter::new(1.0, 0.1, 1.0);
        for i in 0..10 {
            kf.update(100.0 + i as f64);
        }
        let forecast = kf.forecast(5);
        assert_eq!(forecast.len(), 5);
        for f in &forecast {
            assert!(f.is_finite());
        }
    }

    #[test]
    fn test_matrix2x2_operations() {
        let a = Matrix2x2::identity();
        let b = Matrix2x2::new([[2.0, 0.0], [0.0, 2.0]]);

        let c = a.multiply(&b);
        assert_eq!(c.data[0][0], 2.0);
        assert_eq!(c.data[1][1], 2.0);

        let inv = b.inverse().unwrap();
        assert_eq!(inv.data[0][0], 0.5);
    }
} 






















