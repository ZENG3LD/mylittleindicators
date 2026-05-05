// High-performance Lowest indicator
// Поиск минимума за N периодов с эффективным буфером
// (c) 2024

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct Lowest {
    period: usize,
    buffer: ArrayVec<f64, 512>,
    index: usize,
    filled: bool,
    value: f64,
}

impl Lowest {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: ArrayVec::new(),
            index: 0,
            filled: false,
            value: 0.0,
        }
    }
    
    /// Обновить Lowest новым баром (используется low)
    pub fn update_bar(&mut self, _open: f64, _high: f64, low: f64, _close: f64, _volume: f64) -> f64 {
        self.update(low)
    }
    
    /// Обновить Lowest новым значением
    pub fn update(&mut self, value: f64) -> f64 {
        if self.buffer.len() < self.period {
            self.buffer.push(value);
        } else {
            // Циклический буфер - заменяем старое значение
            self.buffer[self.index] = value;
            self.index = (self.index + 1) % self.period;
            self.filled = true;
        }
        
        // Эффективный поиск минимума в буфере
        self.value = self.buffer.iter().copied().fold(f64::INFINITY, f64::min);
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
    fn test_lowest_creation() {
        let lowest = Lowest::new(10);
        assert!(!lowest.is_ready());
        assert_eq!(lowest.value().main(), 0.0);
        assert_eq!(lowest.period(), 10);
    }

    #[test]
    fn test_lowest_finds_min() {
        let mut lowest = Lowest::new(5);
        lowest.update(20.0);
        lowest.update(10.0);
        lowest.update(15.0);
        lowest.update(5.0);
        lowest.update(18.0);
        assert!(lowest.is_ready());
        assert_eq!(lowest.value().main(), 5.0);
    }

    #[test]
    fn test_lowest_rolling_window() {
        let mut lowest = Lowest::new(3);
        lowest.update(20.0);
        lowest.update(10.0);
        lowest.update(15.0);
        assert_eq!(lowest.value().main(), 10.0);
        lowest.update(25.0); // pushes out 20
        assert_eq!(lowest.value().main(), 10.0);
        lowest.update(30.0); // pushes out 10
        assert_eq!(lowest.value().main(), 15.0);
    }

    #[test]
    fn test_lowest_update_bar() {
        let mut lowest = Lowest::new(5);
        for i in 1..=10 {
            let low = 100.0 - i as f64;
            lowest.update_bar(95.0, 110.0, low, 98.0, 1000.0);
        }
        assert!(lowest.is_ready());
        // Last 5 lows: 94, 93, 92, 91, 90
        assert_eq!(lowest.value().main(), 90.0);
    }

    #[test]
    fn test_lowest_reset() {
        let mut lowest = Lowest::new(5);
        for i in 1..=10 {
            lowest.update(100.0 - i as f64);
        }
        assert!(lowest.is_ready());
        lowest.reset();
        assert!(!lowest.is_ready());
        assert_eq!(lowest.value().main(), 0.0);
    }
}






















