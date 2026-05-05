//! GARCH Models
//! Generalized AutoRegressive Conditional Heteroskedasticity models
//! GARCH(p,q) - модель для волатильности с авторегрессией и скользящим средним
//! EGARCH - Exponential GARCH с асимметричными эффектами

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// GARCH Model - Generalized AutoRegressive Conditional Heteroskedasticity
#[derive(Clone)]
pub struct Garch {
    // Параметры модели
    p: usize, // ARCH order (лаги квадратов остатков)
    q: usize, // GARCH order (лаги условной дисперсии)
    
    // Данные
    returns: ArrayVec<f64, 512>,          // Логарифмические доходности
    residuals: ArrayVec<f64, 512>,        // Остатки модели среднего
    conditional_variance: ArrayVec<f64, 512>, // Условная дисперсия
    
    // Коэффициенты модели
    omega: f64,                           // Константа
    alpha_coefficients: ArrayVec<f64, 16>, // ARCH коэффициенты
    beta_coefficients: ArrayVec<f64, 16>,  // GARCH коэффициенты
    
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
            returns: ArrayVec::new(),
            residuals: ArrayVec::new(),
            conditional_variance: ArrayVec::new(),
            omega: 0.01,
            alpha_coefficients: ArrayVec::new(),
            beta_coefficients: ArrayVec::new(),
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
            if !self.residuals.is_full() {
                self.residuals.push(residual);
            }
        }
    }
    
    /// Оценка GARCH параметров (упрощенная версия)
    fn estimate_garch_parameters(&mut self) {
        if self.residuals.len() < self.p.max(self.q) + 5 {
            return;
        }
        
        // Инициализируем коэффициенты
        self.alpha_coefficients.clear();
        self.beta_coefficients.clear();
        
        // Простая эвристическая оценка параметров
        // В реальной реализации здесь был бы алгоритм максимального правдоподобия
        
        // Оцениваем безусловную дисперсию
        let unconditional_var: f64 = self.residuals.iter()
            .map(|&r| r * r)
            .sum::<f64>() / self.residuals.len() as f64;
        
        // Простые начальные значения
        self.omega = unconditional_var * 0.1;
        
        // ARCH коэффициенты (убывающие)
        let total_arch_weight = 0.3;
        for i in 0..self.p {
            let weight = total_arch_weight * (0.8_f64).powi(i as i32);
            if !self.alpha_coefficients.is_full() {
                self.alpha_coefficients.push(weight);
            }
        }
        
        // GARCH коэффициенты (убывающие)
        let total_garch_weight = 0.6;
        for i in 0..self.q {
            let weight = total_garch_weight * (0.9_f64).powi(i as i32);
            if !self.beta_coefficients.is_full() {
                self.beta_coefficients.push(weight);
            }
        }
        
        // Нормализация для обеспечения стационарности
        let total_persistence: f64 = self.alpha_coefficients.iter().sum::<f64>() + 
                                    self.beta_coefficients.iter().sum::<f64>();
        
        if total_persistence >= 0.99 {
            let scale_factor = 0.95 / total_persistence;
            for coeff in &mut self.alpha_coefficients {
                *coeff *= scale_factor;
            }
            for coeff in &mut self.beta_coefficients {
                *coeff *= scale_factor;
            }
        }
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
            if !self.conditional_variance.is_full() {
                self.conditional_variance.push(unconditional_var);
            }
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
            
            if !self.conditional_variance.is_full() {
                self.conditional_variance.push(variance);
            }
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
    returns: ArrayVec<f64, 512>,
    residuals: ArrayVec<f64, 512>,
    log_conditional_variance: ArrayVec<f64, 512>, // ln(σ²_t)
    standardized_residuals: ArrayVec<f64, 512>,   // z_t = ε_t / σ_t
    
    // Коэффициенты EGARCH модели
    omega: f64,                           // Константа
    alpha_coefficients: ArrayVec<f64, 16>, // Коэффициенты для |z_{t-i}|
    gamma_coefficients: ArrayVec<f64, 16>, // Асимметричные коэффициенты для z_{t-i}
    beta_coefficients: ArrayVec<f64, 16>,  // GARCH коэффициенты
    
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
            returns: ArrayVec::new(),
            residuals: ArrayVec::new(),
            log_conditional_variance: ArrayVec::new(),
            standardized_residuals: ArrayVec::new(),
            omega: -1.0,
            alpha_coefficients: ArrayVec::new(),
            gamma_coefficients: ArrayVec::new(),
            beta_coefficients: ArrayVec::new(),
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
            if !self.residuals.is_full() {
                self.residuals.push(residual);
            }
        }
    }
    
    /// Оценка EGARCH параметров (упрощенная версия)
    fn estimate_egarch_parameters(&mut self) {
        self.alpha_coefficients.clear();
        self.gamma_coefficients.clear();
        self.beta_coefficients.clear();
        
        // Простые начальные значения для EGARCH
        self.omega = -0.5;
        
        // Alpha коэффициенты (эффект размера)
        for i in 0..self.p {
            let coeff = 0.2 * (0.8_f64).powi(i as i32);
            if !self.alpha_coefficients.is_full() {
                self.alpha_coefficients.push(coeff);
            }
        }
        
        // Gamma коэффициенты (асимметричный эффект)
        for i in 0..self.p {
            let coeff = -0.1 * (0.9_f64).powi(i as i32); // Отрицательные для leverage effect
            if !self.gamma_coefficients.is_full() {
                self.gamma_coefficients.push(coeff);
            }
        }
        
        // Beta коэффициенты (персистентность)
        for i in 0..self.q {
            let coeff = 0.7 * (0.95_f64).powi(i as i32);
            if !self.beta_coefficients.is_full() {
                self.beta_coefficients.push(coeff);
            }
        }
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
            if !self.log_conditional_variance.is_full() {
                self.log_conditional_variance.push(initial_log_var);
            }
        }
        
        // Рассчитываем стандартизированные остатки для начальных значений
        for i in 0..start_idx.min(self.residuals.len()) {
            let std_residual = self.residuals[i] / initial_log_var.exp().sqrt();
            if !self.standardized_residuals.is_full() {
                self.standardized_residuals.push(std_residual);
            }
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
            
            if !self.log_conditional_variance.is_full() {
                self.log_conditional_variance.push(log_variance);
            }
            
            // Рассчитываем стандартизированный остаток
            let variance = log_variance.exp();
            let std_residual = self.residuals[t] / variance.sqrt();
            if !self.standardized_residuals.is_full() {
                self.standardized_residuals.push(std_residual);
            }
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
}






















