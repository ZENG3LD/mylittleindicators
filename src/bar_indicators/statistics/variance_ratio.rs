// Rolling Lo-MacKinlay Variance Ratio test (simple m-step version) on returns

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct VarianceRatio {
    window: usize,
    m: usize,
    vals: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    ratio: f64,
}

impl VarianceRatio {
    pub fn new(window: usize, m: usize) -> Self {
        let w = window.max(20);
        let step = m.max(2).min(w / 2);
        Self {
            window: w,
            m: step,
            vals: vec![0.0; w],
            idx: 0,
            filled: false,
            last_close: None,
            ratio: 1.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.vals.fill(0.0);
        self.last_close = None;
        self.ratio = 1.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> f64 {
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            self.vals[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if self.idx == 0 {
                self.filled = true;
            }
            if self.filled {
                self.ratio = self.compute_ratio();
            }
        }
        self.last_close = Some(close);
        self.ratio
    }

    fn compute_ratio(&self) -> f64 {
        let n = self.window;
        // variance of 1-step returns
        let mut mean = 0.0;
        for i in 0..n {
            mean += self.vals[i];
        }
        mean /= n as f64;
        let mut var1 = 0.0;
        for i in 0..n {
            let d = self.vals[i] - mean;
            var1 += d * d;
        }
        var1 /= n as f64;
        if var1 <= 1e-12 {
            return 1.0;
        }
        // variance of m-step cumulative returns (overlapping)
        let m = self.m;
        let mut sums = Vec::with_capacity(n - m + 1);
        let mut cur = 0.0;
        for i in 0..m {
            cur += self.vals[(self.idx + i) % n];
        }
        sums.push(cur);
        for i in m..n {
            cur += self.vals[(self.idx + i) % n] - self.vals[(self.idx + i - m) % n];
            sums.push(cur);
        }
        let count = sums.len();
        let mut smean = 0.0;
        for v in &sums {
            smean += *v;
        }
        smean /= count as f64;
        let mut varm = 0.0;
        for v in &sums {
            let d = *v - smean;
            varm += d * d;
        }
        varm /= count as f64;
        (varm / var1) / (m as f64)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.ratio)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variance_ratio_creation() {
        let vr = VarianceRatio::new(50, 5);
        assert!(!vr.is_ready());
        assert_eq!(vr.value().main(), 1.0);
    }

    #[test]
    fn test_variance_ratio_warmup() {
        let mut vr = VarianceRatio::new(50, 5);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vr.is_ready());
    }

    #[test]
    fn test_variance_ratio_positive() {
        let mut vr = VarianceRatio::new(50, 5);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value > 0.0, "VR should be positive");
        }
    }

    #[test]
    fn test_variance_ratio_reset() {
        let mut vr = VarianceRatio::new(50, 5);
        for i in 0..60 {
            vr.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        vr.reset();
        assert!(!vr.is_ready());
        assert_eq!(vr.value().main(), 1.0);
    }
}
