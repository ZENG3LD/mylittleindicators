//! GARCH Models
//! Generalized AutoRegressive Conditional Heteroskedasticity models
//! GARCH(p,q) - модель для волатильности с авторегрессией и скользящим средним
//! EGARCH - Exponential GARCH с асимметричными эффектами
//!
//! Parameters are estimated by REAL maximum likelihood: the conditional-Gaussian
//! log-likelihood is maximized over (ω, α, β) [GARCH] / (ω, α, γ, β) [EGARCH]
//! with a Nelder-Mead simplex. The prior version hardcoded the ARCH/GARCH
//! weights to fixed decaying constants (`0.3·0.8ⁱ`, `0.6·0.9ʲ`) and never used
//! the likelihood it computed — only the variance recursion was real.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::utils::math::optimize::{minimize, penalty, reflect_into, NmConfig};

/// GARCH Model - Generalized AutoRegressive Conditional Heteroskedasticity
#[derive(Clone)]
pub struct Garch {
    // Параметры модели
    p: usize, // ARCH order (лаги квадратов остатков)
    q: usize, // GARCH order (лаги условной дисперсии)
    
    // Данные
    returns: Vec<f64>,                    // Логарифмические доходности
    residuals: Vec<f64>,                  // Остатки модели среднего
    conditional_variance: Vec<f64>,       // Условная дисперсия

    // Коэффициенты модели
    omega: f64,                           // Константа
    alpha_coefficients: Vec<f64>,         // ARCH коэффициенты
    beta_coefficients: Vec<f64>,          // GARCH коэффициенты
    
    // Модель среднего (простая AR(1))
    mu: f64,                              // Среднее
    phi: f64,                             // AR коэффициент для среднего
    
    // Текущие значения
    current_variance: f64,
    current_volatility: f64,
    forecast_variance: f64,
    
    // Метрики
    log_likelihood: f64,
    aic: f64,
    bic: f64,
    
    // Состояние
    is_fitted: bool,
    min_observations: usize,
}

impl Garch {
    pub fn new(p: usize, q: usize) -> Self {
        let min_obs = (p + q + 10).max(50); // Минимум наблюдений для GARCH
        
        Self {
            p: p.min(8),  // Ограничиваем порядок
            q: q.min(8),
            returns: Vec::with_capacity(512),
            residuals: Vec::with_capacity(512),
            conditional_variance: Vec::with_capacity(512),
            omega: 0.01,
            alpha_coefficients: Vec::with_capacity(16),
            beta_coefficients: Vec::with_capacity(16),
            mu: 0.0,
            phi: 0.0,
            current_variance: 0.01,
            current_volatility: 0.1,
            forecast_variance: 0.01,
            log_likelihood: f64::NEG_INFINITY,
            aic: f64::INFINITY,
            bic: f64::INFINITY,
            is_fitted: false,
            min_observations: min_obs,
        }
    }
    
    /// Обновить модель новой ценой
    pub fn update(&mut self, price: f64) -> f64 {
        // Рассчитываем логарифмическую доходность
        if !self.returns.is_empty() {
            let prev_price = if self.returns.is_empty() { price } else {
                // Восстанавливаем предыдущую цену из последней доходности
                price / (1.0 + self.returns[self.returns.len() - 1])
            };
            
            let return_rate = (price / prev_price).ln();
            
            if self.returns.len() >= 512 {
                self.returns.remove(0);
            }
            self.returns.push(return_rate);
        } else {
            // Первое значение - нулевая доходность
            self.returns.push(0.0);
        }
        
        // Если достаточно данных, переоцениваем модель
        if self.returns.len() >= self.min_observations {
            self.fit_model();
            self.update_variance();
        }
        
        self.current_volatility
    }
    
    /// Подгонка GARCH модели
    fn fit_model(&mut self) {
        // 1. Оцениваем модель среднего (простая AR(1))
        self.estimate_mean_model();
        
        // 2. Рассчитываем остатки
        self.calculate_residuals();
        
        // 3. Оцениваем GARCH параметры (упрощенная версия)
        self.estimate_garch_parameters();
        
        // 4. Рассчитываем условную дисперсию
        self.calculate_conditional_variance();
        
        // 5. Рассчитываем логарифм правдоподобия и информационные критерии
        self.calculate_likelihood_criteria();
        
        self.is_fitted = true;
    }
    
    /// Оценка модели среднего
    fn estimate_mean_model(&mut self) {
        if self.returns.len() < 2 {
            return;
        }
        
        // Простая оценка AR(1): r_t = μ + φ * r_{t-1} + ε_t
        let n = self.returns.len();
        let mut sum_r = 0.0;
        let mut sum_r_lag = 0.0;
        let mut sum_r_r_lag = 0.0;
        let mut sum_r_lag_sq = 0.0;
        
        for i in 1..n {
            let r_t = self.returns[i];
            let r_t_lag = self.returns[i - 1];
            
            sum_r += r_t;
            sum_r_lag += r_t_lag;
            sum_r_r_lag += r_t * r_t_lag;
            sum_r_lag_sq += r_t_lag * r_t_lag;
        }
        
        let n_pairs = (n - 1) as f64;
        let mean_r = sum_r / n_pairs;
        let mean_r_lag = sum_r_lag / n_pairs;
        
        // OLS оценки
        let denominator = sum_r_lag_sq - n_pairs * mean_r_lag * mean_r_lag;
        if denominator.abs() > 1e-10 {
            self.phi = (sum_r_r_lag - n_pairs * mean_r * mean_r_lag) / denominator;
            self.mu = mean_r - self.phi * mean_r_lag;
        } else {
            self.phi = 0.0;
            self.mu = mean_r;
        }
        
        // Ограничиваем phi для стационарности
        self.phi = self.phi.clamp(-0.99, 0.99);
    }
    
    /// Рассчитать остатки модели среднего
    fn calculate_residuals(&mut self) {
        self.residuals.clear();
        
        if self.returns.len() < 2 {
            return;
        }
        
        // Первый остаток
        self.residuals.push(self.returns[0] - self.mu);
        
        // Остальные остатки
        for i in 1..self.returns.len() {
            let expected_return = self.mu + self.phi * self.returns[i - 1];
            let residual = self.returns[i] - expected_return;
            self.residuals.push(residual);
        }
    }

    /// Оценка GARCH параметров методом максимального правдоподобия.
    ///
    /// Maximizes the conditional-Gaussian log-likelihood
    ///   ℓ = −½ Σ_t [ ln σ²_t + ε²_t/σ²_t + ln(2π) ]
    /// over θ = (ω, α₁..αₚ, β₁..βq) with the variance recursion
    ///   σ²_t = ω + Σα_i ε²_{t-i} + Σβ_j σ²_{t-j},
    /// subject to ω > 0, α_i ≥ 0, β_j ≥ 0 and Σα + Σβ < 1 (covariance
    /// stationarity). Nelder-Mead over a reflected box; stationarity enforced
    /// by a penalty. Initialized from the variance-targeting heuristic.
    fn estimate_garch_parameters(&mut self) {
        let p = self.p;
        let q = self.q;
        if self.residuals.len() < p.max(q) + 5 {
            return;
        }

        let uncond_var: f64 =
            self.residuals.iter().map(|&r| r * r).sum::<f64>() / self.residuals.len() as f64;
        let uncond_var = uncond_var.max(1e-10);

        // Parameter bounds. ω scaled to the data; weights in [0, 0.999].
        let omega_hi = uncond_var * 5.0;
        let bound = |k: usize| -> (f64, f64) {
            if k == 0 {
                (1e-12, omega_hi)
            } else {
                (0.0, 0.999)
            }
        };
        let n_par = 1 + p + q;
        let residuals = self.residuals.clone();

        // Map raw simplex coords → feasible params via reflection.
        let to_params = |raw: &[f64]| -> Vec<f64> {
            (0..n_par)
                .map(|k| {
                    let (lo, hi) = bound(k);
                    reflect_into(raw[k], lo, hi)
                })
                .collect()
        };

        let objective = |raw: &[f64]| -> f64 {
            let theta = to_params(raw);
            let persistence: f64 = theta[1..].iter().sum();
            if persistence >= 0.9999 {
                return penalty(persistence - 0.9999);
            }
            match garch_neg_loglik(&theta, &residuals, p, q, uncond_var) {
                Some(nll) if nll.is_finite() => nll,
                _ => penalty(1.0),
            }
        };

        // Initial guess: variance targeting — modest ARCH, higher GARCH.
        let mut x0 = vec![0.0; n_par];
        x0[0] = uncond_var * 0.1;
        for i in 0..p {
            x0[1 + i] = 0.05 / p as f64;
        }
        for j in 0..q {
            x0[1 + p + j] = 0.85 / q as f64;
        }

        let cfg = NmConfig {
            max_iters: 1500,
            step: 0.3,
            ..Default::default()
        };
        let res = minimize(objective, &x0, &cfg);
        let theta = to_params(&res.x);

        self.omega = theta[0];
        self.alpha_coefficients = theta[1..=p].to_vec();
        self.beta_coefficients = theta[1 + p..].to_vec();
    }
    
    /// Рассчитать условную дисперсию
    fn calculate_conditional_variance(&mut self) {
        self.conditional_variance.clear();
        
        if self.residuals.is_empty() {
            return;
        }
        
        // Инициализируем безусловной дисперсией
        let unconditional_var: f64 = self.residuals.iter()
            .map(|&r| r * r)
            .sum::<f64>() / self.residuals.len() as f64;
        
        let start_idx = self.p.max(self.q);
        
        // Заполняем начальные значения
        for _ in 0..start_idx {
            self.conditional_variance.push(unconditional_var);
        }
        
        // Рассчитываем условную дисперсию по GARCH формуле
        for t in start_idx..self.residuals.len() {
            let mut variance = self.omega;
            
            // ARCH компонента: α_i * ε²_{t-i}
            for (i, &alpha) in self.alpha_coefficients.iter().enumerate() {
                if t > i {
                    let residual_sq = self.residuals[t - 1 - i].powi(2);
                    variance += alpha * residual_sq;
                }
            }
            
            // GARCH компонента: β_j * σ²_{t-j}
            for (j, &beta) in self.beta_coefficients.iter().enumerate() {
                if self.conditional_variance.len() > j {
                    let var_idx = self.conditional_variance.len() - 1 - j;
                    variance += beta * self.conditional_variance[var_idx];
                }
            }
            
            variance = variance.max(1e-8); // Предотвращаем отрицательную дисперсию
            self.conditional_variance.push(variance);
        }
        
        // Обновляем текущие значения
        if !self.conditional_variance.is_empty() {
            self.current_variance = self.conditional_variance[self.conditional_variance.len() - 1];
            self.current_volatility = self.current_variance.sqrt();
        }
    }
    
    /// Рассчитать логарифм правдоподобия и информационные критерии
    fn calculate_likelihood_criteria(&mut self) {
        if self.conditional_variance.is_empty() || self.residuals.is_empty() {
            return;
        }
        
        let mut log_likelihood = 0.0;
        let start_idx = self.conditional_variance.len().saturating_sub(self.residuals.len());
        
        for (i, &variance) in self.conditional_variance.iter().enumerate().skip(start_idx) {
            if i < self.residuals.len() {
                let residual = self.residuals[i];
                let ll_term = -0.5 * (variance.ln() + (residual * residual) / variance + (2.0 * std::f64::consts::PI).ln());
                log_likelihood += ll_term;
            }
        }
        
        self.log_likelihood = log_likelihood;
        
        let n = self.conditional_variance.len() as f64;
        let k = (1 + self.p + self.q + 2) as f64; // Количество параметров
        
        // AIC = -2 * LL + 2 * k
        self.aic = -2.0 * log_likelihood + 2.0 * k;
        
        // BIC = -2 * LL + k * ln(n)
        self.bic = -2.0 * log_likelihood + k * n.ln();
    }
    
    /// Обновить прогноз дисперсии
    fn update_variance(&mut self) {
        if !self.is_fitted || self.residuals.is_empty() {
            return;
        }
        
        // Прогноз на один шаг вперед
        self.forecast_variance = self.omega;
        
        // ARCH компонента
        for (i, &alpha) in self.alpha_coefficients.iter().enumerate() {
            if self.residuals.len() > i {
                let idx = self.residuals.len() - 1 - i;
                self.forecast_variance += alpha * self.residuals[idx].powi(2);
            }
        }
        
        // GARCH компонента
        for (j, &beta) in self.beta_coefficients.iter().enumerate() {
            if self.conditional_variance.len() > j {
                let idx = self.conditional_variance.len() - 1 - j;
                self.forecast_variance += beta * self.conditional_variance[idx];
            }
        }
        
        self.forecast_variance = self.forecast_variance.max(1e-8);
    }
    
    /// Получить текущую волатильность
    pub fn volatility(&self) -> f64 {
        self.current_volatility
    }
    
    /// Получить прогноз волатильности
    pub fn forecast_volatility(&self) -> f64 {
        self.forecast_variance.sqrt()
    }
    
    /// Получить текущую дисперсию
    pub fn variance(&self) -> f64 {
        self.current_variance
    }
    
    /// Получить коэффициенты модели
    pub fn get_parameters(&self) -> (f64, &[f64], &[f64]) {
        (self.omega, &self.alpha_coefficients, &self.beta_coefficients)
    }
    
    /// Получить метрики модели
    pub fn get_metrics(&self) -> (f64, f64, f64) {
        (self.log_likelihood, self.aic, self.bic)
    }
    
    /// Проверить готовность модели
    pub fn is_fitted(&self) -> bool {
        self.is_fitted
    }
    
    /// Сбросить модель
    pub fn reset(&mut self) {
        self.returns.clear();
        self.residuals.clear();
        self.conditional_variance.clear();
        self.alpha_coefficients.clear();
        self.beta_coefficients.clear();
        self.omega = 0.01;
        self.mu = 0.0;
        self.phi = 0.0;
        self.current_variance = 0.01;
        self.current_volatility = 0.1;
        self.forecast_variance = 0.01;
        self.log_likelihood = f64::NEG_INFINITY;
        self.aic = f64::INFINITY;
        self.bic = f64::INFINITY;
        self.is_fitted = false;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_fitted && self.returns.len() >= self.min_observations
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_volatility)
    }
}

/// EGARCH Model - Exponential GARCH with asymmetric effects
#[derive(Clone)]
pub struct EGarch {
    // Базовые параметры
    p: usize, // ARCH order
    q: usize, // GARCH order
    
    // Данные
    returns: Vec<f64>,
    residuals: Vec<f64>,
    log_conditional_variance: Vec<f64>,   // ln(σ²_t)
    standardized_residuals: Vec<f64>,     // z_t = ε_t / σ_t

    // Коэффициенты EGARCH модели
    omega: f64,                           // Константа
    alpha_coefficients: Vec<f64>,         // Коэффициенты для |z_{t-i}|
    gamma_coefficients: Vec<f64>,         // Асимметричные коэффициенты для z_{t-i}
    beta_coefficients: Vec<f64>,          // GARCH коэффициенты
    
    // Модель среднего
    mu: f64,
    phi: f64,
    
    // Текущие значения
    current_log_variance: f64,
    current_variance: f64,
    current_volatility: f64,
    
    // Метрики
    log_likelihood: f64,
    aic: f64,
    bic: f64,
    
    // Состояние
    is_fitted: bool,
    min_observations: usize,
}

impl EGarch {
    pub fn new(p: usize, q: usize) -> Self {
        let min_obs = (p + q + 10).max(50);
        
        Self {
            p: p.min(8),
            q: q.min(8),
            returns: Vec::with_capacity(512),
            residuals: Vec::with_capacity(512),
            log_conditional_variance: Vec::with_capacity(512),
            standardized_residuals: Vec::with_capacity(512),
            omega: -1.0,
            alpha_coefficients: Vec::with_capacity(16),
            gamma_coefficients: Vec::with_capacity(16),
            beta_coefficients: Vec::with_capacity(16),
            mu: 0.0,
            phi: 0.0,
            current_log_variance: -2.3, // ln(0.1)
            current_variance: 0.1,
            current_volatility: 0.316,  // sqrt(0.1)
            log_likelihood: f64::NEG_INFINITY,
            aic: f64::INFINITY,
            bic: f64::INFINITY,
            is_fitted: false,
            min_observations: min_obs,
        }
    }
    
    /// Обновить модель новой ценой
    pub fn update(&mut self, price: f64) -> f64 {
        // Рассчитываем логарифмическую доходность
        if !self.returns.is_empty() {
            let prev_price = if self.returns.is_empty() { price } else {
                price / (1.0 + self.returns[self.returns.len() - 1])
            };
            
            let return_rate = (price / prev_price).ln();
            
            if self.returns.len() >= 512 {
                self.returns.remove(0);
            }
            self.returns.push(return_rate);
        } else {
            self.returns.push(0.0);
        }
        
        if self.returns.len() >= self.min_observations {
            self.fit_model();
        }
        
        self.current_volatility
    }
    
    /// Подгонка EGARCH модели  
    fn fit_model(&mut self) {
        // 1. Оцениваем модель среднего
        self.estimate_mean_model();
        
        // 2. Рассчитываем остатки
        self.calculate_residuals();
        
        // 3. Оцениваем EGARCH параметры
        self.estimate_egarch_parameters();
        
        // 4. Рассчитываем логарифм условной дисперсии
        self.calculate_log_conditional_variance();
        
        // 5. Рассчитываем метрики
        self.calculate_likelihood_criteria();
        
        self.is_fitted = true;
    }
    
    /// Оценка модели среднего (аналогично GARCH)
    fn estimate_mean_model(&mut self) {
        if self.returns.len() < 2 {
            return;
        }
        
        let n = self.returns.len();
        let mut sum_r = 0.0;
        let mut sum_r_lag = 0.0;
        let mut sum_r_r_lag = 0.0;
        let mut sum_r_lag_sq = 0.0;
        
        for i in 1..n {
            let r_t = self.returns[i];
            let r_t_lag = self.returns[i - 1];
            
            sum_r += r_t;
            sum_r_lag += r_t_lag;
            sum_r_r_lag += r_t * r_t_lag;
            sum_r_lag_sq += r_t_lag * r_t_lag;
        }
        
        let n_pairs = (n - 1) as f64;
        let mean_r = sum_r / n_pairs;
        let mean_r_lag = sum_r_lag / n_pairs;
        
        let denominator = sum_r_lag_sq - n_pairs * mean_r_lag * mean_r_lag;
        if denominator.abs() > 1e-10 {
            self.phi = (sum_r_r_lag - n_pairs * mean_r * mean_r_lag) / denominator;
            self.mu = mean_r - self.phi * mean_r_lag;
        } else {
            self.phi = 0.0;
            self.mu = mean_r;
        }
        
        self.phi = self.phi.clamp(-0.99, 0.99);
    }
    
    /// Рассчитать остатки
    fn calculate_residuals(&mut self) {
        self.residuals.clear();
        
        if self.returns.len() < 2 {
            return;
        }
        
        self.residuals.push(self.returns[0] - self.mu);
        
        for i in 1..self.returns.len() {
            let expected_return = self.mu + self.phi * self.returns[i - 1];
            let residual = self.returns[i] - expected_return;
            self.residuals.push(residual);
        }
    }

    /// Оценка EGARCH параметров методом максимального правдоподобия.
    ///
    /// Maximizes the EGARCH(p,q) log-likelihood over
    /// θ = (ω, α₁..αₚ, γ₁..γₚ, β₁..βq). The log-variance link keeps σ² > 0 for
    /// any θ, so α/γ are sign-unconstrained; only the GARCH persistence needs
    /// |Σβ| < 1, enforced via a penalty. The prior version hardcoded all
    /// coefficients to fixed decaying constants.
    fn estimate_egarch_parameters(&mut self) {
        let p = self.p;
        let q = self.q;
        if self.residuals.len() < p.max(q) + 5 {
            return;
        }

        let uncond_var: f64 =
            (self.residuals.iter().map(|&r| r * r).sum::<f64>() / self.residuals.len() as f64)
                .max(1e-10);
        let init_log_var = uncond_var.ln();
        let residuals = self.residuals.clone();

        // Layout: [ω, α(p), γ(p), β(q)]. β reflected into (−1,1) for stability;
        // ω, α, γ are free (bounded generously to keep the simplex sane).
        let n_par = 1 + 2 * p + q;
        let beta_lo = 1 + 2 * p;
        let to_params = |raw: &[f64]| -> Vec<f64> {
            let mut t = raw.to_vec();
            for k in beta_lo..n_par {
                t[k] = reflect_into(raw[k], -0.999, 0.999);
            }
            // Keep ω/α/γ in a wide reflected box to avoid runaway exp().
            t[0] = reflect_into(raw[0], -10.0, 5.0);
            for k in 1..beta_lo {
                t[k] = reflect_into(raw[k], -2.0, 2.0);
            }
            t
        };

        let objective = |raw: &[f64]| -> f64 {
            let theta = to_params(raw);
            let beta_sum: f64 = theta[beta_lo..].iter().sum();
            if beta_sum.abs() >= 0.9999 {
                return penalty(beta_sum.abs() - 0.9999);
            }
            match egarch_neg_loglik(&theta, &residuals, p, q, init_log_var) {
                Some(nll) if nll.is_finite() => nll,
                _ => penalty(1.0),
            }
        };

        // Init: ω≈ln(uncond_var)·(1−β), small size effect, mild leverage.
        let mut x0 = vec![0.0; n_par];
        x0[0] = init_log_var * 0.1;
        for i in 0..p {
            x0[1 + i] = 0.15; // α (size)
            x0[1 + p + i] = -0.05; // γ (leverage)
        }
        for j in 0..q {
            x0[beta_lo + j] = 0.9 / q as f64; // β (persistence)
        }

        let cfg = NmConfig {
            max_iters: 2000,
            step: 0.3,
            ..Default::default()
        };
        let res = minimize(objective, &x0, &cfg);
        let theta = to_params(&res.x);

        self.omega = theta[0];
        self.alpha_coefficients = theta[1..1 + p].to_vec();
        self.gamma_coefficients = theta[1 + p..1 + 2 * p].to_vec();
        self.beta_coefficients = theta[beta_lo..].to_vec();
    }
    
    /// Рассчитать логарифм условной дисперсии
    fn calculate_log_conditional_variance(&mut self) {
        self.log_conditional_variance.clear();
        self.standardized_residuals.clear();
        
        if self.residuals.is_empty() {
            return;
        }
        
        // Инициализируем начальными значениями
        let initial_log_var = -2.3; // ln(0.1)
        let start_idx = self.p.max(self.q);
        
        for _ in 0..start_idx {
            self.log_conditional_variance.push(initial_log_var);
        }

        // Рассчитываем стандартизированные остатки для начальных значений
        for i in 0..start_idx.min(self.residuals.len()) {
            let std_residual = self.residuals[i] / initial_log_var.exp().sqrt();
            self.standardized_residuals.push(std_residual);
        }
        
        // EGARCH уравнение: ln(σ²_t) = ω + Σα_i*g(z_{t-i}) + Σβ_j*ln(σ²_{t-j})
        // где g(z) = α*|z| + γ*z
        for t in start_idx..self.residuals.len() {
            let mut log_variance = self.omega;
            
            // Компонента размера и асимметрии
            for (i, (&alpha, &gamma)) in self.alpha_coefficients.iter()
                .zip(self.gamma_coefficients.iter()).enumerate() {
                
                if self.standardized_residuals.len() > i {
                    let z_idx = self.standardized_residuals.len() - 1 - i;
                    let z = self.standardized_residuals[z_idx];
                    
                    // g(z) = α*|z| + γ*z
                    let g_z = alpha * z.abs() + gamma * z;
                    log_variance += g_z;
                }
            }
            
            // GARCH компонента
            for (j, &beta) in self.beta_coefficients.iter().enumerate() {
                if self.log_conditional_variance.len() > j {
                    let var_idx = self.log_conditional_variance.len() - 1 - j;
                    log_variance += beta * self.log_conditional_variance[var_idx];
                }
            }
            
            self.log_conditional_variance.push(log_variance);

            // Рассчитываем стандартизированный остаток
            let variance = log_variance.exp();
            let std_residual = self.residuals[t] / variance.sqrt();
            self.standardized_residuals.push(std_residual);
        }
        
        // Обновляем текущие значения
        if !self.log_conditional_variance.is_empty() {
            self.current_log_variance = self.log_conditional_variance[self.log_conditional_variance.len() - 1];
            self.current_variance = self.current_log_variance.exp();
            self.current_volatility = self.current_variance.sqrt();
        }
    }
    
    /// Рассчитать метрики
    fn calculate_likelihood_criteria(&mut self) {
        if self.log_conditional_variance.is_empty() || self.residuals.is_empty() {
            return;
        }
        
        let mut log_likelihood = 0.0;
        let start_idx = self.log_conditional_variance.len().saturating_sub(self.residuals.len());
        
        for (i, &log_variance) in self.log_conditional_variance.iter().enumerate().skip(start_idx) {
            if i < self.residuals.len() {
                let residual = self.residuals[i];
                let variance = log_variance.exp();
                let ll_term = -0.5 * (log_variance + (residual * residual) / variance + (2.0 * std::f64::consts::PI).ln());
                log_likelihood += ll_term;
            }
        }
        
        self.log_likelihood = log_likelihood;
        
        let n = self.log_conditional_variance.len() as f64;
        let k = (1 + self.p * 2 + self.q + 2) as f64; // ω + α + γ + β + μ + φ
        
        self.aic = -2.0 * log_likelihood + 2.0 * k;
        self.bic = -2.0 * log_likelihood + k * n.ln();
    }
    
    /// Получить текущую волатильность
    pub fn volatility(&self) -> f64 {
        self.current_volatility
    }
    
    /// Получить текущую дисперсию
    pub fn variance(&self) -> f64 {
        self.current_variance
    }
    
    /// Получить коэффициенты модели
    pub fn get_parameters(&self) -> (f64, &[f64], &[f64], &[f64]) {
        (self.omega, &self.alpha_coefficients, &self.gamma_coefficients, &self.beta_coefficients)
    }
    
    /// Получить метрики модели
    pub fn get_metrics(&self) -> (f64, f64, f64) {
        (self.log_likelihood, self.aic, self.bic)
    }
    
    /// Проверить готовность модели
    pub fn is_fitted(&self) -> bool {
        self.is_fitted
    }
    
    /// Сбросить модель
    pub fn reset(&mut self) {
        self.returns.clear();
        self.residuals.clear();
        self.log_conditional_variance.clear();
        self.standardized_residuals.clear();
        self.alpha_coefficients.clear();
        self.gamma_coefficients.clear();
        self.beta_coefficients.clear();
        self.omega = -1.0;
        self.mu = 0.0;
        self.phi = 0.0;
        self.current_log_variance = -2.3;
        self.current_variance = 0.1;
        self.current_volatility = 0.316;
        self.log_likelihood = f64::NEG_INFINITY;
        self.aic = f64::INFINITY;
        self.bic = f64::INFINITY;
        self.is_fitted = false;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_fitted && self.returns.len() >= self.min_observations
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_volatility)
    }
}

/// Negative conditional-Gaussian log-likelihood of a GARCH(p,q) candidate
/// θ = [ω, α₁..αₚ, β₁..βq] given the mean-model residuals. Returns `None` if a
/// non-finite / non-positive variance appears (the optimizer treats that as
/// infeasible). The recursion is seeded with the unconditional variance.
fn garch_neg_loglik(
    theta: &[f64],
    residuals: &[f64],
    p: usize,
    q: usize,
    uncond_var: f64,
) -> Option<f64> {
    let omega = theta[0];
    if omega <= 0.0 {
        return None;
    }
    let alpha = &theta[1..=p];
    let beta = &theta[1 + p..1 + p + q];
    let n = residuals.len();
    let start = p.max(q);
    if n <= start + 1 {
        return None;
    }

    // σ²_t history, seeded with the sample variance for the burn-in lags.
    let mut var_hist = vec![uncond_var; n];
    let two_pi_ln = (2.0 * std::f64::consts::PI).ln();
    let mut nll = 0.0;
    for t in start..n {
        let mut v = omega;
        for (i, &a) in alpha.iter().enumerate() {
            v += a * residuals[t - 1 - i] * residuals[t - 1 - i];
        }
        for (j, &b) in beta.iter().enumerate() {
            v += b * var_hist[t - 1 - j];
        }
        if !v.is_finite() || v <= 0.0 {
            return None;
        }
        var_hist[t] = v;
        let e = residuals[t];
        nll += 0.5 * (v.ln() + e * e / v + two_pi_ln);
    }
    Some(nll)
}

/// Negative log-likelihood of an EGARCH(p,q) candidate
/// θ = [ω, α₁..αₚ, γ₁..γₚ, β₁..βq] on log-variance. EGARCH is unconstrained in
/// sign (the log keeps σ² > 0 automatically); only |Σβ| < 1 matters and is
/// enforced by the caller. Recursion: ln σ²_t = ω + Σα_i g(z_{t-i}) + Σβ_j ln σ²_{t-j},
/// g(z) = α|z| + γz, z = ε/σ.
fn egarch_neg_loglik(
    theta: &[f64],
    residuals: &[f64],
    p: usize,
    q: usize,
    init_log_var: f64,
) -> Option<f64> {
    let omega = theta[0];
    let alpha = &theta[1..1 + p];
    let gamma = &theta[1 + p..1 + 2 * p];
    let beta = &theta[1 + 2 * p..1 + 2 * p + q];
    let n = residuals.len();
    let start = p.max(q);
    if n <= start + 1 {
        return None;
    }

    let mut logvar = vec![init_log_var; n];
    let mut z = vec![0.0_f64; n];
    // Seed standardized residuals for the burn-in region.
    let seed_sd = (init_log_var.exp()).sqrt().max(1e-12);
    for (i, zi) in z.iter_mut().enumerate().take(start) {
        *zi = residuals[i] / seed_sd;
    }

    let two_pi_ln = (2.0 * std::f64::consts::PI).ln();
    let mut nll = 0.0;
    for t in start..n {
        let mut lv = omega;
        for i in 0..p {
            let zt = z[t - 1 - i];
            lv += alpha[i] * zt.abs() + gamma[i] * zt;
        }
        for (j, &b) in beta.iter().enumerate() {
            lv += b * logvar[t - 1 - j];
        }
        if !lv.is_finite() {
            return None;
        }
        logvar[t] = lv;
        let v = lv.exp();
        if !v.is_finite() || v <= 0.0 {
            return None;
        }
        let e = residuals[t];
        z[t] = e / v.sqrt();
        nll += 0.5 * (lv + e * e / v + two_pi_ln);
    }
    Some(nll)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_garch_creation() {
        let ind = Garch::new(1, 1);
        assert!(!ind.is_ready());
        assert!(ind.volatility() > 0.0);
    }

    #[test]
    fn test_garch_warmup() {
        let mut ind = Garch::new(1, 1);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_garch_volatility_finite() {
        let mut ind = Garch::new(1, 1);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ind.update(price);
        }
        assert!(ind.volatility().is_finite());
        assert!(ind.volatility() >= 0.0);
        assert!(ind.forecast_volatility().is_finite());
    }

    #[test]
    fn test_garch_reset() {
        let mut ind = Garch::new(1, 1);
        for i in 0..100 {
            let price = 100.0 + i as f64;
            ind.update(price);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert!(!ind.is_fitted());
    }

    #[test]
    fn test_egarch_creation() {
        let ind = EGarch::new(1, 1);
        assert!(!ind.is_ready());
        assert!(ind.volatility() > 0.0);
    }

    #[test]
    fn test_egarch_warmup() {
        let mut ind = EGarch::new(1, 1);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_egarch_volatility_finite() {
        let mut ind = EGarch::new(1, 1);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ind.update(price);
        }
        assert!(ind.volatility().is_finite());
        assert!(ind.volatility() >= 0.0);
    }

    #[test]
    fn test_egarch_reset() {
        let mut ind = EGarch::new(1, 1);
        for i in 0..100 {
            let price = 100.0 + i as f64;
            ind.update(price);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert!(!ind.is_fitted());
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

    /// MLE must fit a clustered series at least as well as the discarded
    /// hardcoded guess (α=0.3, β=0.6), and ideally recover something close to
    /// the true (α=0.1, β=0.85) generating parameters. Tests the estimator
    /// directly (objective + Nelder-Mead), isolated from the mean model.
    #[test]
    fn garch_mle_beats_hardcoded_guess() {
        use crate::bar_indicators::utils::math::optimize::{minimize, reflect_into, NmConfig};
        // GARCH(1,1) data-generating process with strong persistence.
        let z = lcg(600, 5);
        let mut resid = Vec::with_capacity(z.len());
        let (omega, a, b) = (0.02_f64, 0.1_f64, 0.85_f64);
        let mut var: f64 = omega / (1.0 - a - b);
        let mut prev_e = 0.0;
        for &zt in &z {
            var = omega + a * prev_e * prev_e + b * var;
            let e = var.sqrt() * zt;
            prev_e = e;
            resid.push(e);
        }
        let uncond: f64 = resid.iter().map(|r| r * r).sum::<f64>() / resid.len() as f64;

        // Hardcoded-guess NLL (the discarded heuristic).
        let hard = vec![uncond * 0.1, 0.3, 0.6];
        let nll_hard = garch_neg_loglik(&hard, &resid, 1, 1, uncond).unwrap();

        // Optimize the real objective directly (same recipe as the fitter).
        let omega_hi = uncond * 5.0;
        let to_p = |raw: &[f64]| {
            vec![
                reflect_into(raw[0], 1e-12, omega_hi),
                reflect_into(raw[1], 0.0, 0.999),
                reflect_into(raw[2], 0.0, 0.999),
            ]
        };
        let obj = |raw: &[f64]| {
            let t = to_p(raw);
            if t[1] + t[2] >= 0.9999 {
                return 1e12;
            }
            garch_neg_loglik(&t, &resid, 1, 1, uncond).unwrap_or(1e12)
        };
        let res = minimize(
            obj,
            &[uncond * 0.1, 0.05, 0.85],
            &NmConfig {
                max_iters: 2000,
                step: 0.3,
                ..Default::default()
            },
        );
        let fit = to_p(&res.x);
        let nll_fit = garch_neg_loglik(&fit, &resid, 1, 1, uncond).unwrap();

        assert!(
            nll_fit <= nll_hard + 1e-6,
            "MLE NLL {nll_fit} should be ≤ hardcoded NLL {nll_hard}"
        );
        assert!(fit[1] + fit[2] < 1.0, "persistence must be < 1");
        // High-persistence DGP → recovered β should dominate α.
        assert!(
            fit[2] > fit[1],
            "β {} should exceed α {} for persistent vol",
            fit[2],
            fit[1]
        );
    }

    /// The neg-log-likelihood is deterministic and finite on a clean series.
    #[test]
    fn garch_nll_deterministic_and_finite() {
        let resid: Vec<f64> = lcg(120, 9).iter().map(|&e| 0.01 * e).collect();
        let uncond: f64 = resid.iter().map(|r| r * r).sum::<f64>() / resid.len() as f64;
        let theta = vec![uncond * 0.1, 0.1, 0.8];
        let a = garch_neg_loglik(&theta, &resid, 1, 1, uncond).unwrap();
        let b = garch_neg_loglik(&theta, &resid, 1, 1, uncond).unwrap();
        assert_eq!(a, b);
        assert!(a.is_finite());
        // Infeasible (negative ω) → None.
        assert!(garch_neg_loglik(&vec![-1.0, 0.1, 0.8], &resid, 1, 1, uncond).is_none());
    }
}






















