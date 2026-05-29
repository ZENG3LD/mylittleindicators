//! VAR Model - Vector AutoRegression
//! Векторная авторегрессионная модель для анализа множественных временных рядов
//! VAR(p) - p лагов для каждой переменной во всех уравнениях
//!
//! REAL implementation. Three corrections over the prior version:
//! 1. Per-equation coefficients via the shared `linalg::ols` (full Gaussian
//!    elimination + pivot) instead of the diagonal-only normal-equations hack
//!    (`xty[i]/xtx[i][i]`) that ignored regressor correlation entirely.
//! 2. The Gaussian log-likelihood uses the TRUE determinant of Σ
//!    (`linalg::determinant`), not the product of its diagonal.
//! 3. Impulse responses are the genuine orthogonalized VMA(∞) responses —
//!    Ψ_h = Σ_{i=1..min(h,p)} A_i Ψ_{h-i}, Ψ_0 = I, post-multiplied by the
//!    Cholesky factor of Σ for a structural (one-S.D.) shock — replacing the
//!    invented `coeff · 0.95^h` discount.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::utils::math::linalg::{cholesky, determinant, ols};

/// VAR Model - Vector AutoRegression
#[derive(Clone)]
pub struct Var {
    // Параметры модели
    p: usize,                    // Порядок VAR (количество лагов)
    n_vars: usize,              // Количество переменных
    
    // Данные
    data: Vec<Vec<f64>>,                 // Матрица данных [time x variables]

    // Коэффициенты модели - матрица коэффициентов для каждого лага
    // coefficients[lag][var_from][var_to] = коэффициент влияния var_from на var_to с лагом lag
    coefficients: Vec<Vec<Vec<f64>>>,    // [lag][from_var][to_var]
    constants: Vec<f64>,                 // Константы для каждого уравнения

    // Остатки и прогнозы
    residuals: Vec<Vec<f64>>,            // Остатки для каждой переменной
    fitted_values: Vec<Vec<f64>>,        // Подогнанные значения
    forecasts: Vec<f64>,                 // Прогнозы для каждой переменной

    // Ковариационная матрица остатков
    residual_covariance: Vec<Vec<f64>>,  // Σ matrix

    // Метрики модели
    log_likelihood: f64,
    aic: f64,
    bic: f64,

    // Импульсные отклики (упрощенная версия)
    impulse_responses: Vec<Vec<Vec<f64>>>, // [horizon][shock_var][response_var]
    
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
            data: Vec::with_capacity(512),
            coefficients: Vec::with_capacity(8),
            constants: Vec::with_capacity(16),
            residuals: Vec::with_capacity(512),
            fitted_values: Vec::with_capacity(512),
            forecasts: Vec::with_capacity(16),
            residual_covariance: Vec::with_capacity(16),
            log_likelihood: f64::NEG_INFINITY,
            aic: f64::INFINITY,
            bic: f64::INFINITY,
            impulse_responses: Vec::with_capacity(20),
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
        let mut data_row: Vec<f64> = Vec::with_capacity(self.n_vars);
        for &val in values.iter().take(self.n_vars) {
            data_row.push(val);
        }

        if self.data.len() >= 512 {
            self.data.remove(0);
        }
        self.data.push(data_row);
        
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
            let mut lag_coeffs: Vec<Vec<f64>> = Vec::with_capacity(self.n_vars);
            for _ in 0..self.n_vars {
                lag_coeffs.push(vec![0.0; self.n_vars]);
            }
            self.coefficients.push(lag_coeffs);
        }

        // Инициализируем константы
        for _ in 0..self.n_vars {
            self.constants.push(0.0);
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
    
    /// OLS for one VAR equation via the shared full solver. Flattens the dense
    /// design to row-major and delegates to `linalg::ols` (proper (XᵀX)⁻¹Xᵀy).
    fn solve_ols(&self, x_matrix: &[Vec<f64>], y_vector: &[f64]) -> Vec<f64> {
        if x_matrix.is_empty() || y_vector.is_empty() {
            return Vec::new();
        }
        let rows = x_matrix.len();
        let k = x_matrix[0].len();
        let mut flat = Vec::with_capacity(rows * k);
        for row in &x_matrix[..rows] {
            flat.extend_from_slice(&row[..k]);
        }
        ols(&flat, &y_vector[..rows], rows, k).unwrap_or_else(|| vec![0.0; k])
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
            let mut fitted_row: Vec<f64> = Vec::with_capacity(self.n_vars);
            let mut residual_row: Vec<f64> = Vec::with_capacity(self.n_vars);

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

                fitted_row.push(fitted_value);
                residual_row.push(residual);
            }

            self.fitted_values.push(fitted_row);
            self.residuals.push(residual_row);
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
            let mut cov_row: Vec<f64> = Vec::with_capacity(self.n_vars);
            for j in 0..self.n_vars {
                let mut covariance = 0.0;

                // Рассчитываем ковариацию между переменными i и j
                for residual_row in &self.residuals {
                    if i < residual_row.len() && j < residual_row.len() {
                        covariance += residual_row[i] * residual_row[j];
                    }
                }

                covariance /= n_obs;
                cov_row.push(covariance);
            }
            self.residual_covariance.push(cov_row);
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
    
    /// Определитель ковариационной матрицы остатков (полный LU determinant).
    fn calculate_determinant(&self, matrix: &[Vec<f64>]) -> f64 {
        let n = self.n_vars;
        if matrix.len() < n {
            return 0.0;
        }
        let mut flat = Vec::with_capacity(n * n);
        for row in matrix.iter().take(n) {
            if row.len() < n {
                return 0.0;
            }
            flat.extend_from_slice(&row[..n]);
        }
        determinant(&flat, n).unwrap_or(0.0)
    }
    
    /// Рассчитать ортогонализованные импульсные отклики (настоящий VMA(∞)).
    ///
    /// The reduced-form VMA(∞) responses obey Ψ₀ = I,
    /// Ψ_h = Σ_{i=1..min(h,p)} A_i · Ψ_{h−i}, where A_i is the lag-i coefficient
    /// matrix in standard orientation (`A_i[to][from]`). For a structural
    /// (one-standard-deviation, Cholesky-orthogonalized) shock we post-multiply
    /// by P = chol(Σ): Θ_h = Ψ_h · P. We store
    /// `impulse_responses[h][shock][response] = Θ_h[response][shock]`.
    fn calculate_impulse_responses(&mut self) {
        self.impulse_responses.clear();
        let n = self.n_vars;
        let max_horizon = 20;
        if n == 0 || self.coefficients.is_empty() {
            return;
        }

        // Lag matrices A_i as flat row-major n×n, A_i[to][from].
        let a: Vec<Vec<f64>> = (0..self.p)
            .map(|lag| {
                let mut m = vec![0.0; n * n];
                for from in 0..n {
                    for to in 0..n {
                        m[to * n + from] = self.coefficients[lag][from][to];
                    }
                }
                m
            })
            .collect();

        // Cholesky factor P of Σ (lower-triangular). Fall back to a diagonal of
        // residual standard deviations if Σ is not numerically PD.
        let sigma_flat: Vec<f64> = {
            let mut s = vec![0.0; n * n];
            for i in 0..n {
                for j in 0..n {
                    s[i * n + j] = self.residual_covariance[i][j];
                }
            }
            s
        };
        let p_chol = cholesky(&sigma_flat, n).unwrap_or_else(|| {
            let mut d = vec![0.0; n * n];
            for i in 0..n {
                d[i * n + i] = self.residual_covariance[i][i].max(0.0).sqrt();
            }
            d
        });

        // Ψ history of flat n×n matrices. Ψ₀ = I.
        let mut psi: Vec<Vec<f64>> = Vec::with_capacity(max_horizon);
        let mut psi0 = vec![0.0; n * n];
        for i in 0..n {
            psi0[i * n + i] = 1.0;
        }
        psi.push(psi0);

        for h in 1..max_horizon {
            let mut psi_h = vec![0.0; n * n];
            for i in 1..=self.p.min(h) {
                let ai = &a[i - 1];
                let prev = &psi[h - i];
                // psi_h += A_i · prev
                for r in 0..n {
                    for c in 0..n {
                        let mut acc = 0.0;
                        for k in 0..n {
                            acc += ai[r * n + k] * prev[k * n + c];
                        }
                        psi_h[r * n + c] += acc;
                    }
                }
            }
            psi.push(psi_h);
        }

        // Θ_h = Ψ_h · P, then reorder to [shock][response].
        for psi_h in psi.iter().take(max_horizon) {
            let mut theta = vec![0.0; n * n];
            for r in 0..n {
                for c in 0..n {
                    let mut acc = 0.0;
                    for k in 0..n {
                        acc += psi_h[r * n + k] * p_chol[k * n + c];
                    }
                    theta[r * n + c] = acc;
                }
            }
            // impulse_responses[h][shock][response] = Θ_h[response][shock].
            let mut horizon: Vec<Vec<f64>> = Vec::with_capacity(n);
            for shock in 0..n {
                let mut row = Vec::with_capacity(n);
                for response in 0..n {
                    row.push(theta[response * n + shock]);
                }
                horizon.push(row);
            }
            self.impulse_responses.push(horizon);
        }
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
            
            self.forecasts.push(forecast);
        }
    }
    
    /// Получить прогнозы
    pub fn forecasts(&self) -> &[f64] {
        &self.forecasts
    }
    
    /// Получить коэффициенты модели
    pub fn get_coefficients(&self) -> &[Vec<Vec<f64>>] {
        &self.coefficients
    }

    /// Получить ковариационную матрицу остатков
    pub fn residual_covariance(&self) -> &[Vec<f64>] {
        &self.residual_covariance
    }

    /// Получить импульсные отклики
    pub fn impulse_responses(&self) -> &[Vec<Vec<f64>>] {
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

    fn lcg(n: usize, seed: u64) -> Vec<f64> {
        let mut s = seed;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            s = s
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            out.push(((s >> 33) as f64) / (1u64 << 31) as f64 - 1.0);
        }
        out
    }

    /// Full OLS must recover the cross-equation coefficient (y₁ → y₂) that the
    /// old diagonal-only solver would badly bias. VAR(1), 2 vars:
    ///   y₁_t = 0.5 y₁_{t-1} + e₁
    ///   y₂_t = 0.4 y₁_{t-1} + 0.3 y₂_{t-1} + e₂
    /// coefficients[lag=0][from][to]: [0][0][1] is the 0.4 cross term.
    #[test]
    fn ols_recovers_cross_equation_coefficient() {
        let e1 = lcg(800, 31);
        let e2 = lcg(800, 67);
        let mut y1 = vec![0.0_f64; e1.len()];
        let mut y2 = vec![0.0_f64; e2.len()];
        for t in 1..e1.len() {
            y1[t] = 0.5 * y1[t - 1] + e1[t];
            y2[t] = 0.4 * y1[t - 1] + 0.3 * y2[t - 1] + e2[t];
        }
        let mut v = Var::new(1, 2);
        for t in 0..y1.len() {
            v.update(&[y1[t], y2[t]]);
        }
        let c = v.get_coefficients();
        assert_eq!(c.len(), 1);
        // own lag of y₁.
        assert!((c[0][0][0] - 0.5).abs() < 0.1, "φ₁₁ {} ≈ 0.5", c[0][0][0]);
        // cross: y₁ → y₂.
        assert!((c[0][0][1] - 0.4).abs() < 0.1, "cross {} ≈ 0.4", c[0][0][1]);
        // own lag of y₂.
        assert!((c[0][1][1] - 0.3).abs() < 0.1, "φ₂₂ {} ≈ 0.3", c[0][1][1]);
        // y₂ does not feed back into y₁ → coefficient ≈ 0.
        assert!(c[0][1][0].abs() < 0.1, "no feedback {} ≈ 0", c[0][1][0]);
    }

    /// The horizon-0 orthogonalized impulse response equals the Cholesky factor
    /// of Σ: a unit structural shock to variable j moves variable i on impact
    /// by P[i][j], lower-triangular (P[i][j]=0 for i<j). We check the impact
    /// matrix is lower-triangular in [response][shock] terms.
    #[test]
    fn impulse_response_impact_is_cholesky() {
        let e1 = lcg(600, 5);
        let e2 = lcg(600, 9);
        let mut y1 = vec![0.0_f64; e1.len()];
        let mut y2 = vec![0.0_f64; e2.len()];
        for t in 1..e1.len() {
            y1[t] = 0.3 * y1[t - 1] + e1[t];
            // Correlate the innovations so Σ is non-diagonal.
            y2[t] = 0.2 * y2[t - 1] + e2[t] + 0.5 * e1[t];
        }
        let mut v = Var::new(1, 2);
        for t in 0..y1.len() {
            v.update(&[y1[t], y2[t]]);
        }
        let irf = v.impulse_responses();
        assert!(!irf.is_empty());
        // irf[h=0][shock][response]. A shock to var1 (index1) must NOT move
        // var0 on impact (Cholesky lower-triangular → upper entry zero).
        let shock1_response0 = irf[0][1][0];
        assert!(
            shock1_response0.abs() < 1e-9,
            "impact of shock-to-1 on var-0 must be 0 (lower-tri), got {shock1_response0}"
        );
        // The own-impact of shock-to-0 on var0 = sqrt(Σ₀₀) > 0.
        assert!(irf[0][0][0] > 0.0, "own impact must be positive");
    }
}






















