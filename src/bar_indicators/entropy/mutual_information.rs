// Rolling Mutual Information between r_t and r_{t-lag} using fixed bins and O(1) histogram updates

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct MutualInformation {
    window: usize,
    lag: usize,
    bins: usize,
    clip_abs: f64,
    // returns ring (for binning and lag access)
    rets: Vec<f64>,
    r_idx: usize,
    r_filled: bool,
    // joint histogram counts [bins x bins], marginals derived on the fly
    joint: Vec<usize>,
    count_pairs: usize,
    // to support O(1) removal, we track which pair leaves when advancing the window
    // we remove pair at index s_old = curr_index + bins_len - window + lag (mod bins_len)
    mi: f64,
}

impl MutualInformation {
    pub fn new(window: usize, lag: usize, bins: usize, clip_abs: f64) -> Self {
        let w = window.max(2);
        let l = lag.max(1).min(w - 1);
        let b = bins.max(2);
        Self {
            window: w,
            lag: l,
            bins: b,
            clip_abs: clip_abs.max(1e-6),
            rets: vec![0.0; w + 1 + l],
            r_idx: 0,
            r_filled: false,
            joint: vec![0usize; b * b],
            count_pairs: 0,
            mi: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.rets.fill(0.0);
        self.r_idx = 0;
        self.r_filled = false;
        self.joint.fill(0);
        self.count_pairs = 0;
        self.mi = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.r_filled && self.count_pairs >= (self.window - self.lag)
    }

    #[inline]
    fn bin_index(&self, r: f64) -> usize {
        // map r in [-clip_abs, clip_abs] to 0..bins-1
        let rr = r.max(-self.clip_abs).min(self.clip_abs);
        let x = (rr + self.clip_abs) / (2.0 * self.clip_abs);
        let idx = (x * self.bins as f64).floor() as isize;
        idx.clamp(0, (self.bins as isize) - 1) as usize
    }

    #[inline]
    fn joint_idx(&self, bx: usize, by: usize) -> usize {
        bx * self.bins + by
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> f64 {
        // compute return from last close in ring
        let prev_pos = (self.r_idx + self.rets.len() - 1) % self.rets.len();
        let prev_c = self.rets[prev_pos];

        // Only compute valid return if we have a valid previous close
        let r = if prev_c > 0.0 && close > 0.0 {
            (close / prev_c).ln()
        } else {
            0.0
        };

        // push close into ring
        self.rets[self.r_idx] = close;
        let curr_idx = self.r_idx;
        self.r_idx = (self.r_idx + 1) % self.rets.len();
        if self.r_idx == 0 {
            self.r_filled = true;
        }

        if !self.r_filled {
            return self.mi;
        }

        // new pair at s_new uses return r_t and r_{t-lag}
        let bin_x = self.bin_index(r);
        // compute r_{t-lag}: use closes at indices curr_idx (just written) and idx_lag_prev = curr_idx - lag - 1
        let idx_curr = curr_idx;
        let idx_lag = (idx_curr + self.rets.len() - self.lag) % self.rets.len();
        let c_lag_prev = self.rets[(idx_lag + self.rets.len() - 1) % self.rets.len()];
        let c_lag_curr = self.rets[idx_lag];
        let r_lag = if c_lag_prev > 0.0 && c_lag_curr > 0.0 {
            (c_lag_curr / c_lag_prev).ln()
        } else {
            0.0
        };
        let bin_y = self.bin_index(r_lag);

        // increment joint
        let ji_new = self.joint_idx(bin_x, bin_y);
        self.joint[ji_new] += 1;
        self.count_pairs += 1;

        // remove oldest pair when beyond window
        if self.count_pairs > (self.window - self.lag) {
            // pair leaving corresponds to s_old = t - (window - self.lag)
            let idx_old_curr =
                (idx_curr + self.rets.len() - (self.window - self.lag)) % self.rets.len();
            let idx_old_lag = (idx_old_curr + self.rets.len() - self.lag) % self.rets.len();
            let oc_prev = self.rets[(idx_old_curr + self.rets.len() - 1) % self.rets.len()];
            let oc_curr = self.rets[idx_old_curr];
            let or_t = if oc_prev > 0.0 && oc_curr > 0.0 {
                (oc_curr / oc_prev).ln()
            } else {
                0.0
            };
            let ol_prev = self.rets[(idx_old_lag + self.rets.len() - 1) % self.rets.len()];
            let ol_curr = self.rets[idx_old_lag];
            let or_lag = if ol_prev > 0.0 && ol_curr > 0.0 {
                (ol_curr / ol_prev).ln()
            } else {
                0.0
            };
            let bx_old = self.bin_index(or_t);
            let by_old = self.bin_index(or_lag);
            let ji_old = self.joint_idx(bx_old, by_old);
            if self.joint[ji_old] > 0 {
                self.joint[ji_old] -= 1;
            }
            self.count_pairs -= 1;
        }

        // compute MI from joint
        let total = (self.count_pairs as f64).max(1.0);
        // compute marginals p(x), p(y)
        let mut px = vec![0.0; self.bins];
        let mut py = vec![0.0; self.bins];
        for (bx, px_entry) in px.iter_mut().enumerate() {
            for (by, py_entry) in py.iter_mut().enumerate() {
                let v = self.joint[self.joint_idx(bx, by)] as f64;
                *px_entry += v;
                *py_entry += v;
            }
        }
        for v in &mut px {
            *v /= total;
        }
        for v in &mut py {
            *v /= total;
        }
        let mut mi = 0.0;
        for (bx, &px_val) in px.iter().enumerate() {
            for (by, &py_val) in py.iter().enumerate() {
                let pxy = (self.joint[self.joint_idx(bx, by)] as f64) / total;
                if pxy > 0.0 && px_val > 0.0 && py_val > 0.0 {
                    mi += pxy * (pxy / (px_val * py_val)).ln();
                }
            }
        }
        self.mi = mi.max(0.0);
        self.mi
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.mi)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutual_information_creation() {
        let mi = MutualInformation::new(30, 1, 8, 0.05);
        assert!(!mi.is_ready());
        assert_eq!(mi.value().main(), 0.0);
    }

    #[test]
    fn test_mutual_information_warmup() {
        let mut mi = MutualInformation::new(20, 1, 8, 0.05);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            mi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(mi.is_ready());
    }

    #[test]
    fn test_mutual_information_values_finite() {
        let mut mi = MutualInformation::new(20, 1, 8, 0.05);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = mi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_mutual_information_values_non_negative() {
        let mut mi = MutualInformation::new(20, 1, 8, 0.05);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = mi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_mutual_information_reset() {
        let mut mi = MutualInformation::new(20, 1, 8, 0.05);
        for i in 0..40 {
            mi.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        mi.reset();
        assert!(!mi.is_ready());
        assert_eq!(mi.value().main(), 0.0);
    }

    #[test]
    fn test_mutual_information_nonzero_values() {
        // Test that MI produces non-zero values with autocorrelated data
        let mut mi = MutualInformation::new(30, 1, 8, 0.1);
        let mut price = 100.0;
        let mut any_nonzero = false;

        for i in 0..100 {
            // Create autocorrelated returns (trending pattern)
            let trend = if i < 30 { 1.0 } else if i < 60 { -1.0 } else { 0.5 };
            price += trend + (i as f64 * 0.1).sin() * 0.5;
            let val = mi.update_bar(price - 0.5, price + 1.0, price - 1.0, price, 1000.0);
            eprintln!("Bar {}: MI={:.6}, ready={}", i, val, mi.is_ready());
            if val > 0.0 {
                any_nonzero = true;
            }
        }

        assert!(mi.is_ready(), "MI should be ready after 100 bars");
        assert!(any_nonzero, "MI should produce non-zero values with autocorrelated data");
    }
}
