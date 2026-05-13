// Swing/Fractal strength score: normalized strength of recent swing

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct SwingStrengthScore {
    left: usize,
    right: usize,
    value: f64,
    highs: Vec<f64>,
    lows: Vec<f64>,
    idx: usize,
    filled: bool,
}

impl SwingStrengthScore {
    pub fn new(left: usize, right: usize) -> Self {
        let l = left.clamp(1, 10);
        let r = right.clamp(1, 10);
        let cap = (l + r + 2).max(16);
        Self {
            left: l,
            right: r,
            value: 0.0,
            highs: vec![0.0; cap],
            lows: vec![0.0; cap],
            idx: 0,
            filled: false,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.idx = 0;
        self.filled = false;
        self.highs.fill(0.0);
        self.lows.fill(0.0);
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, _c: f64, _v: f64) -> f64 {
        let n = self.highs.len();
        self.highs[self.idx] = h;
        self.lows[self.idx] = l;
        self.idx = (self.idx + 1) % n;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            // check pivot at position left back from current
            let pivot_idx = (self.idx + n - self.right - 1) % n; // recent pivot
            let pivot_high = self.highs[pivot_idx];
            let pivot_low = self.lows[pivot_idx];
            // left/right ranges
            let mut left_max = f64::NEG_INFINITY;
            let mut left_min = f64::INFINITY;
            for i in 1..=self.left {
                let ii = (pivot_idx + n - i) % n;
                left_max = left_max.max(self.highs[ii]);
                left_min = left_min.min(self.lows[ii]);
            }
            let mut right_max = f64::NEG_INFINITY;
            let mut right_min = f64::INFINITY;
            for i in 1..=self.right {
                let ii = (pivot_idx + i) % n;
                right_max = right_max.max(self.highs[ii]);
                right_min = right_min.min(self.lows[ii]);
            }
            let up_strength = (pivot_high - left_max).max(0.0) + (pivot_high - right_max).max(0.0);
            let down_strength = (left_min - pivot_low).max(0.0) + (right_min - pivot_low).max(0.0);
            let raw = up_strength - down_strength;
            // normalize by recent ATR proxy (range average)
            let mut rng_sum = 0.0;
            for i in 0..(self.left + self.right + 1) {
                let ii = (pivot_idx + n - self.left + i) % n;
                rng_sum += self.highs[ii] - self.lows[ii];
            }
            let denom = (rng_sum / (self.left + self.right + 1) as f64).max(1e-6);
            self.value = (raw / denom).tanh();
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swing_strength_score_creation() {
        let sss = SwingStrengthScore::new(3, 3);
        assert!(!sss.is_ready());
    }

    #[test]
    fn test_swing_strength_score_warmup() {
        let mut sss = SwingStrengthScore::new(3, 3);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            sss.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sss.is_ready());
    }

    #[test]
    fn test_swing_strength_score_range() {
        let mut sss = SwingStrengthScore::new(3, 3);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = sss.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            // tanh output is in [-1, 1]
            assert!(value >= -1.0 && value <= 1.0, "Value should be in [-1, 1]");
        }
    }

    #[test]
    fn test_swing_strength_score_reset() {
        let mut sss = SwingStrengthScore::new(3, 3);
        for i in 0..20 {
            sss.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0, 1000.0);
        }
        sss.reset();
        assert!(!sss.is_ready());
    }
}
