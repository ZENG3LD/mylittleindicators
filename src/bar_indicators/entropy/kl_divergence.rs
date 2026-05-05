// Rolling KL divergence between two adjacent halves of a window of log-returns

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct KLDivergence {
    window: usize,
    bins: usize,
    clip_abs: f64,
    rets: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    value: f64,
}

impl KLDivergence {
    pub fn new(window: usize, bins: usize, clip_abs: f64) -> Self {
        let w = window.max(10) | 1; // ensure odd? use |1 to make odd; but we will split into halves truncating
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
                // first half
                for i in 0..half {
                    let r = self.rets[(self.idx + i) % self.window];
                    p[self.bin_index(r)] += 1.0;
                }
                // second half
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
                let mut kl = 0.0;
                for b in 0..self.bins {
                    let pi = p[b];
                    let qi = q[b];
                    if pi > 0.0 && qi > 0.0 {
                        kl += pi * (pi / qi).ln();
                    }
                }
                self.value = kl.max(0.0);
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
    fn test_kl_divergence_creation() {
        let kld = KLDivergence::new(20, 10, 0.05);
        assert!(!kld.is_ready());
        assert_eq!(kld.value().main(), 0.0);
    }

    #[test]
    fn test_kl_divergence_warmup() {
        let mut kld = KLDivergence::new(15, 10, 0.05);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kld.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(kld.is_ready());
    }

    #[test]
    fn test_kl_divergence_values_finite() {
        let mut kld = KLDivergence::new(15, 10, 0.05);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kld.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_kl_divergence_values_non_negative() {
        let mut kld = KLDivergence::new(15, 10, 0.05);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kld.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_kl_divergence_reset() {
        let mut kld = KLDivergence::new(15, 10, 0.05);
        for i in 0..25 {
            kld.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        kld.reset();
        assert!(!kld.is_ready());
        assert_eq!(kld.value().main(), 0.0);
    }
}
