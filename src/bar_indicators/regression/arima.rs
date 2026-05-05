//! ARIMA and ARIMAX Models
//! AutoRegressive Integrated Moving Average models for time series analysis
//! ARIMA(p,d,q) - p: AR terms, d: differencing, q: MA terms
//! ARIMAX - ARIMA with eXogenous variables

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// ARIMA Model - AutoRegressive Integrated Moving Average
#[derive(Clone)]
pub struct Arima {
    // Параметры модели
    p: usize, // AR order (авторегрессия)
    d: usize, // Differencing order (интегрирование)
    q: usize, // MA order (скользящее среднее)
    
    // Данные
    original_series: ArrayVec<f64, 512>,
    differenced_series: ArrayVec<f64, 512>,
    residuals: ArrayVec<f64, 512>,
    
    // Коэффициенты модели
    ar_coefficients: ArrayVec<f64, 32>, // φ (phi) коэффициенты
    ma_coefficients: ArrayVec<f64, 32>, // θ (theta) коэффициенты
    constant: f64,
    
    // Прогнозы и метрики
    forecast: f64,
    fitted_values: ArrayVec<f64, 512>,
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
            original_series: ArrayVec::new(),
            differenced_series: ArrayVec::new(),
            residuals: ArrayVec::new(),
            ar_coefficients: ArrayVec::new(),
            ma_coefficients: ArrayVec::new(),
            constant: 0.0,
            forecast: 0.0,
            fitted_values: ArrayVec::new(),
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

        // 2. Оцениваем AR коэффициенты методом Юла-Уокера (упрощенная версия)
        self.estimate_ar_coefficients();

        // 3. Оцениваем MA коэффициенты (упрощенная версия)
        self.estimate_ma_coefficients();

        // 4. Вычисляем константу (среднее дифференцированного ряда минус AR вклад)
        self.estimate_constant();

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
            let mut diff_series = ArrayVec::new();
            for i in 1..current_series.len() {
                if !diff_series.is_full() {
                    diff_series.push(current_series[i] - current_series[i-1]);
                }
            }
            current_series = diff_series;
        }
        
        self.differenced_series = current_series;
    }
    
    /// Оценка AR коэффициентов (упрощенный метод наименьших квадратов)
    fn estimate_ar_coefficients(&mut self) {
        self.ar_coefficients.clear();
        
        if self.p == 0 || self.differenced_series.len() < self.p + 1 {
            return;
        }
        
        // Простая регрессия для AR коэффициентов
        let n = self.differenced_series.len();
        let mut x_matrix = Vec::new();
        let mut y_vector = Vec::new();
        
        for t in self.p..n {
            let mut x_row = Vec::new();
            for lag in 1..=self.p {
                x_row.push(self.differenced_series[t - lag]);
            }
            x_matrix.push(x_row);
            y_vector.push(self.differenced_series[t]);
        }
        
        // Решаем систему методом наименьших квадратов (упрощенно)
        if !x_matrix.is_empty() {
            let coeffs = self.solve_least_squares(&x_matrix, &y_vector);
            for &coeff in &coeffs {
                if !self.ar_coefficients.is_full() {
                    self.ar_coefficients.push(coeff);
                }
            }
        }
    }
    
    /// Оценка MA коэффициентов (упрощенная версия)
    fn estimate_ma_coefficients(&mut self) {
        self.ma_coefficients.clear();
        
        if self.q == 0 {
            return;
        }
        
        // Упрощенная оценка MA коэффициентов через автокорреляции остатков
        // В реальной реализации здесь был бы итеративный алгоритм
        for i in 0..self.q.min(8) {
            let coeff = 0.1 / (i as f64 + 1.0); // Простая эвристика
            if !self.ma_coefficients.is_full() {
                self.ma_coefficients.push(coeff);
            }
        }
    }
    
    /// Простой решатель системы линейных уравнений
    fn solve_least_squares(&self, x_matrix: &[Vec<f64>], y_vector: &[f64]) -> Vec<f64> {
        if x_matrix.is_empty() || y_vector.is_empty() {
            return Vec::new();
        }
        
        let n = x_matrix.len();
        let p = x_matrix[0].len();
        
        // X'X матрица
        let mut xtx = vec![vec![0.0; p]; p];
        for i in 0..p {
            for j in 0..p {
                for row in &x_matrix[..n] {
                    xtx[i][j] += row[i] * row[j];
                }
            }
        }

        // X'y вектор
        let mut xty = vec![0.0; p];
        for i in 0..p {
            for (row, &y) in x_matrix[..n].iter().zip(y_vector[..n].iter()) {
                xty[i] += row[i] * y;
            }
        }
        
        // Решение системы (упрощенно - только для диагональных элементов)
        let mut coefficients = Vec::new();
        for i in 0..p {
            if xtx[i][i].abs() > 1e-10 {
                coefficients.push(xty[i] / xtx[i][i]);
            } else {
                coefficients.push(0.0);
            }
        }
        
        coefficients
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
            
            if !self.fitted_values.is_full() {
                self.fitted_values.push(fitted_value);
            }
            
            // Рассчитываем остаток
            let residual = self.differenced_series[t] - fitted_value;
            if !self.residuals.is_full() {
                self.residuals.push(residual);
            }
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
    exogenous_data: ArrayVec<ArrayVec<f64, 16>, 512>, // Матрица экзогенных переменных
    exog_coefficients: ArrayVec<f64, 16>, // Коэффициенты для экзогенных переменных
    num_exog_vars: usize,
}

impl ArimaX {
    pub fn new(p: usize, d: usize, q: usize, num_exog_vars: usize) -> Self {
        Self {
            arima: Arima::new(p, d, q),
            exogenous_data: ArrayVec::new(),
            exog_coefficients: ArrayVec::new(),
            num_exog_vars: num_exog_vars.min(16),
        }
    }
    
    /// Обновить модель с эндогенной переменной и экзогенными переменными
    pub fn update(&mut self, endogenous: f64, exogenous: &[f64]) -> f64 {
        // Добавляем экзогенные переменные
        let mut exog_row = ArrayVec::new();
        for (i, &val) in exogenous.iter().enumerate() {
            if i < self.num_exog_vars && !exog_row.is_full() {
                exog_row.push(val);
            }
        }
        
        if self.exogenous_data.len() >= 512 {
            self.exogenous_data.remove(0);
        }
        if !self.exogenous_data.is_full() {
            self.exogenous_data.push(exog_row);
        }
        
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
} 






















