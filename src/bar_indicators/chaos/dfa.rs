// Detrended Fluctuation Analysis (DFA) proxy: slope of log(F(n)) vs log(n) for few scales

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct Dfa {
    windows: [usize; 4],
    buffers: Vec<Vec<f64>>, // per scale demeaned cum-sum
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    pub alpha: f64,
}

impl Dfa {
    pub fn new(scales: [usize; 4]) -> Self {
        let mut bufs = Vec::with_capacity(4);
        for &w in &scales {
            bufs.push(vec![0.0; w]);
        }
        Self {
            windows: scales,
            buffers: bufs,
            idx: 0,
            filled: false,
            last_close: None,
            alpha: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        for b in &mut self.buffers {
            b.fill(0.0);
        }
        self.idx = 0;
        self.filled = false;
        self.last_close = None;
        self.alpha = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.alpha)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        // maintain cumulative sum per scale and compute RMS detrended via simple linear fit proxy
        for (si, w) in self.windows.iter().enumerate() {
            let buf = &mut self.buffers[si];
            let n = *w;
            buf[self.idx % n] = c;
            if self.idx >= n {
                // demean
                let mean: f64 = buf[..n].iter().sum::<f64>() / n as f64;
                let mut sxy = 0.0;
                let mut sx = 0.0;
                let mut sy = 0.0;
                let mut sxx = 0.0;
                for (i, &bv) in buf[..n].iter().enumerate() {
                    let x = i as f64;
                    let y = bv - mean;
                    sx += x;
                    sy += y;
                    sxy += x * y;
                    sxx += x * x;
                }
                let denom = n as f64 * sxx - sx * sx;
                let (a, beta) = if denom.abs() > 1e-12 {
                    let beta = (n as f64 * sxy - sx * sy) / denom;
                    let a = (sy - beta * sx) / n as f64;
                    (a, beta)
                } else {
                    (0.0, 0.0)
                };
                // RMS residual
                let mut rss = 0.0;
                for (i, &bv) in buf[..n].iter().enumerate() {
                    let x = i as f64;
                    let fit = a + beta * x;
                    let r = (bv - mean) - fit;
                    rss += r * r;
                }
                let f = (rss / n as f64).sqrt();
                // store back f in first cell as cheap cache
                buf[0] = f;
            }
        }
        self.idx += 1;
        if !self.filled {
            self.filled = self.windows.iter().all(|&w| self.idx >= w);
        }
        if self.filled {
            // compute slope between first two non-zero Fs on log-log grid
            let mut xs = Vec::new();
            let mut ys = Vec::new();
            for (si, &w) in self.windows.iter().enumerate() {
                let f = self.buffers[si][0];
                if f > 0.0 {
                    xs.push((w as f64).ln());
                    ys.push(f.ln());
                }
            }
            if xs.len() >= 2 {
                let n = xs.len() as f64;
                let mut sx = 0.0;
                let mut sy = 0.0;
                let mut sxy = 0.0;
                let mut sxx = 0.0;
                for i in 0..xs.len() {
                    sx += xs[i];
                    sy += ys[i];
                    sxy += xs[i] * ys[i];
                    sxx += xs[i] * xs[i];
                }
                let denom = n * sxx - sx * sx;
                self.alpha = if denom.abs() > 1e-9 {
                    (n * sxy - sx * sy) / denom
                } else {
                    0.5
                };
            }
        }
        self.alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dfa_creation() {
        let ind = Dfa::new([8, 16, 32, 64]);
        assert!(!ind.is_ready());
        assert_eq!(ind.alpha, 0.0);
    }

    #[test]
    fn test_dfa_warmup() {
        let mut ind = Dfa::new([8, 16, 32, 64]);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_dfa_values_finite() {
        let mut ind = Dfa::new([8, 16, 32, 64]);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let alpha = ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(alpha.is_finite());
        }
    }

    #[test]
    fn test_dfa_reset() {
        let mut ind = Dfa::new([8, 16, 32, 64]);
        for i in 0..70 {
            ind.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.alpha, 0.0);
    }
}
