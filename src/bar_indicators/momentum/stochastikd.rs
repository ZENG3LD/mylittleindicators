// StochastikD: Stochastics indicator with Nautilus-style %D logic
// (c) 2024
// OPTIMIZED: O(1) running sum for %D calculation

use arrayvec::ArrayVec;
use std::collections::VecDeque;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct StochastikD {
    period_k: usize,
    period_d: usize,
    highs: ArrayVec<f64, 512>,
    lows: ArrayVec<f64, 512>,

    // VecDeque for %D buffers (O(1) pop_front)
    c_sub_1: VecDeque<f64>,
    h_sub_l: VecDeque<f64>,

    // Running sums for O(1) %D calculation
    sum_c_sub_1: f64,
    sum_h_sub_l: f64,

    initialized: bool,
    value_k: f64,
    value_d: f64,
}

impl StochastikD {
    pub fn new(period_k: usize, period_d: usize) -> Self {
        assert!(period_k <= 512, "period_k must be <= 512");
        assert!(period_d <= 512, "period_d must be <= 512");
        Self {
            period_k,
            period_d,
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            c_sub_1: VecDeque::with_capacity(period_d),
            h_sub_l: VecDeque::with_capacity(period_d),
            sum_c_sub_1: 0.0,
            sum_h_sub_l: 0.0,
            initialized: false,
            value_k: 0.0,
            value_d: 0.0,
        }
    }

    /// Обновить StochastikD новым баром (high, low, close)
    pub fn update_bar(&mut self, high: f64, low: f64, close: f64, _volume: f64) -> (f64, f64) {
        // 1. Сначала заполняем буферы highs/lows до period_k
        if self.highs.len() < self.period_k {
            self.highs.push(high);
            self.lows.push(low);
            self.value_k = 0.0;
            self.value_d = 0.0;
            if self.highs.len() == self.period_k {
                self.initialized = true;
            }
            return (self.value_k, self.value_d);
        } else {
            // Сдвигаем кольцевой буфер
            for i in 1..self.period_k {
                self.highs[i - 1] = self.highs[i];
                self.lows[i - 1] = self.lows[i];
            }
            self.highs[self.period_k - 1] = high;
            self.lows[self.period_k - 1] = low;
        }

        if !self.initialized {
            self.value_k = 0.0;
            self.value_d = 0.0;
            return (self.value_k, self.value_d);
        }

        // 2. Вычисляем максимум/минимум по последним period_k
        let (k_min_low, k_max_high) = self.highs.iter()
            .zip(self.lows.iter())
            .fold((f64::INFINITY, f64::NEG_INFINITY),
                  |(min, max), (&h, &l)| (min.min(l), max.max(h)));

        if (k_max_high - k_min_low).abs() < 1e-12 {
            // Не обновляем буферы, не меняем значения
            return (self.value_k, self.value_d);
        }

        // 3. %K
        self.value_k = 100.0 * ((close - k_min_low) / (k_max_high - k_min_low));

        let c1 = close - k_min_low;
        let h1 = k_max_high - k_min_low;

        // 4. Обновляем буферы c_sub_1/h_sub_l с O(1) running sum
        if self.c_sub_1.len() >= self.period_d {
            let old_c = self.c_sub_1.pop_front().unwrap();
            let old_h = self.h_sub_l.pop_front().unwrap();
            self.sum_c_sub_1 -= old_c;
            self.sum_h_sub_l -= old_h;
        }

        self.c_sub_1.push_back(c1);
        self.h_sub_l.push_back(h1);
        self.sum_c_sub_1 += c1;
        self.sum_h_sub_l += h1;

        // 5. %D - O(1) calculation using running sums
        self.value_d = if self.sum_h_sub_l.abs() < 1e-12 {
            0.0
        } else {
            100.0 * (self.sum_c_sub_1 / self.sum_h_sub_l)
        };

        (self.value_k, self.value_d)
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.value_k, self.value_d)
    }

    pub fn value_k(&self) -> f64 {
        self.value_k
    }

    pub fn value_d(&self) -> f64 {
        self.value_d
    }

    pub fn is_ready(&self) -> bool {
        self.initialized
    }

    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.c_sub_1.clear();
        self.h_sub_l.clear();
        self.sum_c_sub_1 = 0.0;
        self.sum_h_sub_l = 0.0;
        self.initialized = false;
        self.value_k = 0.0;
        self.value_d = 0.0;
    }

    pub fn period_k(&self) -> usize {
        self.period_k
    }

    pub fn period_d(&self) -> usize {
        self.period_d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stochastikd_creation() {
        let stoch = StochastikD::new(14, 3);
        assert!(!stoch.is_ready());
        assert_eq!(stoch.value_k(), 0.0);
        assert_eq!(stoch.value_d(), 0.0);
        assert_eq!(stoch.period_k(), 14);
        assert_eq!(stoch.period_d(), 3);
    }

    #[test]
    fn test_stochastikd_uptrend() {
        let mut stoch = StochastikD::new(14, 3);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            stoch.update_bar(price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(stoch.is_ready());
        assert!(stoch.value_k() > 50.0, "StochastikD K should be > 50 in uptrend, got {}", stoch.value_k());
    }

    #[test]
    fn test_stochastikd_downtrend() {
        let mut stoch = StochastikD::new(14, 3);
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            stoch.update_bar(price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(stoch.is_ready());
        assert!(stoch.value_k() < 50.0, "StochastikD K should be < 50 in downtrend, got {}", stoch.value_k());
    }

    #[test]
    fn test_stochastikd_range() {
        let mut stoch = StochastikD::new(14, 3);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let (k, d) = stoch.update_bar(price + 2.0, price - 2.0, price, 1000.0);
            if stoch.is_ready() {
                assert!(k >= 0.0 && k <= 100.0, "K should be in [0, 100], got {}", k);
                assert!(d >= 0.0 && d <= 100.0, "D should be in [0, 100], got {}", d);
            }
        }
    }

    #[test]
    fn test_stochastikd_reset() {
        let mut stoch = StochastikD::new(14, 3);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            stoch.update_bar(price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(stoch.is_ready());
        stoch.reset();
        assert!(!stoch.is_ready());
        assert_eq!(stoch.value_k(), 0.0);
        assert_eq!(stoch.value_d(), 0.0);
    }
}
