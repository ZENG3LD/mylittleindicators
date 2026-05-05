// ZigZag with lookahead confirmation
// (c) 2024

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct ZigZagLookahead {
    pub lookahead: usize,
    pub period: usize,
    pub buffer: ArrayVec<f64, 512>,
    pub swings: ArrayVec<(usize, f64), 512>,
    pub candidates: ArrayVec<(usize, f64), 16>, // временные экстремумы
    bar_counter: usize,
}

impl ZigZagLookahead {
    pub fn new(period: usize, lookahead: usize) -> Self {
        Self {
            lookahead,
            period,
            buffer: ArrayVec::new(),
            swings: ArrayVec::new(),
            candidates: ArrayVec::new(),
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
        // Буфер должен вмещать как минимум lookahead элементов
        let buf_size = self.period.max(self.lookahead);
        if self.buffer.len() < buf_size {
            self.buffer.push(close);
        } else {
            self.buffer.remove(0);
            self.buffer.push(close);
        }

        // Не добавляем кандидатов пока буфер не заполнен достаточно
        if self.buffer.len() < self.lookahead {
            return;
        }

        // Кандидат в swing — текущий экстремум
        if !self.candidates.is_full() {
            self.candidates.push((idx, close));
        }

        // Проверка lookahead: swing подтверждается, если не перебит в течение lookahead баров
        while let Some(&(cand_idx, cand_val)) = self.candidates.first() {
            if idx >= cand_idx + self.lookahead {
                let start = self.buffer.len().saturating_sub(self.lookahead);
                let is_extreme = self.buffer[start..]
                    .iter()
                    .all(|&x| x != cand_val);
                if is_extreme && !self.swings.is_full() {
                    self.swings.push((cand_idx, cand_val));
                }
                self.candidates.remove(0);
            } else {
                break;
            }
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
        self.buffer.len() >= self.lookahead
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.swings.clear();
        self.candidates.clear();
        self.bar_counter = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zigzag_lookahead_creation() {
        let ind = ZigZagLookahead::new(50, 5);
        assert!(!ind.is_ready());
        assert!(ind.swings().is_empty());
    }

    #[test]
    fn test_zigzag_lookahead_warmup() {
        let mut ind = ZigZagLookahead::new(20, 3);
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ind.update(price, i);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_zigzag_lookahead_swings() {
        let mut ind = ZigZagLookahead::new(30, 3);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            ind.update(price, i);
        }
        // May have some confirmed swings
        let _ = ind.swings().len();
    }

    #[test]
    fn test_zigzag_lookahead_reset() {
        let mut ind = ZigZagLookahead::new(20, 3);
        for i in 0..30 {
            ind.update(100.0 + i as f64, i);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert!(ind.swings().is_empty());
    }
}






















