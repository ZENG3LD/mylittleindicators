// Conditional Entropy H(Y|X) proxy using discrete bins for r_t (Y) and r_{t-1} (X)

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct ConditionalEntropy {
    window: usize,
    bins: usize,
    clip_abs: f64,
    rx: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    value: f64,
}

impl ConditionalEntropy {
    pub fn new(window: usize, bins: usize, clip_abs: f64) -> Self {
        let w = window.max(20);
        let b = bins.max(4);
        Self {
            window: w,
            bins: b,
            clip_abs: clip_abs.max(1e-6),
            rx: vec![0.0; w],
            idx: 0,
            filled: false,
            last_close: None,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.last_close = None;
        self.rx.fill(0.0);
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    #[inline]
    fn bin(&self, r: f64) -> usize {
        let rr = r.max(-self.clip_abs).min(self.clip_abs);
        let x = (rr + self.clip_abs) / (2.0 * self.clip_abs);
        (x * self.bins as f64)
            .floor()
            .clamp(0.0, (self.bins - 1) as f64) as usize
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
            self.rx[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if self.idx == 0 {
                self.filled = true;
            }
            if self.filled {
                self.value = self.compute();
            }
        }
        self.last_close = Some(close);
        self.value
    }

    fn compute(&self) -> f64 {
        let n = self.window;
        let b = self.bins;
        let mut joint = vec![0usize; b * b];
        let mut px = vec![0usize; b];
        for t in 1..n {
            let y = self.bin(self.rx[(self.idx + t) % n]);
            let x = self.bin(self.rx[(self.idx + t - 1) % n]);
            joint[y * b + x] += 1;
            px[x] += 1;
        }
        let total = (n - 1) as f64;
        let mut h = 0.0;
        for x in 0..b {
            if px[x] == 0 {
                continue;
            }
            let p_x = (px[x] as f64) / total;
            let mut h_y_given_x = 0.0;
            for y in 0..b {
                let c = joint[y * b + x] as f64;
                if c > 0.0 {
                    let p_yx = c / (px[x] as f64);
                    h_y_given_x -= p_yx * p_yx.ln();
                }
            }
            h += p_x * h_y_given_x;
        }
        h.max(0.0)
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
    fn test_conditional_entropy_creation() {
        let ce = ConditionalEntropy::new(30, 8, 0.05);
        assert!(!ce.is_ready());
        assert_eq!(ce.value().main(), 0.0);
    }

    #[test]
    fn test_conditional_entropy_warmup() {
        let mut ce = ConditionalEntropy::new(20, 8, 0.05);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ce.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ce.is_ready());
    }

    #[test]
    fn test_conditional_entropy_values_finite() {
        let mut ce = ConditionalEntropy::new(20, 8, 0.05);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ce.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_conditional_entropy_reset() {
        let mut ce = ConditionalEntropy::new(20, 8, 0.05);
        for i in 0..30 {
            ce.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ce.reset();
        assert!(!ce.is_ready());
        assert_eq!(ce.value().main(), 0.0);
    }
}
