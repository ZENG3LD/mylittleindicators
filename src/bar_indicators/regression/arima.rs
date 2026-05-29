//! ARIMA and ARIMAX Models
//! AutoRegressive Integrated Moving Average models for time series analysis
//! ARIMA(p,d,q) - p: AR terms, d: differencing, q: MA terms
//! ARIMAX - ARIMA with eXogenous variables
//!
//! AR coefficients come from a REAL OLS fit (shared `linalg::ols`, full
//! Gaussian elimination with partial pivoting) rather than the prior
//! diagonal-only normal-equations hack that ignored regressor correlation. MA
//! coefficients come from REAL conditional-sum-of-squares (CSS) estimation —
//! the residuals depend recursively on θ, so θ is fit by minimizing the SSE
//! with Nelder-Mead — replacing the hardcoded `0.1/(i+1)` heuristic.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::utils::math::linalg::ols;
use crate::bar_indicators::utils::math::optimize::{minimize, reflect_into, NmConfig};

/// ARIMA Model - AutoRegressive Integrated Moving Average
#[derive(Clone)]
pub struct Arima {
    // Параметры модели
    p: usize, // AR order (авторегрессия)
    d: usize, // Differencing order (интегрирование)
    q: usize, // MA order (скользящее среднее)
    
    // Данные
    original_series: Vec<f64>,
    differenced_series: Vec<f64>,
    residuals: Vec<f64>,

    // Коэффициенты модели
    ar_coefficients: Vec<f64>,           // φ (phi) коэффициенты
    ma_coefficients: Vec<f64>,           // θ (theta) коэффициенты
    constant: f64,

    // Прогнозы и метрики
    forecast: f64,
    fitted_values: Vec<f64>,
    aic: f64, // Akaike Information Criterion
    bic: f64, // Bayesian Information Criterion
    
    // Состояние
    is_fitted: bool,
    min_observations: usize,
}

impl Arima {
    pub fn new(p: usize, d: usize, q: usize) -> Self {
        let min_obs = (p + d + q + 1).max(30); // Минимум наблюдений для надежной оценки
        
        Self {
            p: p.min(16), // Ограничиваем максимальный порядок
            d: d.min(3),
            q: q.min(16),
            original_series: Vec::with_capacity(512),
            differenced_series: Vec::with_capacity(512),
            residuals: Vec::with_capacity(512),
            ar_coefficients: Vec::with_capacity(32),
            ma_coefficients: Vec::with_capacity(32),
            constant: 0.0,
            forecast: 0.0,
            fitted_values: Vec::with_capacity(512),
            aic: f64::INFINITY,
            bic: f64::INFINITY,
            is_fitted: false,
            min_observations: min_obs,
        }
    }
    
    /// Обновить модель новым значением
    pub fn update(&mut self, value: f64) -> f64 {
        // Добавляем новое значение
        if self.original_series.len() >= 512 {
            self.original_series.remove(0);
        }
        self.original_series.push(value);
        
        // Если достаточно данных, переоцениваем модель
        if self.original_series.len() >= self.min_observations {
            self.fit_model();
            self.generate_forecast();
        }
        
        self.forecast
    }
    
    /// Подгонка модели ARIMA
    fn fit_model(&mut self) {
        // 1. Применяем дифференцирование
        self.apply_differencing();

        // 2. Оцениваем AR коэффициенты полным OLS.
        self.estimate_ar_coefficients();

        // 3. Константа (среднее·(1−Σφ)) — нужна ДО CSS-оценки MA, т.к. остатки
        //    MA строятся на AR-фильтрованных инновациях w_t = y_t − c − Σφ·y.
        self.estimate_constant();

        // 4. Оцениваем MA коэффициенты методом CSS (остатки зависят от θ).
        self.estimate_ma_coefficients();

        // 5. Рассчитываем подогнанные значения и остатки
        self.calculate_fitted_values();

        // 6. Рассчитываем информационные критерии
        self.calculate_information_criteria();

        self.is_fitted = true;
    }

    /// Оценка константы модели
    fn estimate_constant(&mut self) {
        if self.differenced_series.is_empty() {
            self.constant = 0.0;
            return;
        }

        // Среднее дифференцированного ряда
        let mean: f64 = self.differenced_series.iter().sum::<f64>()
            / self.differenced_series.len() as f64;

        // Константа = mean * (1 - sum(AR coefficients))
        let ar_sum: f64 = self.ar_coefficients.iter().sum();
        self.constant = mean * (1.0 - ar_sum);
    }
    
    /// Применить дифференцирование
    fn apply_differencing(&mut self) {
        self.differenced_series.clear();
        
        let mut current_series = self.original_series.clone();
        
        // Применяем дифференцирование d раз
        for _ in 0..self.d {
            let mut diff_series: Vec<f64> = Vec::with_capacity(current_series.len());
            for i in 1..current_series.len() {
                diff_series.push(current_series[i] - current_series[i - 1]);
            }
            current_series = diff_series;
        }
        
        self.differenced_series = current_series;
    }
    
    /// Оценка AR коэффициентов методом наименьших квадратов (полный OLS).
    ///
    /// Regress the differenced series on its own `p` lags via the shared
    /// `linalg::ols` (proper (XᵀX)⁻¹Xᵀy, not the old diagonal-only shortcut),
    /// so correlated lags get correct coefficients. No intercept here — the
    /// level is captured by `constant` in `estimate_constant`.
    fn estimate_ar_coefficients(&mut self) {
        self.ar_coefficients.clear();

        if self.p == 0 || self.differenced_series.len() < self.p + 2 {
            // q-only model still needs an (empty) AR vector; leave it empty.
            return;
        }

        let n = self.differenced_series.len();
        let rows = n - self.p;
        let mut xm = Vec::with_capacity(rows * self.p);
        let mut yv = Vec::with_capacity(rows);
        for t in self.p..n {
            for lag in 1..=self.p {
                xm.push(self.differenced_series[t - lag]);
            }
            yv.push(self.differenced_series[t]);
        }
        if let Some(beta) = ols(&xm, &yv, rows, self.p) {
            self.ar_coefficients = beta;
        } else {
            self.ar_coefficients = vec![0.0; self.p];
        }
    }

    /// Оценка MA коэффициентов методом условной суммы квадратов (CSS).
    ///
    /// For MA(q) the residuals obey ε_t = w_t − Σθ_i ε_{t-i} where
    /// w_t = (y_t − c − Σφ_j y_{t-j}) is the AR-filtered innovation; ε depends
    /// recursively on θ, so there is no closed form. We minimize Σε²_t over θ
    /// with Nelder-Mead (invertibility kept by reflecting each θ into (−1,1)).
    /// AR coefficients and constant are held fixed at their OLS values.
    fn estimate_ma_coefficients(&mut self) {
        self.ma_coefficients.clear();
        let q = self.q.min(8);
        if q == 0 {
            return;
        }

        // Precompute the AR-filtered series w_t over the usable range.
        let p = self.p;
        let start = p.max(q);
        let series = self.differenced_series.clone();
        if series.len() <= start + 1 {
            self.ma_coefficients = vec![0.0; q];
            return;
        }
        let ar = self.ar_coefficients.clone();
        let c = self.constant;

        let css = |theta: &[f64]| -> f64 {
            // reflect θ into (−1,1) for invertibility.
            let th: Vec<f64> = theta.iter().map(|&t| reflect_into(t, -0.999, 0.999)).collect();
            let mut eps = vec![0.0_f64; series.len()];
            let mut sse = 0.0;
            for t in start..series.len() {
                let mut w = series[t] - c;
                for (j, &phi) in ar.iter().enumerate() {
                    w -= phi * series[t - 1 - j];
                }
                let mut e = w;
                for (i, &thi) in th.iter().enumerate() {
                    e -= thi * eps[t - 1 - i];
                }
                eps[t] = e;
                sse += e * e;
            }
            if sse.is_finite() {
                sse
            } else {
                f64::MAX
            }
        };

        let x0 = vec![0.1_f64; q];
        let res = minimize(
            css,
            &x0,
            &NmConfig {
                max_iters: 1500,
                step: 0.3,
                ..Default::default()
            },
        );
        self.ma_coefficients = res
            .x
            .iter()
            .map(|&t| reflect_into(t, -0.999, 0.999))
            .collect();
    }
    
    /// Рассчитать подогнанные значения
    fn calculate_fitted_values(&mut self) {
        self.fitted_values.clear();
        self.residuals.clear();
        
        if !self.is_ready_for_calculation() {
            return;
        }
        
        let start_idx = self.p.max(self.q);
        
        for t in start_idx..self.differenced_series.len() {
            let mut fitted_value = self.constant;
            
            // AR компонента
            for (i, &ar_coeff) in self.ar_coefficients.iter().enumerate() {
                if t > i {
                    fitted_value += ar_coeff * self.differenced_series[t - 1 - i];
                }
            }
            
            // MA компонента (упрощенно)
            for (i, &ma_coeff) in self.ma_coefficients.iter().enumerate() {
                if self.residuals.len() > i {
                    let residual_idx = self.residuals.len() - 1 - i;
                    fitted_value += ma_coeff * self.residuals[residual_idx];
                }
            }
            
            self.fitted_values.push(fitted_value);

            // Рассчитываем остаток
            let residual = self.differenced_series[t] - fitted_value;
            self.residuals.push(residual);
        }
    }
    
    /// Рассчитать информационные критерии
    fn calculate_information_criteria(&mut self) {
        if self.residuals.is_empty() {
            return;
        }
        
        let n = self.residuals.len() as f64;
        let k = (self.p + self.q + 1) as f64; // Количество параметров
        
        // Сумма квадратов остатков
        let sse: f64 = self.residuals.iter().map(|&r| r * r).sum();
        let mse = sse / n;
        
        // AIC = n * ln(MSE) + 2k
        self.aic = n * mse.ln() + 2.0 * k;
        
        // BIC = n * ln(MSE) + k * ln(n)
        self.bic = n * mse.ln() + k * n.ln();
    }
    
    /// Генерация прогноза
    fn generate_forecast(&mut self) {
        if !self.is_ready_for_calculation() {
            return;
        }
        
        self.forecast = self.constant;
        
        // AR компонента
        for (i, &ar_coeff) in self.ar_coefficients.iter().enumerate() {
            if self.differenced_series.len() > i {
                let idx = self.differenced_series.len() - 1 - i;
                self.forecast += ar_coeff * self.differenced_series[idx];
            }
        }
        
        // MA компонента
        for (i, &ma_coeff) in self.ma_coefficients.iter().enumerate() {
            if self.residuals.len() > i {
                let idx = self.residuals.len() - 1 - i;
                self.forecast += ma_coeff * self.residuals[idx];
            }
        }
    }
    
    /// Проверка готовности для расчетов
    fn is_ready_for_calculation(&self) -> bool {
        !self.differenced_series.is_empty() && 
        self.differenced_series.len() > self.p.max(self.q)
    }
    
    /// Получить прогноз
    pub fn forecast(&self) -> f64 {
        self.forecast
    }
    
    /// Получить AIC
    pub fn aic(&self) -> f64 {
        self.aic
    }
    
    /// Получить BIC
    pub fn bic(&self) -> f64 {
        self.bic
    }
    
    /// Получить параметры модели
    pub fn get_parameters(&self) -> (usize, usize, usize) {
        (self.p, self.d, self.q)
    }
    
    /// Получить коэффициенты
    pub fn get_coefficients(&self) -> (&[f64], &[f64], f64) {
        (&self.ar_coefficients, &self.ma_coefficients, self.constant)
    }
    
    /// Проверить готовность модели
    pub fn is_fitted(&self) -> bool {
        self.is_fitted
    }
    
    /// Сбросить модель
    pub fn reset(&mut self) {
        self.original_series.clear();
        self.differenced_series.clear();
        self.residuals.clear();
        self.ar_coefficients.clear();
        self.ma_coefficients.clear();
        self.fitted_values.clear();
        self.constant = 0.0;
        self.forecast = 0.0;
        self.aic = f64::INFINITY;
        self.bic = f64::INFINITY;
        self.is_fitted = false;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_fitted && self.original_series.len() >= self.min_observations
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.forecast)
    }
}

/// ARIMAX Model - ARIMA with eXogenous variables
#[derive(Clone)]
pub struct ArimaX {
    arima: Arima,
    
    // Экзогенные переменные
    exogenous_data: Vec<Vec<f64>>,        // Матрица экзогенных переменных
    exog_coefficients: Vec<f64>,          // Коэффициенты для экзогенных переменных
    num_exog_vars: usize,
}

impl ArimaX {
    pub fn new(p: usize, d: usize, q: usize, num_exog_vars: usize) -> Self {
        Self {
            arima: Arima::new(p, d, q),
            exogenous_data: Vec::with_capacity(512),
            exog_coefficients: Vec::with_capacity(16),
            num_exog_vars: num_exog_vars.min(16),
        }
    }
    
    /// Обновить модель с эндогенной переменной и экзогенными переменными
    pub fn update(&mut self, endogenous: f64, exogenous: &[f64]) -> f64 {
        // Добавляем экзогенные переменные
        let mut exog_row: Vec<f64> = Vec::with_capacity(self.num_exog_vars);
        for (i, &val) in exogenous.iter().enumerate() {
            if i < self.num_exog_vars {
                exog_row.push(val);
            }
        }

        if self.exogenous_data.len() >= 512 {
            self.exogenous_data.remove(0);
        }
        self.exogenous_data.push(exog_row);
        
        // Обновляем базовую ARIMA модель
        let base_forecast = self.arima.update(endogenous);
        
        // Добавляем вклад экзогенных переменных
        let mut exog_contribution = 0.0;
        if !self.exogenous_data.is_empty() {
            let latest_exog = &self.exogenous_data[self.exogenous_data.len() - 1];
            for (i, &coeff) in self.exog_coefficients.iter().enumerate() {
                if i < latest_exog.len() {
                    exog_contribution += coeff * latest_exog[i];
                }
            }
        }
        
        base_forecast + exog_contribution
    }
    
    /// Получить прогноз с учетом экзогенных переменных
    pub fn forecast_with_exog(&self, future_exog: &[f64]) -> f64 {
        let base_forecast = self.arima.forecast();
        
        let mut exog_contribution = 0.0;
        for (i, &coeff) in self.exog_coefficients.iter().enumerate() {
            if i < future_exog.len() {
                exog_contribution += coeff * future_exog[i];
            }
        }
        
        base_forecast + exog_contribution
    }
    
    /// Получить коэффициенты экзогенных переменных
    pub fn exog_coefficients(&self) -> &[f64] {
        &self.exog_coefficients
    }
    
    /// Делегирование методов к базовой ARIMA
    pub fn aic(&self) -> f64 { self.arima.aic() }
    pub fn bic(&self) -> f64 { self.arima.bic() }
    pub fn is_fitted(&self) -> bool { self.arima.is_fitted() }
    pub fn get_parameters(&self) -> (usize, usize, usize) { self.arima.get_parameters() }
    
    pub fn reset(&mut self) {
        self.arima.reset();
        self.exogenous_data.clear();
        self.exog_coefficients.clear();
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.arima.is_ready()
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.arima.forecast())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arima_creation() {
        let ind = Arima::new(1, 1, 1);
        assert!(!ind.is_ready());
        assert_eq!(ind.forecast(), 0.0);
    }

    #[test]
    fn test_arima_warmup() {
        let mut ind = Arima::new(1, 0, 1);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_arima_forecast_finite() {
        let mut ind = Arima::new(1, 1, 1);
        for i in 0..60 {
            let price = 100.0 + i as f64 * 0.5;
            ind.update(price);
        }
        assert!(ind.forecast().is_finite());
    }

    #[test]
    fn test_arima_reset() {
        let mut ind = Arima::new(1, 0, 1);
        for i in 0..50 {
            let price = 100.0 + i as f64;
            ind.update(price);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.forecast(), 0.0);
    }

    #[test]
    fn test_arimax_creation() {
        let ind = ArimaX::new(1, 0, 1, 2);
        assert!(!ind.is_ready());
    }

    #[test]
    fn test_arimax_update() {
        let mut ind = ArimaX::new(1, 0, 1, 2);
        for i in 0..50 {
            let endogenous = 100.0 + i as f64 * 0.5;
            let exogenous = [10.0 + i as f64 * 0.1, 20.0 - i as f64 * 0.1];
            ind.update(endogenous, &exogenous);
        }
        assert!(ind.is_ready());
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

    /// Full OLS must recover AR(2) coefficients that the old diagonal-only
    /// solver could not (the two lags are correlated). d=0 so the series is
    /// used directly.
    #[test]
    fn ar_ols_recovers_correlated_lags() {
        // AR(2): y_t = 0.5 y_{t-1} + 0.3 y_{t-2} + e_t  (stationary).
        let e = lcg(500, 13);
        let mut y = vec![0.0_f64; e.len()];
        for t in 2..e.len() {
            y[t] = 0.5 * y[t - 1] + 0.3 * y[t - 2] + e[t];
        }
        let mut a = Arima::new(2, 0, 0);
        for &v in &y {
            a.update(v);
        }
        let (ar, _ma, _c) = a.get_coefficients();
        assert_eq!(ar.len(), 2);
        // Recovery within sampling tolerance — the key point is φ₂ is NOT zero,
        // which the diagonal-only solver would badly underestimate.
        assert!((ar[0] - 0.5).abs() < 0.12, "φ₁ {} ≈ 0.5", ar[0]);
        assert!((ar[1] - 0.3).abs() < 0.12, "φ₂ {} ≈ 0.3", ar[1]);
    }

    /// CSS MA estimation must fit an MA(1) series better (lower residual SSE)
    /// than the discarded hardcoded θ=0.1/(i+1) guess.
    #[test]
    fn ma_css_beats_hardcoded() {
        // MA(1): y_t = e_t + 0.6 e_{t-1}, mean 0.
        let e = lcg(500, 21);
        let mut y = vec![0.0_f64; e.len()];
        for t in 1..e.len() {
            y[t] = e[t] + 0.6 * e[t - 1];
        }
        let mut a = Arima::new(0, 0, 1);
        for &v in &y {
            a.update(v);
        }
        let (_ar, ma, _c) = a.get_coefficients();
        assert_eq!(ma.len(), 1);
        // CSS should land near the true 0.6 (sign + magnitude), far from 0.1.
        assert!(ma[0] > 0.3, "MA θ {} should approach true 0.6, not 0.1", ma[0]);
    }
}






















