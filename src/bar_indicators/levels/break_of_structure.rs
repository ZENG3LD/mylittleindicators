// Break of Structure (BOS) / Change of Character (CHOCH) detector

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct BosChochDetector {
    lookback: usize,
    highs: Vec<f64>,
    lows: Vec<f64>,
    idx: usize,
    filled: bool,
    pub bos_up: bool,
    pub bos_down: bool,
}

impl BosChochDetector {
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback: lookback.max(2),
            highs: vec![0.0; lookback.max(2)],
            lows: vec![0.0; lookback.max(2)],
            idx: 0,
            filled: false,
            bos_up: false,
            bos_down: false,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.highs.fill(0.0);
        self.lows.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.bos_up = false;
        self.bos_down = false;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        high: f64,
        low: f64,
        _close: f64,
        _volume: f64,
    ) -> (bool, bool) {
        self.highs[self.idx] = high;
        self.lows[self.idx] = low;
        self.idx = (self.idx + 1) % self.lookback;
        if self.idx == 0 {
            self.filled = true;
        }
        if !self.filled {
            return (false, false);
        }

        // reference prev swing extremes as max/min over window-1 preceding bars
        let len = self.lookback;
        let mut prev_max = f64::MIN;
        let mut prev_min = f64::MAX;
        for k in 1..len {
            // exclude current index
            let i = (self.idx + len - 1 - k) % len;
            if self.highs[i] > prev_max {
                prev_max = self.highs[i];
            }
            if self.lows[i] < prev_min {
                prev_min = self.lows[i];
            }
        }
        let curr_i = (self.idx + len - 1) % len;
        let h = self.highs[curr_i];
        let l = self.lows[curr_i];
        self.bos_up = h > prev_max;
        self.bos_down = l < prev_min;
        (self.bos_up, self.bos_down)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::DoubleFlag(self.bos_up, self.bos_down)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bos_choch_creation() {
        let bos = BosChochDetector::new(10);
        assert!(!bos.is_ready());
        assert!(!bos.bos_up);
        assert!(!bos.bos_down);
    }

    #[test]
    fn test_bos_choch_warmup() {
        let mut bos = BosChochDetector::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            bos.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(bos.is_ready());
    }

    #[test]
    fn test_bos_choch_break_up() {
        let mut bos = BosChochDetector::new(5);
        // Fill with stable prices
        for _ in 0..5 {
            bos.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0);
        }
        // Break above previous high
        let (up, _down) = bos.update_bar(105.0, 110.0, 104.0, 108.0, 1000.0);
        assert!(up, "Should detect break of structure up");
    }

    #[test]
    fn test_bos_choch_break_down() {
        let mut bos = BosChochDetector::new(5);
        // Fill with stable prices
        for _ in 0..5 {
            bos.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0);
        }
        // Break below previous low
        let (_up, down) = bos.update_bar(95.0, 96.0, 90.0, 92.0, 1000.0);
        assert!(down, "Should detect break of structure down");
    }

    #[test]
    fn test_bos_choch_reset() {
        let mut bos = BosChochDetector::new(10);
        for i in 0..15 {
            bos.update_bar(100.0 + i as f64, 101.0 + i as f64, 99.0, 100.0, 1000.0);
        }
        bos.reset();
        assert!(!bos.is_ready());
        assert!(!bos.bos_up);
        assert!(!bos.bos_down);
    }
}
