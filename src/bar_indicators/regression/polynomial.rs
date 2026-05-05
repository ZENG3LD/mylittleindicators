//! Polynomial Regression
//! Полиномиальная регрессия для моделирования нелинейных трендов
//! y = β₀ + β₁x + β₂x² + ... + βₙxⁿ + ε

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// Polynomial Regression Model
#[derive(Clone)]
pub struct PolynomialRegression {
    // Параметры модели
    degree: usize,                      // Степень полинома

    // Данные
    x_values: ArrayVec<f64, 512>,       // Независимая переменная (обычно время/индекс)
    y_values: ArrayVec<f64, 512>,       // Зависимая переменная (цена/значение)

    // Коэффициенты полинома
    coefficients: ArrayVec<f64, 16>,    // β₀, β₁, β₂, ..., βₙ

    // Подогнанные значения и остатки
    fitted_values: ArrayVec<f64, 512>,  // Предсказанные значения
    residuals: ArrayVec<f64, 512>,      // Остатки

    // Статистики модели
    r_squared: f64,                     // Коэффициент детерминации
    adjusted_r_squared: f64,            // Скорректированный R²
    mse: f64,                          // Среднеквадратичная ошибка
    rmse: f64,                         // Корень из MSE

    // Производные для анализа тренда
    first_derivative: f64,              // Первая производная (скорость изменения)
    second_derivative: f64,             // Вторая производная (ускорение)

    // Прогноз
    forecast: f64,
    forecast_trend: TrendDirection,

    // Источник данных
    source: OhlcvField,                 // Поле OHLCV для расчета

    // Состояние
    is_fitted: bool,
    min_observations: usize,
    current_x: f64,                     // Текущее значение x (обычно индекс)
}

/// Направление тренда на основе производных
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrendDirection {
    StrongUptrend,      // Первая производная > 0, вторая > 0 (ускоряющийся рост)
    Uptrend,           // Первая производная > 0, вторая ≤ 0 (замедляющийся рост)
    Sideways,          // Первая производная ≈ 0 (боковое движение)
    Downtrend,         // Первая производная < 0, вторая ≥ 0 (замедляющееся падение)
    StrongDowntrend,   // Первая производная < 0, вторая < 0 (ускоряющееся падение)
}

impl PolynomialRegression {
    pub fn new(degree: usize) -> Self {
        Self::with_source(degree, OhlcvField::Close)
    }

    /// Создать с настраиваемым источником данных
    pub fn with_source(degree: usize, source: OhlcvField) -> Self {
        let degree = degree.clamp(1, 8); // Ограничиваем степень от 1 до 8
        let min_obs = (degree + 2).max(10);

        Self {
            degree,
            x_values: ArrayVec::new(),
            y_values: ArrayVec::new(),
            coefficients: ArrayVec::new(),
            fitted_values: ArrayVec::new(),
            residuals: ArrayVec::new(),
            r_squared: 0.0,
            adjusted_r_squared: 0.0,
            mse: 0.0,
            rmse: 0.0,
            first_derivative: 0.0,
            second_derivative: 0.0,
            forecast: 0.0,
            forecast_trend: TrendDirection::Sideways,
            source,
            is_fitted: false,
            min_observations: min_obs,
            current_x: 0.0,
        }
    }

    /// Обновить модель с OHLCV баром
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let value = self.source.extract(open, high, low, close, volume);
        self.update(value)
    }

    /// Обновить модель новым значением
    pub fn update(&mut self, value: f64) -> f64 {
        // Добавляем новое значение
        if self.y_values.len() >= 512 {
            self.y_values.remove(0);
            self.x_values.remove(0);
            // Сдвигаем x значения
            for x in &mut self.x_values {
                *x -= 1.0;
            }
            self.current_x -= 1.0;
        }
        
        if !self.y_values.is_full() {
            self.y_values.push(value);
            self.x_values.push(self.current_x);
        }
        
        self.current_x += 1.0;
        
        // Если достаточно данных, переоцениваем модель
        if self.y_values.len() >= self.min_observations {
            self.fit_model();
            self.calculate_derivatives();
            self.generate_forecast();
            self.determine_trend_direction();
        }
        
        self.forecast
    }
    
    /// Подгонка полиномиальной модели
    fn fit_model(&mut self) {
        if self.y_values.len() < self.min_observations {
            return;
        }
        
        // 1. Создаем матрицу Вандермонда
        let vandermonde_matrix = self.create_vandermonde_matrix();
        
        // 2. Решаем систему нормальных уравнений
        self.solve_normal_equations(&vandermonde_matrix);
        
        // 3. Рассчитываем подогнанные значения и остатки
        self.calculate_fitted_and_residuals();
        
        // 4. Рассчитываем статистики модели
        self.calculate_model_statistics();
        
        self.is_fitted = true;
    }
    
    /// Создание матрицы Вандермонда
    fn create_vandermonde_matrix(&self) -> Vec<Vec<f64>> {
        let n = self.x_values.len();
        let mut matrix = vec![vec![0.0; self.degree + 1]; n];
        
        for (row, &x) in matrix.iter_mut().zip(self.x_values.iter()) {
            for (j, cell) in row.iter_mut().enumerate() {
                *cell = x.powi(j as i32);
            }
        }
        
        matrix
    }
    
    /// Решение системы нормальных уравнений (X'X)β = X'y
    fn solve_normal_equations(&mut self, x_matrix: &[Vec<f64>]) {
        self.coefficients.clear();
        
        let n = x_matrix.len();
        let p = self.degree + 1;
        
        if n == 0 || p == 0 {
            return;
        }
        
        // Создаем X'X матрицу
        let mut xtx = vec![vec![0.0; p]; p];
        for i in 0..p {
            for j in 0..p {
                for row in &x_matrix[..n] {
                    xtx[i][j] += row[i] * row[j];
                }
            }
        }

        // Создаем X'y вектор
        let mut xty = vec![0.0; p];
        for i in 0..p {
            for (row, &y) in x_matrix[..n].iter().zip(self.y_values.iter()) {
                xty[i] += row[i] * y;
            }
        }
        
        // Решаем систему (упрощенный метод - диагональное приближение)
        for i in 0..p {
            if xtx[i][i].abs() > 1e-12 {
                let coeff = xty[i] / xtx[i][i];
                if !self.coefficients.is_full() {
                    self.coefficients.push(coeff);
                }
            } else if !self.coefficients.is_full() {
                self.coefficients.push(0.0);
            }
        }
        
        // Если не удалось решить систему, используем простую линейную регрессию
        if self.coefficients.len() < 2 {
            self.fallback_to_linear_regression();
        }
    }
    
    /// Резервная линейная регрессия если полиномиальная не работает
    fn fallback_to_linear_regression(&mut self) {
        self.coefficients.clear();
        
        if self.x_values.len() < 2 || self.y_values.len() < 2 {
            return;
        }
        
        let n = self.x_values.len() as f64;
        let sum_x: f64 = self.x_values.iter().sum();
        let sum_y: f64 = self.y_values.iter().sum();
        let sum_xy: f64 = self.x_values.iter().zip(self.y_values.iter())
            .map(|(&x, &y)| x * y).sum();
        let sum_x2: f64 = self.x_values.iter().map(|&x| x * x).sum();
        
        let mean_x = sum_x / n;
        let mean_y = sum_y / n;
        
        // Slope (β₁)
        let denominator = sum_x2 - n * mean_x * mean_x;
        let slope = if denominator.abs() > 1e-12 {
            (sum_xy - n * mean_x * mean_y) / denominator
        } else {
            0.0
        };
        
        // Intercept (β₀)
        let intercept = mean_y - slope * mean_x;
        
        if !self.coefficients.is_full() {
            self.coefficients.push(intercept);
        }
        if !self.coefficients.is_full() {
            self.coefficients.push(slope);
        }
    }
    
    /// Рассчитать подогнанные значения и остатки
    fn calculate_fitted_and_residuals(&mut self) {
        self.fitted_values.clear();
        self.residuals.clear();
        
        for (i, &x) in self.x_values.iter().enumerate() {
            let fitted_value = self.evaluate_polynomial(x);
            
            if !self.fitted_values.is_full() {
                self.fitted_values.push(fitted_value);
            }
            
            if i < self.y_values.len() {
                let residual = self.y_values[i] - fitted_value;
                if !self.residuals.is_full() {
                    self.residuals.push(residual);
                }
            }
        }
    }
    
    /// Вычислить значение полинома в точке x
    fn evaluate_polynomial(&self, x: f64) -> f64 {
        let mut result = 0.0;
        for (i, &coeff) in self.coefficients.iter().enumerate() {
            result += coeff * x.powi(i as i32);
        }
        result
    }
    
    /// Рассчитать статистики модели
    fn calculate_model_statistics(&mut self) {
        if self.y_values.is_empty() || self.fitted_values.is_empty() {
            return;
        }
        
        let n = self.y_values.len() as f64;
        let p = self.coefficients.len() as f64;
        
        // Среднее значение y
        let y_mean: f64 = self.y_values.iter().sum::<f64>() / n;
        
        // Сумма квадратов
        let mut ss_tot = 0.0; // Общая сумма квадратов
        let mut ss_res = 0.0; // Остаточная сумма квадратов
        
        for (i, &y_actual) in self.y_values.iter().enumerate() {
            ss_tot += (y_actual - y_mean).powi(2);
            
            if i < self.fitted_values.len() {
                let y_fitted = self.fitted_values[i];
                ss_res += (y_actual - y_fitted).powi(2);
            }
        }
        
        // R-squared
        self.r_squared = if ss_tot > 0.0 {
            1.0 - (ss_res / ss_tot)
        } else {
            0.0
        };
        
        // Adjusted R-squared
        if n > p + 1.0 {
            self.adjusted_r_squared = 1.0 - ((ss_res / (n - p)) / (ss_tot / (n - 1.0)));
        } else {
            self.adjusted_r_squared = self.r_squared;
        }
        
        // MSE и RMSE
        self.mse = ss_res / n;
        self.rmse = self.mse.sqrt();
    }
    
    /// Рассчитать производные в текущей точке
    fn calculate_derivatives(&mut self) {
        if self.coefficients.len() < 2 {
            self.first_derivative = 0.0;
            self.second_derivative = 0.0;
            return;
        }
        
        let x = self.current_x - 1.0; // Последняя точка
        
        // Первая производная: d/dx(Σ βᵢxⁱ) = Σ i*βᵢxⁱ⁻¹
        self.first_derivative = 0.0;
        for (i, &coeff) in self.coefficients.iter().enumerate().skip(1) {
            self.first_derivative += (i as f64) * coeff * x.powi(i as i32 - 1);
        }
        
        // Вторая производная: d²/dx²(Σ βᵢxⁱ) = Σ i*(i-1)*βᵢxⁱ⁻²
        self.second_derivative = 0.0;
        for (i, &coeff) in self.coefficients.iter().enumerate().skip(2) {
            self.second_derivative += (i as f64) * ((i - 1) as f64) * coeff * x.powi(i as i32 - 2);
        }
    }
    
    /// Генерация прогноза на следующую точку
    fn generate_forecast(&mut self) {
        if !self.is_fitted {
            return;
        }
        
        self.forecast = self.evaluate_polynomial(self.current_x);
    }
    
    /// Определение направления тренда
    fn determine_trend_direction(&mut self) {
        let first_deriv_threshold = self.rmse * 0.1; // Порог для определения значимости
        
        self.forecast_trend = if self.first_derivative.abs() < first_deriv_threshold {
            TrendDirection::Sideways
        } else if self.first_derivative > 0.0 {
            if self.second_derivative > 0.0 {
                TrendDirection::StrongUptrend
            } else {
                TrendDirection::Uptrend
            }
        } else if self.second_derivative < 0.0 {
            TrendDirection::StrongDowntrend
        } else {
            TrendDirection::Downtrend
        };
    }
    
    /// Получить прогноз
    pub fn forecast(&self) -> f64 {
        self.forecast
    }
    
    /// Получить направление тренда
    pub fn trend_direction(&self) -> TrendDirection {
        self.forecast_trend
    }
    
    /// Получить производные
    pub fn derivatives(&self) -> (f64, f64) {
        (self.first_derivative, self.second_derivative)
    }
    
    /// Получить коэффициенты полинома
    pub fn coefficients(&self) -> &[f64] {
        &self.coefficients
    }
    
    /// Получить статистики модели
    pub fn statistics(&self) -> (f64, f64, f64, f64) {
        (self.r_squared, self.adjusted_r_squared, self.mse, self.rmse)
    }
    
    /// Получить степень полинома
    pub fn degree(&self) -> usize {
        self.degree
    }
    
    /// Проверить готовность модели
    pub fn is_fitted(&self) -> bool {
        self.is_fitted
    }
    
    /// Вычислить значение полинома для произвольной точки
    pub fn predict(&self, x: f64) -> f64 {
        if !self.is_fitted {
            return 0.0;
        }
        self.evaluate_polynomial(x)
    }
    
    /// Получить производную в произвольной точке
    pub fn derivative_at(&self, x: f64) -> f64 {
        if self.coefficients.len() < 2 {
            return 0.0;
        }
        
        let mut derivative = 0.0;
        for (i, &coeff) in self.coefficients.iter().enumerate().skip(1) {
            derivative += (i as f64) * coeff * x.powi(i as i32 - 1);
        }
        derivative
    }
    
    /// Найти экстремумы полинома (корни первой производной)
    pub fn find_extrema(&self) -> ArrayVec<f64, 8> {
        let mut extrema = ArrayVec::new();
        
        if self.coefficients.len() < 3 {
            return extrema; // Линейная функция не имеет экстремумов
        }
        
        // Для простоты находим экстремумы численно в текущем диапазоне данных
        if let (Some(&x_min), Some(&x_max)) = (self.x_values.first(), self.x_values.last()) {
            let step = (x_max - x_min) / 100.0;
            let mut prev_deriv = self.derivative_at(x_min);
            
            for i in 1..=100 {
                let x = x_min + i as f64 * step;
                let curr_deriv = self.derivative_at(x);
                
                // Смена знака производной указывает на экстремум
                if prev_deriv * curr_deriv < 0.0
                    && !extrema.is_full() {
                        extrema.push(x - step * 0.5); // Приблизительная позиция экстремума
                    }
                
                prev_deriv = curr_deriv;
            }
        }
        
        extrema
    }
    
    /// Сбросить модель
    pub fn reset(&mut self) {
        self.x_values.clear();
        self.y_values.clear();
        self.coefficients.clear();
        self.fitted_values.clear();
        self.residuals.clear();
        self.r_squared = 0.0;
        self.adjusted_r_squared = 0.0;
        self.mse = 0.0;
        self.rmse = 0.0;
        self.first_derivative = 0.0;
        self.second_derivative = 0.0;
        self.forecast = 0.0;
        self.forecast_trend = TrendDirection::Sideways;
        self.is_fitted = false;
        self.current_x = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.y_values.len() >= self.min_observations
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.forecast)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polynomial_regression_creation() {
        let ind = PolynomialRegression::new(2);
        assert!(!ind.is_ready());
        assert_eq!(ind.forecast(), 0.0);
    }

    #[test]
    fn test_polynomial_regression_warmup() {
        let mut ind = PolynomialRegression::new(2);
        for i in 0..15 {
            let value = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update(value);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_polynomial_regression_forecast_finite() {
        let mut ind = PolynomialRegression::new(2);
        for i in 0..20 {
            let value = 100.0 + i as f64 * 0.5;
            ind.update(value);
        }
        assert!(ind.forecast().is_finite());
        let (r2, _, _, rmse) = ind.statistics();
        assert!(r2.is_finite());
        assert!(rmse.is_finite());
    }

    #[test]
    fn test_polynomial_regression_trend() {
        let mut ind = PolynomialRegression::new(2);
        // Uptrend data
        for i in 0..20 {
            let value = 100.0 + i as f64 * 2.0;
            ind.update(value);
        }
        let (first_deriv, _) = ind.derivatives();
        assert!(first_deriv > 0.0);
    }

    #[test]
    fn test_polynomial_regression_reset() {
        let mut ind = PolynomialRegression::new(2);
        for i in 0..20 {
            let value = 100.0 + i as f64;
            ind.update(value);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.forecast(), 0.0);
    }
} 






















