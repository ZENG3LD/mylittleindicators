// Swing Age: bars since last local high/low (naive HH/LL detectors)

#[derive(Clone)]
pub struct SwingAge {
    lookback: usize,
    highs: Vec<f64>,
    lows: Vec<f64>,
    idx: usize,
    filled: bool,
    pub age_since_high: usize,
    pub age_since_low: usize,
}

impl SwingAge {
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback: lookback.max(2),
            highs: vec![0.0; lookback.max(2)],
            lows: vec![0.0; lookback.max(2)],
            idx: 0,
            filled: false,
            age_since_high: 0,
            age_since_low: 0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.highs.fill(0.0);
        self.lows.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.age_since_high = 0;
        self.age_since_low = 0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    #[inline]
    pub fn value(&self) -> crate::IndicatorValue {
        crate::IndicatorValue::Double(
            self.age_since_high as f64,
            self.age_since_low as f64,
        )
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        high: f64,
        low: f64,
        _close: f64,
        _volume: f64,
    ) -> (usize, usize) {
        // update rings
        self.highs[self.idx] = high;
        self.lows[self.idx] = low;
        self.idx = (self.idx + 1) % self.lookback;
        if self.idx == 0 {
            self.filled = true;
        }
        if !self.filled {
            return (self.age_since_high, self.age_since_low);
        }

        let len = self.lookback;
        let curr_i = (self.idx + len - 1) % len;
        let h = self.highs[curr_i];
        let l = self.lows[curr_i];
        // if new HH/LL versus previous len-1
        let mut prev_max = f64::MIN;
        let mut prev_min = f64::MAX;
        for k in 1..len {
            let i = (self.idx + len - 1 - k) % len;
            prev_max = prev_max.max(self.highs[i]);
            prev_min = prev_min.min(self.lows[i]);
        }
        if h >= prev_max {
            self.age_since_high = 0;
        } else {
            self.age_since_high = self.age_since_high.saturating_add(1);
        }
        if l <= prev_min {
            self.age_since_low = 0;
        } else {
            self.age_since_low = self.age_since_low.saturating_add(1);
        }
        (self.age_since_high, self.age_since_low)
    }

    pub fn lookback(&self) -> usize {
        self.lookback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swing_age_creation() {
        let sa = SwingAge::new(14);
        assert!(!sa.is_ready());
        assert_eq!(sa.age_since_high, 0);
        assert_eq!(sa.age_since_low, 0);
        assert_eq!(sa.lookback(), 14);
    }

    #[test]
    fn test_swing_age_basic() {
        let mut sa = SwingAge::new(14);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            sa.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sa.is_ready());
    }

    #[test]
    fn test_swing_age_new_high_resets() {
        let mut sa = SwingAge::new(10);
        for i in 1..=20 {
            let price = 100.0 + i as f64 * 2.0;
            let (age_h, _) = sa.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if sa.is_ready() {
                assert_eq!(age_h, 0, "In uptrend, new highs should keep resetting age");
            }
        }
    }

    #[test]
    fn test_swing_age_reset() {
        let mut sa = SwingAge::new(14);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            sa.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(sa.is_ready());
        sa.reset();
        assert!(!sa.is_ready());
        assert_eq!(sa.age_since_high, 0);
        assert_eq!(sa.age_since_low, 0);
    }
}
