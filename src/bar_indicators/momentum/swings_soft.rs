// SwingsSoft: реализация swings с "мягкой" логикой (идентично Nautilus)
// (c) 2024

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SwingsSoft {
    period: usize,
    highs: ArrayVec<f64, 512>,
    lows: ArrayVec<f64, 512>,
    filled: bool,
    direction: i8,
    high_price: f64,
    low_price: f64,
    last_highs: Vec<f64>, // стек последних swing high
    last_lows: Vec<f64>,  // стек последних swing low
} 

impl SwingsSoft {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            filled: false,
            direction: 0,
            high_price: 0.0,
            low_price: 0.0,
            last_highs: Vec::new(),
            last_lows: Vec::new(),
        }
    }
    /// Мягкая логика (Nautilus style)
    /// Хранит стек последних swing high/low для анализа
    pub fn update_bar(&mut self, high: f64, low: f64, _close: f64, _volume: f64) -> i8 {
        if self.highs.len() < self.period {
            self.highs.push(high);
            self.lows.push(low);
        } else {
            self.highs.remove(0);
            self.lows.remove(0);
            self.highs.push(high);
            self.lows.push(low);
            self.filled = true;
        }
        let len = self.highs.len();
        if len < self.period {
            self.direction = 0;
            return self.direction;
        }
        let max_high = self.highs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min_low = self.lows.iter().copied().fold(f64::INFINITY, f64::min);
        let is_swing_high = high >= max_high && low >= min_low;
        let is_swing_low = high <= max_high && low <= min_low;
        if is_swing_high && !is_swing_low {
            if self.high_price != 0.0 {
                self.last_highs.push(self.high_price);
            }
            self.high_price = high;
            self.direction = 1;
        } else if is_swing_low && !is_swing_high {
            if self.low_price != 0.0 {
                self.last_lows.push(self.low_price);
            }
            self.low_price = low;
            self.direction = -1;
        }
        self.direction
    }
    pub fn direction(&self) -> i8 {
        self.direction
    }
    pub fn high_price(&self) -> f64 {
        self.high_price
    }
    pub fn low_price(&self) -> f64 {
        self.low_price
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.high_price, self.low_price)
    }
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn last_high(&self) -> Option<f64> {
        self.last_highs.last().copied()
    }
    pub fn last_low(&self) -> Option<f64> {
        self.last_lows.last().copied()
    }
    pub fn last_highs(&self) -> &[f64] {
        &self.last_highs
    }
    pub fn last_lows(&self) -> &[f64] {
        &self.last_lows
    }
    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.filled = false;
        self.direction = 0;
        self.high_price = 0.0;
        self.low_price = 0.0;
        self.last_highs.clear();
        self.last_lows.clear();
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swings_soft_creation() {
        let swings = SwingsSoft::new(10);
        assert!(!swings.is_ready());
        assert_eq!(swings.direction(), 0);
        assert_eq!(swings.period(), 10);
    }

    #[test]
    fn test_swings_soft_uptrend() {
        let mut swings = SwingsSoft::new(10);
        for i in 1..=20 {
            let price = 100.0 + i as f64 * 2.0;
            swings.update_bar(price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(swings.is_ready());
    }

    #[test]
    fn test_swings_soft_finite() {
        let mut swings = SwingsSoft::new(10);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let dir = swings.update_bar(price + 2.0, price - 2.0, price, 1000.0);
            assert!(dir >= -1 && dir <= 1);
        }
    }

    #[test]
    fn test_swings_soft_reset() {
        let mut swings = SwingsSoft::new(10);
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            swings.update_bar(price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(swings.is_ready());
        swings.reset();
        assert!(!swings.is_ready());
        assert_eq!(swings.direction(), 0);
    }
}






















