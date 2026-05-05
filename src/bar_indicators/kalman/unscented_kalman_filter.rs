//! Unscented Kalman Filter
//! Сигма-точечный фильтр Калмана для нелинейных систем
//! Использует детерминированную выборку сигма-точек для аппроксимации распределений

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Параметры Unscented Transform
#[derive(Debug, Clone, Copy)]
pub struct UnscentedTransformParams {
    pub alpha: f64,  // Параметр разброса (обычно 0.001-1)
    pub beta: f64,   // Параметр для учета высших моментов (обычно 2.0 для гауссовых распределений)
    pub kappa: f64,  // Вторичный параметр масштабирования (обычно 3-n)
}

impl Default for UnscentedTransformParams {
    fn default() -> Self {
        Self {
            alpha: 0.001,
            beta: 2.0,
            kappa: 0.0,
        }
    }
}

/// Сигма-точка
#[derive(Debug, Clone, Copy)]
pub struct SigmaPoint {
    pub state: [f64; 2],    // Состояние [позиция, скорость]
    pub weight_m: f64,      // Вес для вычисления среднего
    pub weight_c: f64,      // Вес для вычисления ковариации
}

/// Результат UKF
#[derive(Debug, Clone)]
pub struct UkfResult {
    pub filtered_value: f64,        // Отфильтрованное значение
    pub predicted_value: f64,       // Предсказанное значение
    pub velocity: f64,              // Скорость
    pub uncertainty: f64,           // Неопределенность
    pub innovation: f64,            // Инновация
    pub sigma_point_spread: f64,    // Разброс сигма-точек
    pub transform_quality: f64,     // Качество трансформации
    pub effective_sample_size: f64, // Эффективный размер выборки
}

impl Default for UkfResult {
    fn default() -> Self {
        Self::new()
    }
}

impl UkfResult {
    pub fn new() -> Self {
        Self {
            filtered_value: 0.0,
            predicted_value: 0.0,
            velocity: 0.0,
            uncertainty: 0.0,
            innovation: 0.0,
            sigma_point_spread: 0.0,
            transform_quality: 0.0,
            effective_sample_size: 0.0,
        }
    }
}

/// Матрица 2x2 для UKF
#[derive(Debug, Clone, Copy)]
pub struct Matrix2x2 {
    pub data: [[f64; 2]; 2],
}

impl Matrix2x2 {
    pub fn new(data: [[f64; 2]; 2]) -> Self {
        Self { data }
    }
    
    pub fn identity() -> Self {
        Self::new([[1.0, 0.0], [0.0, 1.0]])
    }
    
    pub fn zeros() -> Self {
        Self::new([[0.0, 0.0], [0.0, 0.0]])
    }
    
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
    
    pub fn add(&self, other: &Matrix2x2) -> Matrix2x2 {
        Matrix2x2::new([
            [self.data[0][0] + other.data[0][0], self.data[0][1] + other.data[0][1]],
            [self.data[1][0] + other.data[1][0], self.data[1][1] + other.data[1][1]]
        ])
    }
    
    pub fn scale(&self, scalar: f64) -> Matrix2x2 {
        Matrix2x2::new([
            [self.data[0][0] * scalar, self.data[0][1] * scalar],
            [self.data[1][0] * scalar, self.data[1][1] * scalar]
        ])
    }
    
    pub fn transpose(&self) -> Matrix2x2 {
        Matrix2x2::new([
            [self.data[0][0], self.data[1][0]],
            [self.data[0][1], self.data[1][1]]
        ])
    }
    
    pub fn cholesky(&self) -> Option<Matrix2x2> {
        // Разложение Холецкого для положительно определенной матрицы
        let a = self.data[0][0];
        let b = self.data[0][1];
        let c = self.data[1][1];
        
        if a <= 0.0 {
            return None;
        }
        
        let l11 = a.sqrt();
        let l21 = b / l11;
        let l22_sq = c - l21 * l21;
        
        if l22_sq <= 0.0 {
            return None;
        }
        
        let l22 = l22_sq.sqrt();
        
        Some(Matrix2x2::new([
            [l11, 0.0],
            [l21, l22]
        ]))
    }
    
    pub fn determinant(&self) -> f64 {
        self.data[0][0] * self.data[1][1] - self.data[0][1] * self.data[1][0]
    }
    
    pub fn trace(&self) -> f64 {
        self.data[0][0] + self.data[1][1]
    }
}

/// Unscented Kalman Filter
#[derive(Clone)]
pub struct UnscentedKalmanFilter {
    // Параметры фильтра
    dt: f64,
    process_noise: Matrix2x2,
    measurement_noise: f64,
    ut_params: UnscentedTransformParams,
    
    // Состояние фильтра
    state: [f64; 2],                    // [позиция, скорость]
    covariance: Matrix2x2,              // Ковариационная матрица
    
    // Сигма-точки
    sigma_points: ArrayVec<SigmaPoint, 5>,  // 2*n+1 точек для n=2
    predicted_sigma_points: ArrayVec<[f64; 2], 5>,
    predicted_observations: ArrayVec<f64, 5>,
    
    // Результаты
    current_result: UkfResult,
    
    // Параметры трансформации
    lambda: f64,                        // Параметр масштабирования
    #[allow(dead_code)]
    n_sigma: usize,                     // Количество сигма-точек
    
    // История для анализа
    innovation_history: ArrayVec<f64, 100>,
    uncertainty_history: ArrayVec<f64, 100>,
    
    // Диагностика
    transform_quality_history: ArrayVec<f64, 50>,
    sigma_point_spreads: ArrayVec<f64, 50>,
    
    // Состояние
    is_initialized: bool,
}

impl UnscentedKalmanFilter {
    pub fn new(
        dt: f64,
        process_noise_std: f64,
        measurement_noise_std: f64,
        ut_params: Option<UnscentedTransformParams>
    ) -> Self {
        let ut_params = ut_params.unwrap_or_default();
        let n = 2; // Размерность состояния
        let lambda = ut_params.alpha.powi(2) * (n as f64 + ut_params.kappa) - n as f64;
        
        let mut filter = Self {
            dt,
            process_noise: Matrix2x2::new([
                [process_noise_std.powi(2), 0.0],
                [0.0, process_noise_std.powi(2)]
            ]),
            measurement_noise: measurement_noise_std.powi(2),
            ut_params,
            state: [0.0, 0.0],
            covariance: Matrix2x2::new([
                [1000.0, 0.0],
                [0.0, 100.0]
            ]),
            sigma_points: ArrayVec::new(),
            predicted_sigma_points: ArrayVec::new(),
            predicted_observations: ArrayVec::new(),
            current_result: UkfResult::new(),
            lambda,
            n_sigma: 2 * n + 1,
            innovation_history: ArrayVec::new(),
            uncertainty_history: ArrayVec::new(),
            transform_quality_history: ArrayVec::new(),
            sigma_point_spreads: ArrayVec::new(),
            is_initialized: false,
        };
        
        filter.calculate_weights();
        filter
    }
    
    /// Обновление фильтра
    pub fn update(&mut self, observation: f64) -> &UkfResult {
        if !self.is_initialized {
            self.initialize(observation);
            self.current_result.filtered_value = observation;
            return &self.current_result;
        }
        
        // 1. Генерация сигма-точек
        self.generate_sigma_points();
        
        // 2. Шаг предсказания
        self.predict();
        
        // 3. Шаг коррекции
        self.correct(observation);
        
        // 4. Анализ качества
        self.analyze_transform_quality();
        
        // 5. Сохранение истории
        self.save_history();
        
        &self.current_result
    }
    
    /// Инициализация фильтра
    fn initialize(&mut self, observation: f64) {
        self.state[0] = observation;
        self.state[1] = 0.0;
        
        self.current_result.filtered_value = observation;
        self.current_result.predicted_value = observation;
        self.current_result.velocity = 0.0;
        self.current_result.uncertainty = self.covariance.data[0][0].sqrt();
        
        self.is_initialized = true;
    }
    
    /// Вычисление весов для сигма-точек
    fn calculate_weights(&mut self) {
        self.sigma_points.clear();
        
        let n = 2.0;
        let lambda = self.lambda;
        let alpha = self.ut_params.alpha;
        let beta = self.ut_params.beta;
        
        // Веса для средней точки
        let w_m_0 = lambda / (n + lambda);
        let w_c_0 = w_m_0 + (1.0 - alpha.powi(2) + beta);
        
        // Веса для остальных точек
        let w_others = 1.0 / (2.0 * (n + lambda));
        
        // Добавляем точки с весами
        if !self.sigma_points.is_full() {
            self.sigma_points.push(SigmaPoint {
                state: [0.0, 0.0],
                weight_m: w_m_0,
                weight_c: w_c_0,
            });
        }
        
        // Добавляем остальные 4 точки
        for _ in 0..4 {
            if !self.sigma_points.is_full() {
                self.sigma_points.push(SigmaPoint {
                    state: [0.0, 0.0],
                    weight_m: w_others,
                    weight_c: w_others,
                });
            }
        }
    }
    
    /// Генерация сигма-точек
    fn generate_sigma_points(&mut self) {
        let n = 2.0;
        let lambda = self.lambda;
        
        // Разложение Холецкого
        let sqrt_matrix = if let Some(chol) = self.covariance.scale(n + lambda).cholesky() {
            chol
        } else {
            // Если разложение не удалось, используем диагональную матрицу
            Matrix2x2::new([
                [(n + lambda) * self.covariance.data[0][0], 0.0],
                [0.0, (n + lambda) * self.covariance.data[1][1]]
            ])
        };
        
        // Центральная точка
        if !self.sigma_points.is_empty() {
            self.sigma_points[0].state = self.state;
        }
        
        // Точки +/- для каждой размерности
        for i in 0..2 {
            if i * 2 + 1 < self.sigma_points.len() {
                self.sigma_points[i * 2 + 1].state = [
                    self.state[0] + sqrt_matrix.data[0][i],
                    self.state[1] + sqrt_matrix.data[1][i]
                ];
            }
            
            if i * 2 + 2 < self.sigma_points.len() {
                self.sigma_points[i * 2 + 2].state = [
                    self.state[0] - sqrt_matrix.data[0][i],
                    self.state[1] - sqrt_matrix.data[1][i]
                ];
            }
        }
    }
    
    /// Шаг предсказания
    fn predict(&mut self) {
        self.predicted_sigma_points.clear();
        self.predicted_observations.clear();
        
        // Пропускаем каждую сигма-точку через нелинейную функцию состояния
        for sigma_point in &self.sigma_points {
            let predicted_state = self.state_function(&sigma_point.state);
            if !self.predicted_sigma_points.is_full() {
                self.predicted_sigma_points.push(predicted_state);
            }
            
            // Также вычисляем предсказанные наблюдения
            let predicted_obs = self.observation_function(&predicted_state);
            if !self.predicted_observations.is_full() {
                self.predicted_observations.push(predicted_obs);
            }
        }
        
        // Вычисляем предсказанное среднее состояние
        let mut mean_state = [0.0, 0.0];
        for (i, predicted_state) in self.predicted_sigma_points.iter().enumerate() {
            if i < self.sigma_points.len() {
                let weight = self.sigma_points[i].weight_m;
                mean_state[0] += weight * predicted_state[0];
                mean_state[1] += weight * predicted_state[1];
            }
        }
        
        // Вычисляем предсказанную ковариацию
        let mut predicted_cov = Matrix2x2::zeros();
        for (i, predicted_state) in self.predicted_sigma_points.iter().enumerate() {
            if i < self.sigma_points.len() {
                let weight = self.sigma_points[i].weight_c;
                let diff = [
                    predicted_state[0] - mean_state[0],
                    predicted_state[1] - mean_state[1]
                ];
                
                predicted_cov.data[0][0] += weight * diff[0] * diff[0];
                predicted_cov.data[0][1] += weight * diff[0] * diff[1];
                predicted_cov.data[1][0] += weight * diff[1] * diff[0];
                predicted_cov.data[1][1] += weight * diff[1] * diff[1];
            }
        }
        
        // Добавляем шум процесса
        predicted_cov = predicted_cov.add(&self.process_noise);
        
        // Обновляем состояние
        self.state = mean_state;
        self.covariance = predicted_cov;
        
        // Вычисляем предсказанное наблюдение
        let mut predicted_obs = 0.0;
        for (i, obs) in self.predicted_observations.iter().enumerate() {
            if i < self.sigma_points.len() {
                predicted_obs += self.sigma_points[i].weight_m * obs;
            }
        }
        
        self.current_result.predicted_value = predicted_obs;
        self.current_result.velocity = self.state[1];
    }
    
    /// Шаг коррекции
    fn correct(&mut self, observation: f64) {
        // Вычисляем ковариацию инноваций
        let mut innovation_cov = 0.0;
        let predicted_obs = self.current_result.predicted_value;
        
        for (i, obs) in self.predicted_observations.iter().enumerate() {
            if i < self.sigma_points.len() {
                let weight = self.sigma_points[i].weight_c;
                let diff = obs - predicted_obs;
                innovation_cov += weight * diff * diff;
            }
        }
        innovation_cov += self.measurement_noise;
        
        // Вычисляем кросс-ковариацию
        let mut cross_cov = [0.0, 0.0];
        for (i, (predicted_state, predicted_obs_i)) in self.predicted_sigma_points.iter()
            .zip(self.predicted_observations.iter()).enumerate() {
            if i < self.sigma_points.len() {
                let weight = self.sigma_points[i].weight_c;
                let state_diff = [
                    predicted_state[0] - self.state[0],
                    predicted_state[1] - self.state[1]
                ];
                let obs_diff = predicted_obs_i - predicted_obs;
                
                cross_cov[0] += weight * state_diff[0] * obs_diff;
                cross_cov[1] += weight * state_diff[1] * obs_diff;
            }
        }
        
        // Вычисляем коэффициент усиления Калмана
        let innovation = observation - predicted_obs;
        
        if innovation_cov > 1e-12 {
            let kalman_gain = [
                cross_cov[0] / innovation_cov,
                cross_cov[1] / innovation_cov
            ];
            
            // Обновляем состояние
            self.state[0] += kalman_gain[0] * innovation;
            self.state[1] += kalman_gain[1] * innovation;
            
            // Обновляем ковариацию
            let gain_innovation_cov = Matrix2x2::new([
                [kalman_gain[0] * innovation_cov * kalman_gain[0], kalman_gain[0] * innovation_cov * kalman_gain[1]],
                [kalman_gain[1] * innovation_cov * kalman_gain[0], kalman_gain[1] * innovation_cov * kalman_gain[1]]
            ]);
            
            self.covariance = self.covariance.add(&gain_innovation_cov.scale(-1.0));
            
            // Обновляем результаты
            self.current_result.filtered_value = self.observation_function(&self.state);
            self.current_result.innovation = innovation;
            self.current_result.uncertainty = self.covariance.data[0][0].sqrt();
        }
    }
    
    /// Функция состояния (модель движения)
    fn state_function(&self, state: &[f64; 2]) -> [f64; 2] {
        // Простая модель постоянной скорости с затуханием
        let friction = 0.95;
        [
            state[0] + self.dt * state[1],
            state[1] * friction
        ]
    }
    
    /// Функция наблюдения
    fn observation_function(&self, state: &[f64; 2]) -> f64 {
        // Наблюдаем позицию напрямую
        state[0]
    }
    
    /// Анализ качества трансформации
    fn analyze_transform_quality(&mut self) {
        // Разброс сигма-точек
        if !self.predicted_sigma_points.is_empty() {
            let mut mean = [0.0, 0.0];
            for state in &self.predicted_sigma_points {
                mean[0] += state[0];
                mean[1] += state[1];
            }
            mean[0] /= self.predicted_sigma_points.len() as f64;
            mean[1] /= self.predicted_sigma_points.len() as f64;
            
            let mut spread = 0.0;
            for state in &self.predicted_sigma_points {
                let diff = [(state[0] - mean[0]).powi(2), (state[1] - mean[1]).powi(2)];
                spread += diff[0] + diff[1];
            }
            spread = (spread / self.predicted_sigma_points.len() as f64).sqrt();
            
            self.current_result.sigma_point_spread = spread;
        }
        
        // Эффективный размер выборки
        let mut sum_weights_sq = 0.0;
        for sigma_point in &self.sigma_points {
            sum_weights_sq += sigma_point.weight_m.powi(2);
        }
        self.current_result.effective_sample_size = if sum_weights_sq > 0.0 {
            1.0 / sum_weights_sq
        } else {
            0.0
        };
        
        // Качество трансформации на основе детерминанта ковариации
        let det = self.covariance.determinant();
        self.current_result.transform_quality = if det > 0.0 {
            1.0 / (1.0 + det.ln().abs())
        } else {
            0.0
        };
    }
    
    /// Сохранение истории
    fn save_history(&mut self) {
        // Сохраняем инновации
        if self.innovation_history.len() >= 100 {
            self.innovation_history.remove(0);
        }
        if !self.innovation_history.is_full() {
            self.innovation_history.push(self.current_result.innovation);
        }
        
        // Сохраняем неопределенность
        if self.uncertainty_history.len() >= 100 {
            self.uncertainty_history.remove(0);
        }
        if !self.uncertainty_history.is_full() {
            self.uncertainty_history.push(self.current_result.uncertainty);
        }
        
        // Сохраняем качество трансформации
        if self.transform_quality_history.len() >= 50 {
            self.transform_quality_history.remove(0);
        }
        if !self.transform_quality_history.is_full() {
            self.transform_quality_history.push(self.current_result.transform_quality);
        }
        
        // Сохраняем разброс сигма-точек
        if self.sigma_point_spreads.len() >= 50 {
            self.sigma_point_spreads.remove(0);
        }
        if !self.sigma_point_spreads.is_full() {
            self.sigma_point_spreads.push(self.current_result.sigma_point_spread);
        }
    }
    
    // Публичные методы
    
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
    
    pub fn sigma_point_spread(&self) -> f64 {
        self.current_result.sigma_point_spread
    }
    
    pub fn transform_quality(&self) -> f64 {
        self.current_result.transform_quality
    }
    
    pub fn effective_sample_size(&self) -> f64 {
        self.current_result.effective_sample_size
    }
    
    pub fn state(&self) -> [f64; 2] {
        self.state
    }
    
    pub fn covariance_matrix(&self) -> &Matrix2x2 {
        &self.covariance
    }
    
    pub fn sigma_points(&self) -> &[SigmaPoint] {
        &self.sigma_points
    }
    
    pub fn is_ready(&self) -> bool {
        self.is_initialized
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_result.filtered_value)
    }

    pub fn innovation_history(&self) -> &[f64] {
        &self.innovation_history
    }
    
    pub fn uncertainty_history(&self) -> &[f64] {
        &self.uncertainty_history
    }
    
    pub fn transform_quality_history(&self) -> &[f64] {
        &self.transform_quality_history
    }
    
    pub fn reset(&mut self) {
        self.state = [0.0, 0.0];
        self.covariance = Matrix2x2::new([
            [1000.0, 0.0],
            [0.0, 100.0]
        ]);
        self.sigma_points.clear();
        self.predicted_sigma_points.clear();
        self.predicted_observations.clear();
        self.current_result = UkfResult::new();
        self.innovation_history.clear();
        self.uncertainty_history.clear();
        self.transform_quality_history.clear();
        self.sigma_point_spreads.clear();
        self.is_initialized = false;
        self.calculate_weights();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unscented_kalman_filter_creation() {
        let ukf = UnscentedKalmanFilter::new(1.0, 0.1, 1.0, None);
        assert!(!ukf.is_ready());
        assert_eq!(ukf.filtered_value(), 0.0);
    }

    #[test]
    fn test_unscented_kalman_filter_warmup() {
        let mut ukf = UnscentedKalmanFilter::new(1.0, 0.1, 1.0, None);
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ukf.update(price);
        }
        assert!(ukf.is_ready());
    }

    #[test]
    fn test_unscented_kalman_filter_values_finite() {
        let mut ukf = UnscentedKalmanFilter::new(1.0, 0.1, 1.0, None);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let result = ukf.update(price);
            assert!(result.filtered_value.is_finite());
            assert!(result.velocity.is_finite());
        }
    }

    #[test]
    fn test_unscented_kalman_filter_reset() {
        let mut ukf = UnscentedKalmanFilter::new(1.0, 0.1, 1.0, None);
        for i in 0..10 {
            ukf.update(100.0 + i as f64);
        }
        ukf.reset();
        assert!(!ukf.is_ready());
        assert_eq!(ukf.filtered_value(), 0.0);
    }

    #[test]
    fn test_unscented_kalman_filter_with_params() {
        let params = UnscentedTransformParams {
            alpha: 0.01,
            beta: 2.0,
            kappa: 1.0,
        };
        let mut ukf = UnscentedKalmanFilter::new(1.0, 0.1, 1.0, Some(params));
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            let result = ukf.update(price);
            assert!(result.filtered_value.is_finite());
        }
    }
} 






















