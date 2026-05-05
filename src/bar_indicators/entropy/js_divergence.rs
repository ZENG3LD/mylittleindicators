// Rolling Jensen-Shannon divergence between two adjacent halves of a window

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct JSDivergence {
    window: usize,
    bins: usize,
    clip_abs: f64,
    rets: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    value: f64,
}

impl JSDivergence {
    pub fn new(window: usize, bins: usize, clip_abs: f64) -> Self {
        let w = window.max(10);
        Self {
            window: w,
            bins: bins.max(8),
            clip_abs: clip_abs.max(1e-6),
            rets: vec![0.0; w],
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
        self.rets.fill(0.0);
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    #[inline]
    fn bin_index(&self, r: f64) -> usize {
        let rr = r.max(-self.clip_abs).min(self.clip_abs);
        let x = (rr + self.clip_abs) / (2.0 * self.clip_abs);
        (x * self.bins as f64)
            .floor()
            .clamp(0.0, (self.bins - 1) as f64) as usize
    }

    #[inline]
    fn kl(p: &[f64], q: &[f64]) -> f64 {
        let mut s = 0.0;
        for i in 0..p.len() {
            if p[i] > 0.0 && q[i] > 0.0 {
                s += p[i] * (p[i] / q[i]).ln();
            }
        }
        s
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
            self.rets[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if self.idx == 0 {
                self.filled = true;
            }
            if self.filled {
                let half = self.window / 2;
                let mut p = vec![1e-12; self.bins];
                let mut q = vec![1e-12; self.bins];
                for i in 0..half {
                    let r = self.rets[(self.idx + i) % self.window];
                    p[self.bin_index(r)] += 1.0;
                }
                for i in half..self.window {
                    let r = self.rets[(self.idx + i) % self.window];
                    q[self.bin_index(r)] += 1.0;
                }
                let ps: f64 = p.iter().sum();
                let qs: f64 = q.iter().sum();
                for v in &mut p {
                    *v /= ps;
                }
                for v in &mut q {
                    *v /= qs;
                }
                let mut m = vec![0.0; self.bins];
                for i in 0..self.bins {
                    m[i] = 0.5 * (p[i] + q[i]);
                }
                let js = 0.5 * Self::kl(&p, &m) + 0.5 * Self::kl(&q, &m);
                self.value = js.max(0.0);
            }
        }
        self.last_close = Some(close);
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
    fn test_js_divergence_creation() {
        let jsd = JSDivergence::new(20, 10, 0.05);
        assert!(!jsd.is_ready());
        assert_eq!(jsd.value().main(), 0.0);
    }

    #[test]
    fn test_js_divergence_warmup() {
        let mut jsd = JSDivergence::new(15, 10, 0.05);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            jsd.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(jsd.is_ready());
    }

    #[test]
    fn test_js_divergence_values_finite() {
        let mut jsd = JSDivergence::new(15, 10, 0.05);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = jsd.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_js_divergence_values_non_negative() {
        let mut jsd = JSDivergence::new(15, 10, 0.05);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = jsd.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_js_divergence_reset() {
        let mut jsd = JSDivergence::new(15, 10, 0.05);
        for i in 0..25 {
            jsd.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        jsd.reset();
        assert!(!jsd.is_ready());
        assert_eq!(jsd.value().main(), 0.0);
    }
}
