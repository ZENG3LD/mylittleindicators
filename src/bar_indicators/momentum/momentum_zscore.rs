// Momentum Z-Score: zscore of close change over lookback with rolling window

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct MomentumZscore {
    diff_period: usize,
    window: usize,
    closes: Vec<f64>,
    idx: usize,
    filled: bool,
    // rolling stats for diffs
    diffs: Vec<f64>,
    didx: usize,
    dfilled: bool,
    sum: f64,
    sumsq: f64,
    value: f64,
}

impl MomentumZscore {
    pub fn new(diff_period: usize, window: usize) -> Self {
        Self {
            diff_period: diff_period.max(1),
            window: window.max(2),
            closes: vec![0.0; diff_period.max(1) + window.max(2)],
            idx: 0,
            filled: false,
            diffs: vec![0.0; window.max(2)],
            didx: 0,
            dfilled: false,
            sum: 0.0,
            sumsq: 0.0,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.closes.fill(0.0);
        self.diffs.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.didx = 0;
        self.dfilled = false;
        self.sum = 0.0;
        self.sumsq = 0.0;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.dfilled
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> f64 {
        let cbuf = self.closes.len();
        self.closes[self.idx % cbuf] = close;

        // compute diff if possible
        if self.idx >= self.diff_period {
            let prev = self.closes[(self.idx - self.diff_period) % cbuf];
            let diff = close - prev;

            // rolling stats on diffs
            let old = self.diffs[self.didx];
            self.diffs[self.didx] = diff;
            self.didx = (self.didx + 1) % self.window;
            if self.didx == 0 {
                self.dfilled = true;
            }

            self.sum += diff - old;
            self.sumsq += diff * diff - old * old;

            let n = if self.dfilled {
                self.window as f64
            } else {
                self.didx as f64
            };
            if n >= 2.0 {
                let mean = self.sum / n;
                let var = (self.sumsq / n) - mean * mean;
                let std = if var > 0.0 { var.sqrt() } else { 0.0 };
                self.value = if std > 1e-12 {
                    (diff - mean) / std
                } else {
                    0.0
                };
            } else {
                self.value = 0.0;
            }
        }

        self.idx += 1;
        if self.idx >= cbuf {
            self.filled = true;
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn diff_period(&self) -> usize {
        self.diff_period
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_momentum_zscore_creation() {
        let mz = MomentumZscore::new(10, 20);
        assert!(!mz.is_ready());
        assert_eq!(mz.value().main(), 0.0);
        assert_eq!(mz.diff_period(), 10);
        assert_eq!(mz.window(), 20);
    }

    #[test]
    fn test_momentum_zscore_basic() {
        let mut mz = MomentumZscore::new(10, 20);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            mz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(mz.is_ready());
        assert!(mz.value().main().is_finite());
    }

    #[test]
    fn test_momentum_zscore_reset() {
        let mut mz = MomentumZscore::new(10, 20);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            mz.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(mz.is_ready());
        mz.reset();
        assert!(!mz.is_ready());
        assert_eq!(mz.value().main(), 0.0);
    }

    #[test]
    fn test_momentum_zscore_finite_values() {
        let mut mz = MomentumZscore::new(10, 20);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = mz.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "Momentum Zscore should always be finite");
        }
    }
}
