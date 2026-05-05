// ARCH LM proxy: regress r_t^2 on its lags and return R^2 as clustering proxy

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct ArchLmProxy {
    window: usize,
    lags: usize,
    last_close: Option<f64>,
    returns: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl ArchLmProxy {
    pub fn new(window: usize, lags: usize) -> Self {
        let w = window.clamp(50, 1024);
        let l = lags.clamp(1, 10);
        Self {
            window: w,
            lags: l,
            last_close: None,
            returns: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.last_close = None;
        self.idx = 0;
        self.filled = false;
        self.returns.fill(0.0);
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        if let Some(prev) = self.last_close {
            let r = (c / prev).ln();
            self.returns[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if self.idx == 0 {
                self.filled = true;
            }
        }
        self.last_close = Some(c.max(1e-12));
        if self.filled {
            self.value = self.compute_r2();
        }
        self.value
    }

    fn compute_r2(&self) -> f64 {
        // build y = r_t^2 and X = [1, r_{t-1}^2, ..., r_{t-L}^2]
        let n = self.window;
        let l = self.lags;
        let mut samples: Vec<(f64, Vec<f64>)> = Vec::with_capacity(n - l - 1);
        for k in 0..(n - l - 1) {
            let t = (self.idx + k + l) % n;
            let y = self.returns[t] * self.returns[t];
            let mut x = Vec::with_capacity(l + 1);
            x.push(1.0);
            for j in 1..=l {
                let tj = (t + n - j) % n;
                let r2 = self.returns[tj] * self.returns[tj];
                x.push(r2);
            }
            samples.push((y, x));
        }
        if samples.len() < l + 2 {
            return 0.0;
        }
        // OLS normal equations: beta = (X'X)^-1 X'y
        let p = l + 1; // intercept + lags
        let mut xtx = vec![vec![0.0; p]; p];
        let mut xty = vec![0.0; p];
        let mut ymean = 0.0;
        let mut tss = 0.0;
        for (y, x) in &samples {
            for i in 0..p {
                xty[i] += x[i] * y;
                for j in 0..p {
                    xtx[i][j] += x[i] * x[j];
                }
            }
            ymean += *y;
        }
        let m = samples.len() as f64;
        ymean /= m;
        for (y, _) in &samples {
            let d = *y - ymean;
            tss += d * d;
        }
        // invert small matrix (p <= 11)
        let beta = match invert_sym_posdef(&xtx) {
            Some(inv) => {
                let mut b = vec![0.0; p];
                for i in 0..p {
                    for j in 0..p {
                        b[i] += inv[i][j] * xty[j];
                    }
                }
                b
            }
            None => return 0.0,
        };
        // compute fitted and RSS
        let mut rss = 0.0;
        for (y, x) in &samples {
            let mut yhat = 0.0;
            for i in 0..p {
                yhat += beta[i] * x[i];
            }
            let e = *y - yhat;
            rss += e * e;
        }
        if tss <= 1e-12 {
            return 0.0;
        }
        (1.0 - (rss / tss)).clamp(0.0, 1.0)
    }
}

fn invert_sym_posdef(a: &Vec<Vec<f64>>) -> Option<Vec<Vec<f64>>> {
    // naive Gauss-Jordan for small p
    let n = a.len();
    let mut aug = vec![vec![0.0; 2 * n]; n];
    for i in 0..n {
        for j in 0..n {
            aug[i][j] = a[i][j];
        }
        aug[i][n + i] = 1.0;
    }
    for i in 0..n {
        // pivot
        let mut piv = i;
        let mut best = aug[piv][i].abs();
        for (r, row) in aug[(i + 1)..].iter().enumerate().map(|(r, row)| (i + 1 + r, row)) {
            if row[i].abs() > best {
                best = row[i].abs();
                piv = r;
            }
        }
        if best < 1e-18 {
            return None;
        }
        if piv != i {
            aug.swap(i, piv);
        }
        let diag = aug[i][i];
        for cell in aug[i].iter_mut() {
            *cell /= diag;
        }
        for r in 0..n {
            if r != i {
                let f = aug[r][i];
                if f != 0.0 {
                    // split to allow simultaneous mut borrow of aug[r] and immut of aug[i]
                    if r < i {
                        let (left, right) = aug.split_at_mut(i);
                        for (ar_c, &ai_c) in left[r].iter_mut().zip(right[0].iter()) {
                            *ar_c -= f * ai_c;
                        }
                    } else {
                        let (left, right) = aug.split_at_mut(r);
                        for (ar_c, &ai_c) in right[0].iter_mut().zip(left[i].iter()) {
                            *ar_c -= f * ai_c;
                        }
                    }
                }
            }
        }
    }
    let mut inv = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            inv[i][j] = aug[i][n + j];
        }
    }
    Some(inv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arch_lm_proxy_creation() {
        let arch = ArchLmProxy::new(50, 5);
        assert!(!arch.is_ready());
    }

    #[test]
    fn test_arch_lm_proxy_warmup() {
        let mut arch = ArchLmProxy::new(50, 5);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            arch.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(arch.is_ready());
    }

    #[test]
    fn test_arch_lm_proxy_range() {
        let mut arch = ArchLmProxy::new(50, 5);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = arch.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 1.0, "R^2 should be in [0, 1]");
        }
    }

    #[test]
    fn test_arch_lm_proxy_reset() {
        let mut arch = ArchLmProxy::new(50, 5);
        for i in 0..60 {
            arch.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        arch.reset();
        assert!(!arch.is_ready());
    }
}
