//! VAR Model - Vector AutoRegression
//! Векторная авторегрессионная модель для анализа множественных временных рядов
//! VAR(p) - p лагов для каждой переменной во всех уравнениях

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// VAR Model - Vector AutoRegression
#[derive(Clone)]
pub struct Var {
    // Параметры модели
    p: usize,                    // Порядок VAR (количество лагов)
    n_vars: usize,              // Количество переменных
    
    // Данные
    data: ArrayVec<ArrayVec<f64, 16>, 512>, // Матрица данных [time x variables]
    
    // Коэффициенты модели - матрица коэффициентов для каждого лага
    // coefficients[lag][var_from][var_to] = коэффициент влияния var_from на var_to с лагом lag
    coefficients: ArrayVec<ArrayVec<ArrayVec<f64, 16>, 16>, 8>, // [lag][from_var][to_var]
    constants: ArrayVec<f64, 16>,           // Константы для каждого уравнения
    
    // Остатки и прогнозы
    residuals: ArrayVec<ArrayVec<f64, 16>, 512>, // Остатки для каждой переменной
    fitted_values: ArrayVec<ArrayVec<f64, 16>, 512>, // Подогнанные значения
    forecasts: ArrayVec<f64, 16>,           // Прогнозы для каждой переменной
    
    // Ковариационная матрица остатков
    residual_covariance: ArrayVec<ArrayVec<f64, 16>, 16>, // Σ matrix
    
    // Метрики модели
    log_likelihood: f64,
    aic: f64,
    bic: f64,
    
    // Импульсные отклики (упрощенная версия)
    impulse_responses: ArrayVec<ArrayVec<ArrayVec<f64, 16>, 16>, 20>, // [horizon][shock_var][response_var]
    
    // Состояние
    is_fitted: bool,
    min_observations: usize,
}

impl Var {
    pub fn new(p: usize, n_vars: usize) -> Self {
        let p = p.min(8);        // Максимум 8 лагов
        let n_vars = n_vars.min(16); // Максимум 16 переменных
        let min_obs = (p * n_vars + 10).max(30);
        
        Self {
            p,
            n_vars,
            data: ArrayVec::new(),
            coefficients: ArrayVec::new(),
            constants: ArrayVec::new(),
            residuals: ArrayVec::new(),
            fitted_values: ArrayVec::new(),
            forecasts: ArrayVec::new(),
            residual_covariance: ArrayVec::new(),
            log_likelihood: f64::NEG_INFINITY,
            aic: f64::INFINITY,
            bic: f64::INFINITY,
            impulse_responses: ArrayVec::new(),
            is_fitted: false,
            min_observations: min_obs,
        }
    }
    
    /// Обновить модель новыми данными
    pub fn update(&mut self, values: &[f64]) -> &[f64] {
        // Проверяем размерность входных данных
        if values.len() != self.n_vars {
            return &[];
        }
        
        // Добавляем новые данные
        let mut data_row = ArrayVec::new();
        for &val in values.iter().take(self.n_vars) {
            if !data_row.is_full() {
                data_row.push(val);
            }
        }
        
        if self.data.len() >= 512 {
            self.data.remove(0);
        }
        if !self.data.is_full() {
            self.data.push(data_row);
        }
        
        // Если достаточно данных, переоцениваем модель
        if self.data.len() >= self.min_observations {
            self.fit_model();
            self.generate_forecasts();
        }
        
        &self.forecasts
    }
    
    /// Подгонка VAR модели
    fn fit_model(&mut self) {
        if self.data.len() < self.min_observations {
            return;
        }
        
        // 1. Инициализируем структуры коэффициентов
        self.initialize_coefficient_structures();
        
        // 2. Оцениваем коэффициенты методом наименьших квадратов
        self.estimate_coefficients();
        
        // 3. Рассчитываем остатки и подогнанные значения
        self.calculate_residuals_and_fitted();
        
        // 4. Рассчитываем ковариационную матрицу остатков
        self.calculate_residual_covariance();
        
        // 5. Рассчитываем метрики модели
        self.calculate_model_metrics();
        
        // 6. Рассчитываем импульсные отклики (упрощенно)
        self.calculate_impulse_responses();
        
        self.is_fitted = true;
    }
    
    /// Инициализация структур коэффициентов
    fn initialize_coefficient_structures(&mut self) {
        self.coefficients.clear();
        self.constants.clear();
        
        // Инициализируем коэффициенты нулями
        for _ in 0..self.p {
            let mut lag_coeffs = ArrayVec::new();
            for _ in 0..self.n_vars {
                let mut from_var_coeffs = ArrayVec::new();
                for _ in 0..self.n_vars {
                    if !from_var_coeffs.is_full() {
                        from_var_coeffs.push(0.0);
                    }
                }
                if !lag_coeffs.is_full() {
                    lag_coeffs.push(from_var_coeffs);
                }
            }
            if !self.coefficients.is_full() {
                self.coefficients.push(lag_coeffs);
            }
        }
        
        // Инициализируем константы
        for _ in 0..self.n_vars {
            if !self.constants.is_full() {
                self.constants.push(0.0);
            }
        }
    }
    
    /// Оценка коэффициентов методом наименьших квадратов
    fn estimate_coefficients(&mut self) {
        let n_obs = self.data.len();
        let start_idx = self.p;
        
        if n_obs <= start_idx {
            return;
        }
        
        // Для каждой переменной (уравнения) оцениваем коэффициенты отдельно
        for eq_idx in 0..self.n_vars {
            self.estimate_single_equation(eq_idx, start_idx);
        }
    }
    
    /// Оценка одного уравнения VAR
    fn estimate_single_equation(&mut self, eq_idx: usize, start_idx: usize) {
        let n_obs = self.data.len() - start_idx;
        let n_regressors = self.n_vars * self.p + 1; // +1 для константы
        
        // Создаем матрицы для регрессии
        let mut x_matrix = vec![vec![0.0; n_regressors]; n_obs];
        let mut y_vector = vec![0.0; n_obs];
        
        // Заполняем данные
        for t in 0..n_obs {
            let data_idx = start_idx + t;
            
            // Зависимая переменная
            y_vector[t] = self.data[data_idx][eq_idx];
            
            // Константа
            x_matrix[t][0] = 1.0;
            
            // Лаги всех переменных
            let mut regressor_idx = 1;
            for lag in 1..=self.p {
                for var_idx in 0..self.n_vars {
                    if data_idx >= lag {
                        x_matrix[t][regressor_idx] = self.data[data_idx - lag][var_idx];
                    }
                    regressor_idx += 1;
                }
            }
        }
        
        // Решаем систему методом наименьших квадратов (упрощенно)
        let coeffs = self.solve_ols(&x_matrix, &y_vector);
        
        // Сохраняем коэффициенты
        if !coeffs.is_empty() {
            // Константа
            if eq_idx < self.constants.len() {
                self.constants[eq_idx] = coeffs[0];
            }
            
            // Коэффициенты лагов
            let mut coeff_idx = 1;
            for lag in 0..self.p {
                for var_idx in 0..self.n_vars {
                    if lag < self.coefficients.len() && 
                       var_idx < self.coefficients[lag].len() && 
                       eq_idx < self.coefficients[lag][var_idx].len() &&
                       coeff_idx < coeffs.len() {
                        self.coefficients[lag][var_idx][eq_idx] = coeffs[coeff_idx];
                    }
                    coeff_idx += 1;
                }
            }
        }
    }
    
    /// Простой решатель OLS (метод нормальных уравнений)
    fn solve_ols(&self, x_matrix: &[Vec<f64>], y_vector: &[f64]) -> Vec<f64> {
        if x_matrix.is_empty() || y_vector.is_empty() {
            return Vec::new();
        }
        
        let n = x_matrix.len();
        let k = x_matrix[0].len();
        
        // X'X матрица
        let mut xtx = vec![vec![0.0; k]; k];
        for i in 0..k {
            for j in 0..k {
                for row in &x_matrix[..n] {
                    xtx[i][j] += row[i] * row[j];
                }
            }
        }

        // X'y вектор
        let mut xty = vec![0.0; k];
        for i in 0..k {
            for (row, &y) in x_matrix[..n].iter().zip(y_vector[..n].iter()) {
                xty[i] += row[i] * y;
            }
        }
        
        // Решение системы (диагональное приближение)
        let mut coeffs = Vec::new();
        for i in 0..k {
            if xtx[i][i].abs() > 1e-10 {
                coeffs.push(xty[i] / xtx[i][i]);
            } else {
                coeffs.push(0.0);
            }
        }
        
        coeffs
    }
    
    /// Рассчитать остатки и подогнанные значения
    fn calculate_residuals_and_fitted(&mut self) {
        self.residuals.clear();
        self.fitted_values.clear();
        
        let start_idx = self.p;
        if self.data.len() <= start_idx {
            return;
        }
        
        for t in start_idx..self.data.len() {
            let mut fitted_row = ArrayVec::new();
            let mut residual_row = ArrayVec::new();
            
            // Для каждой переменной
            for var_idx in 0..self.n_vars {
                let mut fitted_value = if var_idx < self.constants.len() {
                    self.constants[var_idx]
                } else {
                    0.0
                };
                
                // Добавляем вклад лагов
                for lag in 0..self.p {
                    for lag_var in 0..self.n_vars {
                        if t > lag && 
                           lag < self.coefficients.len() &&
                           lag_var < self.coefficients[lag].len() &&
                           var_idx < self.coefficients[lag][lag_var].len() {
                            let coeff = self.coefficients[lag][lag_var][var_idx];
                            fitted_value += coeff * self.data[t - 1 - lag][lag_var];
                        }
                    }
                }
                
                let actual_value = self.data[t][var_idx];
                let residual = actual_value - fitted_value;
                
                if !fitted_row.is_full() {
                    fitted_row.push(fitted_value);
                }
                if !residual_row.is_full() {
                    residual_row.push(residual);
                }
            }
            
            if !self.fitted_values.is_full() {
                self.fitted_values.push(fitted_row);
            }
            if !self.residuals.is_full() {
                self.residuals.push(residual_row);
            }
        }
    }
    
    /// Рассчитать ковариационную матрицу остатков
    fn calculate_residual_covariance(&mut self) {
        self.residual_covariance.clear();
        
        if self.residuals.is_empty() {
            return;
        }
        
        let n_obs = self.residuals.len() as f64;
        
        // Инициализируем ковариационную матрицу
        for i in 0..self.n_vars {
            let mut cov_row = ArrayVec::new();
            for j in 0..self.n_vars {
                let mut covariance = 0.0;
                
                // Рассчитываем ковариацию между переменными i и j
                for residual_row in &self.residuals {
                    if i < residual_row.len() && j < residual_row.len() {
                        covariance += residual_row[i] * residual_row[j];
                    }
                }
                
                covariance /= n_obs;
                
                if !cov_row.is_full() {
                    cov_row.push(covariance);
                }
            }
            if !self.residual_covariance.is_full() {
                self.residual_covariance.push(cov_row);
            }
        }
    }
    
    /// Рассчитать метрики модели
    fn calculate_model_metrics(&mut self) {
        if self.residuals.is_empty() || self.residual_covariance.is_empty() {
            return;
        }
        
        let n_obs = self.residuals.len() as f64;
        let n_params = (self.n_vars * self.n_vars * self.p + self.n_vars) as f64;
        
        // Логарифм правдоподобия (многомерное нормальное распределение)
        let det_sigma = self.calculate_determinant(&self.residual_covariance);
        
        if det_sigma > 0.0 {
            self.log_likelihood = -0.5 * n_obs * (
                self.n_vars as f64 * (2.0 * std::f64::consts::PI).ln() + 
                det_sigma.ln() + 
                self.n_vars as f64
            );
        }
        
        // AIC и BIC
        self.aic = -2.0 * self.log_likelihood + 2.0 * n_params;
        self.bic = -2.0 * self.log_likelihood + n_params * n_obs.ln();
    }
    
    /// Вычисление определителя матрицы (упрощенно - только для диагональных элементов)
    fn calculate_determinant(&self, matrix: &[ArrayVec<f64, 16>]) -> f64 {
        let mut det = 1.0;
        for (i, row) in matrix.iter().enumerate().take(self.n_vars) {
            if i < row.len() {
                det *= row[i].abs();
            }
        }
        det
    }
    
    /// Рассчитать импульсные отклики (упрощенная версия)
    fn calculate_impulse_responses(&mut self) {
        self.impulse_responses.clear();
        
        let max_horizon = 20;
        
        // Для каждого горизонта
        for h in 0..max_horizon {
            let mut horizon_responses = ArrayVec::new();
            
            // Для каждой переменной-шока
            for shock_var in 0..self.n_vars {
                let mut shock_responses = ArrayVec::new();
                
                // Для каждой переменной-отклика
                for response_var in 0..self.n_vars {
                    let impulse_response = self.calculate_single_impulse_response(
                        shock_var, response_var, h
                    );
                    
                    if !shock_responses.is_full() {
                        shock_responses.push(impulse_response);
                    }
                }
                
                if !horizon_responses.is_full() {
                    horizon_responses.push(shock_responses);
                }
            }
            
            if !self.impulse_responses.is_full() {
                self.impulse_responses.push(horizon_responses);
            }
        }
    }
    
    /// Рассчитать одиночный импульсный отклик
    fn calculate_single_impulse_response(&self, shock_var: usize, response_var: usize, horizon: usize) -> f64 {
        if horizon == 0 {
            // Немедленный отклик
            if shock_var == response_var {
                // Стандартное отклонение шока
                if shock_var < self.residual_covariance.len() && 
                   shock_var < self.residual_covariance[shock_var].len() {
                    return self.residual_covariance[shock_var][shock_var].sqrt();
                }
            }
            return 0.0;
        }
        
        // Упрощенный расчет отклика через прямое умножение коэффициентов
        let mut response = 0.0;
        
        for lag in 1..=self.p.min(horizon) {
            if lag <= self.coefficients.len() &&
               shock_var < self.coefficients[lag - 1].len() &&
               response_var < self.coefficients[lag - 1][shock_var].len() {
                let coeff = self.coefficients[lag - 1][shock_var][response_var];
                
                // Дисконтируем отклик по времени
                let discount_factor = 0.95_f64.powi(horizon as i32);
                response += coeff * discount_factor;
            }
        }
        
        response
    }
    
    /// Генерация прогнозов
    fn generate_forecasts(&mut self) {
        self.forecasts.clear();
        
        if !self.is_fitted || self.data.is_empty() {
            return;
        }
        
        // Прогноз на один шаг вперед для каждой переменной
        for var_idx in 0..self.n_vars {
            let mut forecast = if var_idx < self.constants.len() {
                self.constants[var_idx]
            } else {
                0.0
            };
            
            // Добавляем вклад лагов
            for lag in 0..self.p {
                for lag_var in 0..self.n_vars {
                    if self.data.len() > lag &&
                       lag < self.coefficients.len() &&
                       lag_var < self.coefficients[lag].len() &&
                       var_idx < self.coefficients[lag][lag_var].len() {
                        let data_idx = self.data.len() - 1 - lag;
                        let coeff = self.coefficients[lag][lag_var][var_idx];
                        forecast += coeff * self.data[data_idx][lag_var];
                    }
                }
            }
            
            if !self.forecasts.is_full() {
                self.forecasts.push(forecast);
            }
        }
    }
    
    /// Получить прогнозы
    pub fn forecasts(&self) -> &[f64] {
        &self.forecasts
    }
    
    /// Получить коэффициенты модели
    pub fn get_coefficients(&self) -> &[ArrayVec<ArrayVec<f64, 16>, 16>] {
        &self.coefficients
    }
    
    /// Получить ковариационную матрицу остатков
    pub fn residual_covariance(&self) -> &[ArrayVec<f64, 16>] {
        &self.residual_covariance
    }
    
    /// Получить импульсные отклики
    pub fn impulse_responses(&self) -> &[ArrayVec<ArrayVec<f64, 16>, 16>] {
        &self.impulse_responses
    }
    
    /// Получить метрики модели
    pub fn get_metrics(&self) -> (f64, f64, f64) {
        (self.log_likelihood, self.aic, self.bic)
    }
    
    /// Получить параметры модели
    pub fn get_parameters(&self) -> (usize, usize) {
        (self.p, self.n_vars)
    }
    
    /// Проверить готовность модели
    pub fn is_fitted(&self) -> bool {
        self.is_fitted
    }
    
    /// Сбросить модель
    pub fn reset(&mut self) {
        self.data.clear();
        self.coefficients.clear();
        self.constants.clear();
        self.residuals.clear();
        self.fitted_values.clear();
        self.forecasts.clear();
        self.residual_covariance.clear();
        self.impulse_responses.clear();
        self.log_likelihood = f64::NEG_INFINITY;
        self.aic = f64::INFINITY;
        self.bic = f64::INFINITY;
        self.is_fitted = false;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_fitted && self.data.len() >= self.min_observations
    }

    pub fn value(&self) -> IndicatorValue {
        // Return first forecast value if available
        if !self.forecasts.is_empty() {
            IndicatorValue::Single(self.forecasts[0])
        } else {
            IndicatorValue::Single(0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_creation() {
        let ind = Var::new(1, 2);
        assert!(!ind.is_ready());
        assert!(ind.forecasts().is_empty());
    }

    #[test]
    fn test_var_warmup() {
        let mut ind = Var::new(1, 2);
        for i in 0..50 {
            let values = [
                100.0 + (i as f64 * 0.1).sin() * 5.0,
                50.0 + (i as f64 * 0.15).cos() * 3.0,
            ];
            ind.update(&values);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_var_forecasts_finite() {
        let mut ind = Var::new(1, 2);
        for i in 0..50 {
            let values = [100.0 + i as f64 * 0.5, 50.0 + i as f64 * 0.3];
            ind.update(&values);
        }
        for &forecast in ind.forecasts() {
            assert!(forecast.is_finite());
        }
    }

    #[test]
    fn test_var_reset() {
        let mut ind = Var::new(1, 2);
        for i in 0..50 {
            let values = [100.0 + i as f64, 50.0 + i as f64 * 0.5];
            ind.update(&values);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert!(ind.forecasts().is_empty());
    }

    #[test]
    fn test_var_wrong_dimensions() {
        let mut ind = Var::new(1, 3);
        // Update with wrong number of variables (2 instead of 3)
        let result = ind.update(&[100.0, 50.0]);
        assert!(result.is_empty());
    }
} 






















