//! True Range (TR) Calculation Utility
//!
//! True Range = max(H-L, |H-PC|, |L-PC|)
//! где H = high, L = low, PC = previous close
//!
//! Используется в:
//! - ATR (Average True Range)
//! - Ultimate Oscillator
//! - Другие volatility индикаторы

/// Рассчитать True Range для текущего бара
///
/// # Arguments
///
/// * `high` - Максимальная цена текущего бара
/// * `low` - Минимальная цена текущего бара
/// * `prev_close` - Цена закрытия предыдущего бара
///
/// # Returns
///
/// True Range = max(high - low, |high - prev_close|, |low - prev_close|)
///
/// # Performance
///
/// Inline always для нулевой стоимости абстракции
///
/// # Examples
///
/// ```
/// use zengeld_chart_indicators::bar_indicators::utils::true_range::true_range;
///
/// let tr = true_range(105.0, 95.0, 100.0);
/// assert_eq!(tr, 10.0); // max(10, 5, 5) = 10
/// ```
#[inline(always)]
pub fn true_range(high: f64, low: f64, prev_close: f64) -> f64 {
    (high - low)
        .max((high - prev_close).abs())
        .max((low - prev_close).abs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_true_range_normal_bar() {
        // Обычный бар без гэпов: H=105, L=95, PC=100
        // TR = max(10, 5, 5) = 10
        assert_eq!(true_range(105.0, 95.0, 100.0), 10.0);
    }

    #[test]
    fn test_true_range_gap_up() {
        // Гэп вверх: H=110, L=105, PC=100
        // TR = max(5, 10, 5) = 10 (гэп больше range)
        assert_eq!(true_range(110.0, 105.0, 100.0), 10.0);
    }

    #[test]
    fn test_true_range_gap_down() {
        // Гэп вниз: H=95, L=90, PC=100
        // TR = max(5, 5, 10) = 10 (гэп больше range)
        assert_eq!(true_range(95.0, 90.0, 100.0), 10.0);
    }

    #[test]
    fn test_true_range_wide_bar() {
        // Широкий бар: H=120, L=80, PC=100
        // TR = max(40, 20, 20) = 40 (range больше гэпов)
        assert_eq!(true_range(120.0, 80.0, 100.0), 40.0);
    }

    #[test]
    fn test_true_range_doji() {
        // Доджи: H=100.5, L=99.5, PC=100
        // TR = max(1, 0.5, 0.5) = 1
        assert_eq!(true_range(100.5, 99.5, 100.0), 1.0);
    }

    #[test]
    fn test_true_range_extreme_gap() {
        // Экстремальный гэп: H=150, L=145, PC=100
        // TR = max(5, 50, 45) = 50
        assert_eq!(true_range(150.0, 145.0, 100.0), 50.0);
    }
}
