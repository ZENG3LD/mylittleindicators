// ZigZag by time window (bars)
// (c) 2024

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct ZigZagTime {
    pub min_bars: usize,
    pub period: usize,
    pub buffer: ArrayVec<f64, 512>,
    pub swings: ArrayVec<(usize, f64), 512>,
    pub last_extreme: f64,
    pub last_extreme_idx: usize,
    pub direction: i8,
    bar_counter: usize,
}

impl ZigZagTime {
    pub fn new(period: usize, min_bars: usize) -> Self {
        Self {
            min_bars,
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
            self.swings.push((idx, close));
            return;
        }
        if idx - self.last_extreme_idx >= self.min_bars {
            self.direction = if close > self.last_extreme { 1 } else { -1 };
            self.last_extreme = close;
            self.last_extreme_idx = idx;
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
    fn test_zigzag_time_creation() {
        let ind = ZigZagTime::new(50, 5);
        assert!(!ind.is_ready());
        assert!(ind.swings().is_empty());
    }

    #[test]
    fn test_zigzag_time_warmup() {
        let mut ind = ZigZagTime::new(20, 3);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ind.update(price, i);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_zigzag_time_swings() {
        let mut ind = ZigZagTime::new(30, 5);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            ind.update(price, i);
        }
        assert!(ind.swings().len() >= 2);
    }

    #[test]
    fn test_zigzag_time_reset() {
        let mut ind = ZigZagTime::new(20, 3);
        for i in 0..30 {
            ind.update(100.0 + i as f64, i);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert!(ind.swings().is_empty());
    }
}






















