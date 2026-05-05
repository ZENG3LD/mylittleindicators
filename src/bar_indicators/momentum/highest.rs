// High-performance Highest indicator
// Поиск максимума за N периодов с эффективным буфером
// (c) 2024

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct Highest {
    period: usize,
    buffer: ArrayVec<f64, 512>,
    index: usize,
    filled: bool,
    value: f64,
}

impl Highest {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: ArrayVec::new(),
            index: 0,
            filled: false,
            value: 0.0,
        }
    }
    
    /// Обновить Highest новым баром (используется high)
    pub fn update_bar(&mut self, _open: f64, high: f64, _low: f64, _close: f64, _volume: f64) -> f64 {
        self.update(high)
    }
    
    /// Обновить Highest новым значением
    pub fn update(&mut self, value: f64) -> f64 {
        if self.buffer.len() < self.period {
            self.buffer.push(value);
        } else {
            // Циклический буфер - заменяем старое значение
            self.buffer[self.index] = value;
            self.index = (self.index + 1) % self.period;
            self.filled = true;
        }
        
        // Эффективный поиск максимума в буфере
        self.value = self.buffer.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        self.value
    }
    
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    
    pub fn is_ready(&self) -> bool {
        self.filled || self.buffer.len() == self.period
    }
    
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.index = 0;
        self.filled = false;
        self.value = 0.0;
    }
    
    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highest_creation() {
        let highest = Highest::new(10);
        assert!(!highest.is_ready());
        assert_eq!(highest.value().main(), 0.0);
        assert_eq!(highest.period(), 10);
    }

    #[test]
    fn test_highest_finds_max() {
        let mut highest = Highest::new(5);
        highest.update(10.0);
        highest.update(20.0);
        highest.update(15.0);
        highest.update(25.0);
        highest.update(18.0);
        assert!(highest.is_ready());
        assert_eq!(highest.value().main(), 25.0);
    }

    #[test]
    fn test_highest_rolling_window() {
        let mut highest = Highest::new(3);
        highest.update(10.0);
        highest.update(20.0);
        highest.update(15.0);
        assert_eq!(highest.value().main(), 20.0);
        highest.update(5.0); // pushes out 10
        assert_eq!(highest.value().main(), 20.0);
        highest.update(3.0); // pushes out 20
        assert_eq!(highest.value().main(), 15.0);
    }

    #[test]
    fn test_highest_update_bar() {
        let mut highest = Highest::new(5);
        for i in 1..=10 {
            let high = 100.0 + i as f64;
            highest.update_bar(95.0, high, 90.0, 98.0, 1000.0);
        }
        assert!(highest.is_ready());
        // Last 5 highs: 106, 107, 108, 109, 110
        assert_eq!(highest.value().main(), 110.0);
    }

    #[test]
    fn test_highest_reset() {
        let mut highest = Highest::new(5);
        for i in 1..=10 {
            highest.update(100.0 + i as f64);
        }
        assert!(highest.is_ready());
        highest.reset();
        assert!(!highest.is_ready());
        assert_eq!(highest.value().main(), 0.0);
    }
}






















