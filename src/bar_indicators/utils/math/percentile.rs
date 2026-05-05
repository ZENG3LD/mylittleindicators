//! Percentile Ranking Utilities
//!
//! Оптимизированные функции для расчета percentile rank и поиска k-го элемента.
//!
//! ## Performance
//!
//! - `percentile_rank()`: O(n) - линейное сканирование вместо O(n log n) сортировки
//! - `quickselect_nth()`: O(n) average case - для медианы и квартилей
//!
//! ## Speedup
//!
//! Для типичного периода 100:
//! - Old (sort): ~664 операций (100 * log2(100))
//! - New (count): ~100 операций
//! - **Ускорение: 6-8x**

/// Быстрый расчет percentile rank без сортировки
///
/// Вместо O(n log n) сортировки используется O(n) подсчет.
///
/// # Arguments
///
/// * `values` - Слайс значений для анализа
/// * `target` - Значение, для которого ищем percentile rank
///
/// # Returns
///
/// Percentile rank от 0.0 до 100.0
///
/// # Algorithm
///
/// 1. Подсчитываем сколько значений меньше target
/// 2. Подсчитываем сколько значений равны target
/// 3. Применяем формулу: (count_below + 0.5 * count_equal) / total * 100
///
/// # Performance
///
/// - Time: O(n) - один проход по данным
/// - Space: O(1) - константная память
/// - **6-8x быстрее** сортировки для типичных периодов
///
/// # Examples
///
/// ```
/// use zengeld_chart_indicators::bar_indicators::utils::math::percentile::percentile_rank;
///
/// let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// let rank = percentile_rank(&values, 3.0);
/// assert!((rank - 50.0).abs() < 5.0); // ~50th percentile
/// ```
#[inline]
pub fn percentile_rank(values: &[f64], target: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let n = values.len();
    let mut count_below = 0;
    let mut count_equal = 0;

    // Один проход O(n) вместо O(n log n) сортировки
    for &val in values {
        if val < target {
            count_below += 1;
        } else if (val - target).abs() < f64::EPSILON {
            count_equal += 1;
        }
    }

    // Стандартная формула percentile rank с интерполяцией
    ((count_below as f64 + 0.5 * count_equal as f64) / n as f64) * 100.0
}

/// Quickselect алгоритм для поиска k-го наименьшего элемента
///
/// Используется для медианы (k = n/2) и квартилей (k = n/4, 3n/4).
///
/// # Arguments
///
/// * `values` - Изменяемый слайс значений (будет частично отсортирован)
/// * `k` - Индекс искомого элемента (0-based)
///
/// # Returns
///
/// k-й наименьший элемент
///
/// # Algorithm
///
/// Quickselect - модификация quicksort, которая ищет только один элемент.
/// - Average case: O(n)
/// - Worst case: O(n²) - но очень редко на случайных данных
///
/// # Note
///
/// Функция изменяет порядок элементов в `values` (частичная сортировка).
///
/// # Examples
///
/// ```
/// use zengeld_chart_indicators::bar_indicators::utils::math::percentile::quickselect_nth;
///
/// let mut values = vec![5.0, 1.0, 3.0, 2.0, 4.0];
/// let median = quickselect_nth(&mut values, 2); // k=2 для медианы в массиве длины 5
/// assert_eq!(median, 3.0);
/// ```
pub fn quickselect_nth(values: &mut [f64], k: usize) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    if k >= values.len() {
        return *values.last().unwrap();
    }

    quickselect_nth_impl(values, 0, values.len() - 1, k)
}

fn quickselect_nth_impl(values: &mut [f64], left: usize, right: usize, k: usize) -> f64 {
    if left == right {
        return values[left];
    }

    let pivot_index = partition(values, left, right);

    if k == pivot_index {
        values[k]
    } else if k < pivot_index {
        quickselect_nth_impl(values, left, pivot_index.saturating_sub(1), k)
    } else {
        quickselect_nth_impl(values, pivot_index + 1, right, k)
    }
}

fn partition(values: &mut [f64], left: usize, right: usize) -> usize {
    let pivot = values[right];
    let mut i = left;

    for j in left..right {
        if values[j] <= pivot {
            values.swap(i, j);
            i += 1;
        }
    }

    values.swap(i, right);
    i
}

/// Удобная функция для расчета медианы
///
/// # Arguments
///
/// * `values` - Изменяемый слайс значений
///
/// # Returns
///
/// Медиана (50-й перцентиль)
///
/// # Performance
///
/// O(n) average case с quickselect
///
/// # Examples
///
/// ```
/// use zengeld_chart_indicators::bar_indicators::utils::math::percentile::median;
///
/// let mut values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
/// assert_eq!(median(&mut values), 3.0);
/// ```
#[inline]
pub fn median(values: &mut [f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    quickselect_nth(values, values.len() / 2)
}

/// Расчет квартилей (Q1, Q2, Q3)
///
/// # Arguments
///
/// * `values` - Изменяемый слайс значений
///
/// # Returns
///
/// Кортеж (Q1, Q2, Q3) где:
/// - Q1 = 25-й перцентиль
/// - Q2 = 50-й перцентиль (медиана)
/// - Q3 = 75-й перцентиль
///
/// # Performance
///
/// O(n) с тремя проходами quickselect
///
/// # Examples
///
/// ```
/// use zengeld_chart_indicators::bar_indicators::utils::math::percentile::quartiles;
///
/// let mut values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
/// let (q1, q2, q3) = quartiles(&mut values);
/// ```
pub fn quartiles(values: &mut [f64]) -> (f64, f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0, 0.0);
    }
    let n = values.len();
    (
        quickselect_nth(values, n / 4),
        quickselect_nth(values, n / 2),
        quickselect_nth(values, 3 * n / 4),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile_rank_basic() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        // Первый элемент - низкий percentile
        let rank1 = percentile_rank(&values, 1.0);
        assert!((rank1 - 10.0).abs() < 5.0, "rank1 = {}", rank1);

        // Средний элемент - ~50th percentile
        let rank3 = percentile_rank(&values, 3.0);
        assert!((rank3 - 50.0).abs() < 5.0, "rank3 = {}", rank3);

        // Последний элемент - высокий percentile
        let rank5 = percentile_rank(&values, 5.0);
        assert!((rank5 - 90.0).abs() < 5.0, "rank5 = {}", rank5);
    }

    #[test]
    fn test_percentile_rank_duplicates() {
        let values = vec![1.0, 2.0, 2.0, 2.0, 5.0];
        let rank = percentile_rank(&values, 2.0);

        // С дубликатами rank должен быть в середине диапазона дубликатов
        assert!(rank > 20.0 && rank < 80.0, "rank = {}", rank);
    }

    #[test]
    fn test_percentile_rank_empty() {
        let values: Vec<f64> = vec![];
        assert_eq!(percentile_rank(&values, 1.0), 0.0);
    }

    #[test]
    fn test_percentile_rank_single() {
        let values = vec![42.0];
        assert!((percentile_rank(&values, 42.0) - 50.0).abs() < 1.0);
    }

    #[test]
    fn test_percentile_rank_extremes() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        // Значение ниже всех
        assert!(percentile_rank(&values, 0.0) < 10.0);

        // Значение выше всех
        assert!(percentile_rank(&values, 10.0) > 90.0);
    }

    #[test]
    fn test_quickselect_median() {
        let mut values = vec![5.0, 1.0, 3.0, 2.0, 4.0];
        assert_eq!(quickselect_nth(&mut values, 2), 3.0);
    }

    #[test]
    fn test_quickselect_first() {
        let mut values = vec![5.0, 1.0, 3.0, 2.0, 4.0];
        assert_eq!(quickselect_nth(&mut values, 0), 1.0);
    }

    #[test]
    fn test_quickselect_last() {
        let mut values = vec![5.0, 1.0, 3.0, 2.0, 4.0];
        assert_eq!(quickselect_nth(&mut values, 4), 5.0);
    }

    #[test]
    fn test_median_odd_length() {
        let mut values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(median(&mut values), 3.0);
    }

    #[test]
    fn test_median_even_length() {
        let mut values = vec![1.0, 2.0, 3.0, 4.0];
        // Для четной длины берем средний элемент (верхняя медиана)
        let med = median(&mut values);
        assert!(med >= 2.0 && med <= 3.0);
    }

    #[test]
    fn test_median_empty() {
        let mut values: Vec<f64> = vec![];
        assert_eq!(median(&mut values), 0.0);
    }

    #[test]
    fn test_quartiles_basic() {
        let mut values = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
        let (q1, q2, q3) = quartiles(&mut values);

        // Q1 должен быть в первой четверти
        assert!(q1 >= 1.0 && q1 <= 3.0, "q1 = {}", q1);

        // Q2 (медиана) должен быть в середине
        assert!(q2 >= 3.0 && q2 <= 6.0, "q2 = {}", q2);

        // Q3 должен быть в третьей четверти
        assert!(q3 >= 5.0 && q3 <= 8.0, "q3 = {}", q3);

        // Проверка порядка
        assert!(q1 < q2);
        assert!(q2 < q3);
    }

    #[test]
    fn test_quartiles_small() {
        let mut values = vec![1.0, 2.0, 3.0];
        let (q1, q2, q3) = quartiles(&mut values);

        assert!(q1 <= q2);
        assert!(q2 <= q3);
    }

    #[test]
    fn test_quartiles_empty() {
        let mut values: Vec<f64> = vec![];
        let (q1, q2, q3) = quartiles(&mut values);
        assert_eq!((q1, q2, q3), (0.0, 0.0, 0.0));
    }

    #[test]
    fn test_percentile_rank_large_dataset() {
        // Тест на большом датасете (имитация реального использования)
        let values: Vec<f64> = (0..100).map(|x| x as f64).collect();

        // Проверяем различные percentiles
        assert!((percentile_rank(&values, 0.0) - 0.5).abs() < 2.0);
        assert!((percentile_rank(&values, 25.0) - 25.5).abs() < 2.0);
        assert!((percentile_rank(&values, 50.0) - 50.5).abs() < 2.0);
        assert!((percentile_rank(&values, 75.0) - 75.5).abs() < 2.0);
        assert!((percentile_rank(&values, 99.0) - 99.5).abs() < 2.0);
    }
}
