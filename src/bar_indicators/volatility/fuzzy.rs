// High-performance Fuzzy Candlesticks
// (c) 2024

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandleDirection { Bull = 1, None = 0, Bear = -1 }

impl CandleDirection {
    pub fn as_i8(&self) -> i8 {
        match self {
            CandleDirection::Bull => 1,
            CandleDirection::None => 0,
            CandleDirection::Bear => -1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandleSize { None, VerySmall, Small, Medium, Large, VeryLarge, ExtremelyLarge }

impl CandleSize {
    pub fn as_i8(&self) -> i8 {
        match self {
            CandleSize::None => 0,
            CandleSize::VerySmall => 1,
            CandleSize::Small => 2,
            CandleSize::Medium => 3,
            CandleSize::Large => 4,
            CandleSize::VeryLarge => 5,
            CandleSize::ExtremelyLarge => 6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandleBodySize { None, Small, Medium, Large, Trend }

impl CandleBodySize {
    pub fn as_i8(&self) -> i8 {
        match self {
            CandleBodySize::None => 0,
            CandleBodySize::Small => 1,
            CandleBodySize::Medium => 2,
            CandleBodySize::Large => 3,
            CandleBodySize::Trend => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandleWickSize { None, Small, Medium, Large }

impl CandleWickSize {
    pub fn as_i8(&self) -> i8 {
        match self {
            CandleWickSize::None => 0,
            CandleWickSize::Small => 1,
            CandleWickSize::Medium => 2,
            CandleWickSize::Large => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FuzzyCandle {
    pub direction: CandleDirection,
    pub size: CandleSize,
    pub body_size: CandleBodySize,
    pub upper_wick_size: CandleWickSize,
    pub lower_wick_size: CandleWickSize,
}

#[derive(Clone)]
pub struct FuzzyCandlesticks {
    period: usize,
    threshold1: f64,
    threshold2: f64,
    threshold3: f64,
    threshold4: f64,
    lengths: arrayvec::ArrayVec<f64, 512>,
    body_percents: arrayvec::ArrayVec<f64, 512>,
    upper_wick_percents: arrayvec::ArrayVec<f64, 512>,
    lower_wick_percents: arrayvec::ArrayVec<f64, 512>,
    idx: usize,
    filled: bool,
    value: FuzzyCandle,
}

impl FuzzyCandlesticks {
    fn fuzzify_size(length: f64, mean_length: f64, sd_lengths: f64, t1: f64, t2: f64, t3: f64, t4: f64) -> CandleSize {
        if length == 0.0 {
            return CandleSize::None;
        }
        let mut x;
        // VerySmall
        x = sd_lengths.mul_add(-t2, mean_length);
        if length <= x {
            return CandleSize::VerySmall;
        }
        // Small
        x = sd_lengths.mul_add(t1, mean_length);
        if length <= x {
            return CandleSize::Small;
        }
        // Medium
        x = sd_lengths * t2;
        if length <= x {
            return CandleSize::Medium;
        }
        // Large
        x = sd_lengths.mul_add(t3, mean_length);
        if length <= x {
            return CandleSize::Large;
        }
        // VeryLarge
        x = sd_lengths.mul_add(t4, mean_length);
        if length <= x {
            return CandleSize::VeryLarge;
        }
        CandleSize::ExtremelyLarge
    }
    fn fuzzify_body_size(body_percent: f64, mean_body_percent: f64, sd_body_percent: f64, t1: f64, t2: f64, t3: f64) -> CandleBodySize {
        if body_percent == 0.0 {
            return CandleBodySize::None;
        }
        let mut x;
        // Small
        x = sd_body_percent.mul_add(-t1, mean_body_percent);
        if body_percent <= x {
            return CandleBodySize::Small;
        }
        // Medium
        x = sd_body_percent.mul_add(t2, mean_body_percent);
        if body_percent <= x {
            return CandleBodySize::Medium;
        }
        // Large
        x = sd_body_percent.mul_add(t3, mean_body_percent);
        if body_percent <= x {
            return CandleBodySize::Large;
        }
        CandleBodySize::Trend
    }
    fn fuzzify_wick_size(wick_percent: f64, mean_wick_percent: f64, sd_wick_percents: f64, t1: f64, t2: f64) -> CandleWickSize {
        if wick_percent == 0.0 {
            return CandleWickSize::None;
        }
        let mut x;
        // Small
        x = sd_wick_percents.mul_add(-t1, mean_wick_percent);
        if wick_percent <= x {
            return CandleWickSize::Small;
        }
        // Medium
        x = sd_wick_percents.mul_add(t2, mean_wick_percent);
        if wick_percent <= x {
            return CandleWickSize::Medium;
        }
        CandleWickSize::Large
    }
    pub fn new(period: usize, t1: f64, t2: f64, t3: f64, t4: f64) -> Self {
        Self {
            period,
            threshold1: t1,
            threshold2: t2,
            threshold3: t3,
            threshold4: t4,
            lengths: arrayvec::ArrayVec::new(),
            body_percents: arrayvec::ArrayVec::new(),
            upper_wick_percents: arrayvec::ArrayVec::new(),
            lower_wick_percents: arrayvec::ArrayVec::new(),
            idx: 0,
            filled: false,
            value: FuzzyCandle {
                direction: CandleDirection::None,
                size: CandleSize::None,
                body_size: CandleBodySize::None,
                upper_wick_size: CandleWickSize::None,
                lower_wick_size: CandleWickSize::None,
            },
        }
    }
    /// Обновить FuzzyCandlesticks новым баром (open, high, low, close)
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, _volume: f64) -> FuzzyCandle {
        let len = (high - low).abs();
        let (body_percent, upper_wick_percent, lower_wick_percent) = if len == 0.0 {
            (0.0, 0.0, 0.0)
        } else {
            (
                (open - low) / len,
                (high - open.max(close)) / len,
                (open.max(close) - low) / len,
            )
        };
        if self.lengths.len() == self.period {
            self.lengths.remove(0);
            self.body_percents.remove(0);
            self.upper_wick_percents.remove(0);
            self.lower_wick_percents.remove(0);
        }
        self.lengths.push(len);
        self.body_percents.push(body_percent);
        self.upper_wick_percents.push(upper_wick_percent);
        self.lower_wick_percents.push(lower_wick_percent);
        self.idx += 1;
        if self.lengths.len() >= self.period {
            self.filled = true;
        }
        if !self.filled {
            self.value = FuzzyCandle {
                direction: CandleDirection::None,
                size: CandleSize::None,
                body_size: CandleBodySize::None,
                upper_wick_size: CandleWickSize::None,
                lower_wick_size: CandleWickSize::None,
            };
            return self.value;
        }
        let mean_len = self.lengths.iter().sum::<f64>() / self.lengths.len() as f64;
        let sd_len = (self.lengths.iter().map(|&v| (v - mean_len).powi(2)).sum::<f64>() / self.lengths.len() as f64).sqrt();
        let mean_body = self.body_percents.iter().sum::<f64>() / self.body_percents.len() as f64;
        let sd_body = (self.body_percents.iter().map(|&v| (v - mean_body).powi(2)).sum::<f64>() / self.body_percents.len() as f64).sqrt();
        let mean_uw = self.upper_wick_percents.iter().sum::<f64>() / self.upper_wick_percents.len() as f64;
        let sd_uw = (self.upper_wick_percents.iter().map(|&v| (v - mean_uw).powi(2)).sum::<f64>() / self.upper_wick_percents.len() as f64).sqrt();
        let mean_lw = self.lower_wick_percents.iter().sum::<f64>() / self.lower_wick_percents.len() as f64;
        let sd_lw = (self.lower_wick_percents.iter().map(|&v| (v - mean_lw).powi(2)).sum::<f64>() / self.lower_wick_percents.len() as f64).sqrt();
        // Используем всегда последний (самый свежий) элемент буфера для классификации
        let direction = if close > open {
            CandleDirection::Bull
        } else if close < open {
            CandleDirection::Bear
        } else {
            CandleDirection::None
        };
        let idx = self.lengths.len() - 1;
        let size = Self::fuzzify_size(self.lengths[idx], mean_len, sd_len, self.threshold1, self.threshold2, self.threshold3, self.threshold4);
        let body_size = Self::fuzzify_body_size(self.body_percents[idx], mean_body, sd_body, self.threshold1, self.threshold2, self.threshold3);
        let upper_wick_size = Self::fuzzify_wick_size(self.upper_wick_percents[idx], mean_uw, sd_uw, self.threshold1, self.threshold2);
        let lower_wick_size = Self::fuzzify_wick_size(self.lower_wick_percents[idx], mean_lw, sd_lw, self.threshold1, self.threshold2);
        self.value = FuzzyCandle {
            direction,
            size,
            body_size,
            upper_wick_size,
            lower_wick_size,
        };
        self.value
    }
    /// Получить значение как FuzzyCandle (legacy)
    pub fn fuzzy_value(&self) -> FuzzyCandle {
        self.value
    }

    /// Получить значение в виде IndicatorValue
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::FuzzyCandle {
            direction: self.value.direction.as_i8(),
            size: self.value.size.as_i8(),
            body_size: self.value.body_size.as_i8(),
            upper_wick: self.value.upper_wick_size.as_i8(),
            lower_wick: self.value.lower_wick_size.as_i8(),
        }
    }

    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn reset(&mut self) {
        self.lengths.clear();
        self.body_percents.clear();
        self.upper_wick_percents.clear();
        self.lower_wick_percents.clear();
        self.idx = 0;
        self.filled = false;
        self.value = FuzzyCandle {
            direction: CandleDirection::None,
            size: CandleSize::None,
            body_size: CandleBodySize::None,
            upper_wick_size: CandleWickSize::None,
            lower_wick_size: CandleWickSize::None,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_candlesticks_creation() {
        let fc = FuzzyCandlesticks::new(20, 0.5, 1.0, 1.5, 2.0);
        assert!(!fc.is_ready());
        assert_eq!(fc.fuzzy_value().direction, CandleDirection::None);
    }

    #[test]
    fn test_fuzzy_candlesticks_warmup() {
        let mut fc = FuzzyCandlesticks::new(20, 0.5, 1.0, 1.5, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            fc.update_bar(price, price + 1.0, price - 1.0, price + 0.5, 1000.0);
        }
        assert!(fc.is_ready());
    }

    #[test]
    fn test_fuzzy_candlesticks_bull() {
        let mut fc = FuzzyCandlesticks::new(20, 0.5, 1.0, 1.5, 2.0);
        for i in 0..25 {
            let open = 100.0 + i as f64;
            let close = open + 2.0;
            fc.update_bar(open, close + 0.5, open - 0.5, close, 1000.0);
        }
        assert_eq!(fc.fuzzy_value().direction, CandleDirection::Bull);
    }

    #[test]
    fn test_fuzzy_candlesticks_bear() {
        let mut fc = FuzzyCandlesticks::new(20, 0.5, 1.0, 1.5, 2.0);
        for i in 0..25 {
            let open = 100.0 + i as f64;
            let close = open - 2.0;
            fc.update_bar(open, open + 0.5, close - 0.5, close, 1000.0);
        }
        assert_eq!(fc.fuzzy_value().direction, CandleDirection::Bear);
    }

    #[test]
    fn test_fuzzy_candlesticks_reset() {
        let mut fc = FuzzyCandlesticks::new(20, 0.5, 1.0, 1.5, 2.0);
        for i in 0..25 {
            fc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        fc.reset();
        assert!(!fc.is_ready());
        assert_eq!(fc.fuzzy_value().direction, CandleDirection::None);
    }
}






















