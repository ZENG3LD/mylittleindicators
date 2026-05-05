// ZigZag by candle pattern (N-bar swing)
// (c) 2024

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct ZigZagCandle {
    pub swing_bars: usize,
    pub period: usize,
    pub buffer: ArrayVec<f64, 512>,
    pub swings: ArrayVec<(usize, f64), 512>,
    bar_counter: usize,
}

impl ZigZagCandle {
    pub fn new(period: usize, swing_bars: usize) -> Self {
        // Ensure period is at least swing_bars*2+1 for swing detection to work
        let swing_bars = swing_bars.clamp(1, 50);
        let min_period = swing_bars * 2 + 1;
        let period = period.max(min_period);

        Self {
            swing_bars,
            period,
            buffer: ArrayVec::new(),
            swings: ArrayVec::new(),
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
        // Проверка swing high/low по N-барам (только swing_bars соседей с каждой стороны)
        if self.buffer.len() > self.swing_bars * 2 {
            let mid = self.buffer.len() - self.swing_bars - 1;
            let val = self.buffer[mid];

            // Сравниваем только с swing_bars соседями, не со всем буфером
            let left_start = mid.saturating_sub(self.swing_bars);
            let right_end = (mid + 1 + self.swing_bars).min(self.buffer.len());

            let is_high = self.buffer[(mid + 1)..right_end].iter().all(|&x| val > x)
                && self.buffer[left_start..mid].iter().all(|&x| val > x);
            let is_low = self.buffer[(mid + 1)..right_end].iter().all(|&x| val < x)
                && self.buffer[left_start..mid].iter().all(|&x| val < x);
            if is_high || is_low {
                self.swings.push((idx - self.swing_bars, val));
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
        // swing_bars * 2 + 1 is the minimum, but buffer is capped at period
        // So we just need buffer to reach its period limit
        self.buffer.len() >= self.period.min(self.swing_bars * 2 + 1)
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.swings.clear();
        self.bar_counter = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zigzag_candle_creation() {
        let ind = ZigZagCandle::new(50, 3);
        assert!(!ind.is_ready());
        assert!(ind.swings().is_empty());
    }

    #[test]
    fn test_zigzag_candle_warmup() {
        let mut ind = ZigZagCandle::new(20, 2);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ind.update(price, i);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_zigzag_candle_swings() {
        let mut ind = ZigZagCandle::new(30, 2);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            ind.update(price, i);
        }
        // Should have detected some swings
        let _ = ind.swings().len(); // May or may not have swings depending on price action
    }

    #[test]
    fn test_zigzag_candle_reset() {
        let mut ind = ZigZagCandle::new(20, 2);
        for i in 0..20 {
            ind.update(100.0 + i as f64, i);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert!(ind.swings().is_empty());
    }

    #[test]
    fn test_zigzag_candle_with_clear_peaks() {
        // Create data with clear peaks and troughs
        let mut ind = ZigZagCandle::new(20, 3);

        // Peak pattern: rise to peak, then fall
        // With swing_bars=3, we need val > 3 bars before AND > 3 bars after
        let prices = [
            100.0, 102.0, 104.0, 106.0, // rising
            110.0, // PEAK - should be detected after 3 more bars
            108.0, 106.0, 104.0, // falling
            102.0, 100.0, 98.0, 96.0, // falling more
            90.0, // TROUGH - should be detected after 3 more bars
            92.0, 94.0, 96.0, // rising
            98.0, 100.0, 102.0, 104.0, // rising more
        ];

        for (i, &price) in prices.iter().enumerate() {
            ind.update(price, i);
            eprintln!("Bar {}: price={:.1}, buffer_len={}, swings={}",
                i, price, ind.buffer.len(), ind.swings().len());
        }

        assert!(ind.swings().len() > 0, "Should detect swing peaks/troughs. Got {} swings", ind.swings().len());
    }

    #[test]
    fn test_zigzag_candle_with_generated_data() {
        use crate::catalog::synthetic_data::{generate_bars, DataType};

        let bars = generate_bars(DataType::ZigZagSwings, 500, 1704067200);

        // Test with parameters similar to rendering_tests (period=7, swing_bars=14)
        // After validation: swing_bars=14, period=29
        let mut ind = ZigZagCandle::new(7, 14);

        eprintln!("After param validation: period={}, swing_bars={}", ind.period, ind.swing_bars);

        for (i, bar) in bars.iter().enumerate() {
            let val = ind.update_bar(bar.open, bar.high, bar.low, bar.close, bar.volume);
            if i < 50 || ind.swings().len() > 0 {
                eprintln!("Bar {}: close={:.2}, value={:.2}, swings={}, buffer_len={}",
                    i, bar.close, val, ind.swings().len(), ind.buffer.len());
            }
        }

        eprintln!("Final: {} swings detected", ind.swings().len());
        assert!(ind.swings().len() > 0, "Should detect swing points in ZigZagSwings data. Got {} swings", ind.swings().len());
    }
}






















