// Percentile Rate-of-Change over window

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct RocPercentile {
    period: usize,
    window: usize,
    closes: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,      // current ROC
    percentile: f64, // percentile of ROC within window [0..1]
}

impl RocPercentile {
    pub fn new(period: usize, window: usize) -> Self {
        Self {
            period: period.max(1),
            window: window.max(1),
            closes: vec![0.0; window.max(1) + period.max(1)],
            idx: 0,
            filled: false,
            value: 0.0,
            percentile: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.closes.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
        self.percentile = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.idx >= self.period
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> (f64, f64) {
        // push close
        let buf_len = self.closes.len();
        self.closes[self.idx % buf_len] = close;

        // compute ROC if possible
        if self.idx >= self.period {
            let prev = self.closes[(self.idx - self.period) % buf_len];
            self.value = if prev.abs() > 1e-12 {
                (close - prev) / prev
            } else {
                0.0
            };
        } else {
            self.value = 0.0;
        }

        self.idx += 1;
        if self.idx >= buf_len {
            self.filled = true;
        }

        // Percentile over last window ROCs
        let mut count = 0usize;
        let mut count_le = 0usize;
        if self.idx > self.period {
            let start = self.idx.saturating_sub(self.window);
            for j in start..self.idx {
                if j <= self.period {
                    continue;
                }
                let prev = self.closes[(j - self.period) % buf_len];
                let curr = self.closes[j % buf_len];
                let roc = if prev.abs() > 1e-12 {
                    (curr - prev) / prev
                } else {
                    0.0
                };
                count += 1;
                if roc <= self.value {
                    count_le += 1;
                }
            }
        }
        self.percentile = if count > 0 {
            count_le as f64 / count as f64
        } else {
            0.0
        };
        (self.value, self.percentile)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    #[inline]
    pub fn percentile(&self) -> f64 {
        self.percentile
    }

    pub fn period(&self) -> usize {
        self.period
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roc_percentile_creation() {
        let rp = RocPercentile::new(10, 50);
        assert!(!rp.is_ready());
        assert_eq!(rp.value().main(), 0.0);
        assert_eq!(rp.period(), 10);
        assert_eq!(rp.window(), 50);
    }

    #[test]
    fn test_roc_percentile_basic() {
        let mut rp = RocPercentile::new(10, 50);
        for i in 1..=100 {
            let price = 100.0 + i as f64;
            rp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rp.is_ready());
        assert!(rp.value().main().is_finite());
        assert!(rp.percentile() >= 0.0 && rp.percentile() <= 1.0);
    }

    #[test]
    fn test_roc_percentile_reset() {
        let mut rp = RocPercentile::new(10, 50);
        for i in 1..=100 {
            let price = 100.0 + i as f64;
            rp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(rp.is_ready());
        rp.reset();
        assert!(!rp.is_ready());
        assert_eq!(rp.value().main(), 0.0);
    }

    #[test]
    fn test_roc_percentile_finite_values() {
        let mut rp = RocPercentile::new(10, 50);
        for i in 1..=150 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let (roc, pct) = rp.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(roc.is_finite(), "ROC should always be finite");
            assert!(pct.is_finite(), "Percentile should always be finite");
        }
    }
}
