// Classic ZigZag indicator (percent/absolute threshold)
// (c) 2024

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct ZigZagClassic {
    pub threshold_percent: Option<f64>,
    pub threshold_abs: Option<f64>,
    pub period: usize,
    pub buffer: ArrayVec<f64, 512>, // последние close
    pub swings: ArrayVec<(usize, f64), 512>, // последние swing точки (скользящее окно)
    pub last_extreme: f64,
    pub last_extreme_idx: usize,
    pub direction: i8, // 1: up, -1: down
    bar_counter: usize, // internal bar counter for update_bar
}

impl ZigZagClassic {
    pub fn new(period: usize, threshold_percent: Option<f64>, threshold_abs: Option<f64>) -> Self {
        Self {
            threshold_percent,
            threshold_abs,
            period,
            buffer: ArrayVec::new(),
            swings: ArrayVec::new(),
            last_extreme: 0.0,
            last_extreme_idx: 0,
            direction: 0,
            bar_counter: 0,
        }
    }

    /// Update with OHLCV bar (uses close price)
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> f64 {
        self.update(close, self.bar_counter);
        self.bar_counter += 1;
        self.last_swing().map(|(_, price)| price).unwrap_or(close)
    }

    /// Get current indicator value
    pub fn value(&self) -> IndicatorValue {
        let price = self.last_swing().map(|(_, price)| price).unwrap_or(0.0);
        IndicatorValue::Single(price)
    }
    /// Обновить индикатор новым баром (close)
    pub fn update(&mut self, close: f64, idx: usize) {
        if self.buffer.len() < self.period {
            self.buffer.push(close);
        } else {
            self.buffer.remove(0);
            self.buffer.push(close);
        }
        if self.direction == 0 {
            self.last_extreme = close;
            self.last_extreme_idx = idx;
            self.direction = 1;
            // Добавляем в буфер swings с проверкой переполнения
            if self.swings.len() >= 512 {
                self.swings.remove(0);
            }
            self.swings.push((idx, close));
            return;
        }
        let change = close - self.last_extreme;
        let percent = if let Some(p) = self.threshold_percent {
            (change / self.last_extreme).abs() * 100.0 >= p
        } else { false };
        let abs = if let Some(a) = self.threshold_abs {
            change.abs() >= a
        } else { false };
        if percent || abs {
            self.direction = if change > 0.0 { 1 } else { -1 };
            self.last_extreme = close;
            self.last_extreme_idx = idx;
            // Добавляем в буфер swings с проверкой переполнения
            if self.swings.len() >= 512 {
                self.swings.remove(0);
            }
            self.swings.push((idx, close));
        }
    }
    pub fn last_swing(&self) -> Option<(usize, f64)> {
        self.swings.last().copied()
    }
    pub fn swings(&self) -> &ArrayVec<(usize, f64), 512> {
        &self.swings
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.swings.len() >= 2
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.swings.clear();
        self.last_extreme = 0.0;
        self.last_extreme_idx = 0;
        self.direction = 0;
        self.bar_counter = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zigzag_classic_creation() {
        let ind = ZigZagClassic::new(50, Some(5.0), None);
        assert!(!ind.is_ready());
        assert!(ind.swings().is_empty());
    }

    #[test]
    fn test_zigzag_classic_warmup() {
        let mut ind = ZigZagClassic::new(20, Some(3.0), None);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 15.0;
            ind.update(price, i);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_zigzag_classic_swings() {
        let mut ind = ZigZagClassic::new(20, Some(5.0), None);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.4).sin() * 20.0;
            ind.update(price, i);
        }
        assert!(ind.swings().len() >= 2);
    }

    #[test]
    fn test_zigzag_classic_reset() {
        let mut ind = ZigZagClassic::new(20, Some(3.0), None);
        for i in 0..30 {
            ind.update(100.0 + i as f64, i);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert!(ind.swings().is_empty());
    }
}






















