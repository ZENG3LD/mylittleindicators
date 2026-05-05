// Information Gain: IG(Y;X)=H(Y)-H(Y|X) with discrete bins on r_t (Y) and r_{t-1} (X)

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct InformationGain {
    window: usize,
    bins: usize,
    clip_abs: f64,
    vals: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    pub value: f64,
}

impl InformationGain {
    pub fn new(window: usize, bins: usize, clip_abs: f64) -> Self {
        let w = window.max(20);
        let b = bins.max(4);
        Self {
            window: w,
            bins: b,
            clip_abs: clip_abs.max(1e-6),
            vals: vec![0.0; w],
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
        self.vals.fill(0.0);
        self.last_close = None;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        if let Some(prev) = self.last_close {
            let r = (c / prev).ln();
            self.vals[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if !self.filled && self.idx == 0 {
                self.filled = true;
            }
        }
        self.last_close = Some(c);
        if self.filled {
            self.value = self.compute_ig();
        }
        self.value
    }

    fn compute_ig(&self) -> f64 {
        let n = self.window;
        if n < 3 {
            return 0.0;
        }
        let mut hx = vec![0usize; self.bins];
        let mut hy = vec![0usize; self.bins];
        let mut hxy = vec![0usize; self.bins * self.bins];
        let clip = self.clip_abs;
        let min = -clip;
        let mut max = clip;
        if min == max {
            max = min + 1e-6;
        }
        let xi = |v: f64| -> usize {
            let t = ((v.min(max).max(min) - min) / (max - min) * (self.bins as f64)) as usize;
            t.min(self.bins - 1)
        };
        for i in 1..n {
            let y = self.vals[(self.idx + i) % n];
            let x = self.vals[(self.idx + i - 1) % n];
            let bx = xi(x);
            let by = xi(y);
            hx[bx] += 1;
            hy[by] += 1;
            hxy[by * self.bins + bx] += 1;
        }
        let total = (n - 1) as f64;
        let mut h_y = 0.0;
        for c in hy {
            if c > 0 {
                let p = c as f64 / total;
                h_y -= p * p.ln();
            }
        }
        let mut h_yx = 0.0;
        for by in 0..self.bins {
            let mut row_sum = 0usize;
            for bx in 0..self.bins {
                row_sum += hxy[by * self.bins + bx];
            }
            if row_sum > 0 {
                let denom = row_sum as f64;
                for bx in 0..self.bins {
                    let c = hxy[by * self.bins + bx];
                    if c > 0 {
                        let p = c as f64 / denom;
                        h_yx -= (denom / total) * p * p.ln();
                    }
                }
            }
        }
        (h_y - h_yx).max(0.0)
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
    fn test_information_gain_creation() {
        let ig = InformationGain::new(30, 8, 0.05);
        assert!(!ig.is_ready());
        assert_eq!(ig.value, 0.0);
    }

    #[test]
    fn test_information_gain_warmup() {
        let mut ig = InformationGain::new(20, 8, 0.05);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ig.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ig.is_ready());
    }

    #[test]
    fn test_information_gain_values_finite() {
        let mut ig = InformationGain::new(20, 8, 0.05);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ig.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_information_gain_reset() {
        let mut ig = InformationGain::new(20, 8, 0.05);
        for i in 0..30 {
            ig.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ig.reset();
        assert!(!ig.is_ready());
        assert_eq!(ig.value, 0.0);
    }
}
