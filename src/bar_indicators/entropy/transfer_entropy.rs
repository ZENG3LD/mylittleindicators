// Simplified Transfer Entropy proxy: TE(X->Y) with lag using binned returns and joint frequencies
//
// Self-contained: computes returns from close and volume internally
// X = volume returns, Y = price returns
// Measures information flow from volume to price

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct TransferEntropy {
    window: usize,
    lag: usize,
    bins: usize,
    clip_abs: f64,
    rx: Vec<f64>,
    ry: Vec<f64>,
    idx: usize,
    filled: bool,
    // 3D joint: p(y_t, y_{t-1}, x_{t-1}) flattened as [bins^3]
    joint: Vec<usize>,
    count: usize,
    value: f64,
    // For computing returns internally
    prev_close: f64,
    prev_volume: f64,
    has_prev: bool,
}

impl TransferEntropy {
    pub fn new(window: usize, lag: usize, bins: usize, clip_abs: f64) -> Self {
        let w = window.max(3);
        let l = lag.max(1).min(w - 2);
        let b = bins.max(2);
        Self {
            window: w,
            lag: l,
            bins: b,
            clip_abs: clip_abs.max(1e-6),
            rx: vec![0.0; w + 2],
            ry: vec![0.0; w + 2],
            idx: 0,
            filled: false,
            joint: vec![0usize; b * b * b],
            count: 0,
            value: 0.0,
            prev_close: 0.0,
            prev_volume: 0.0,
            has_prev: false,
        }
    }

    #[inline]
    fn bin(&self, r: f64) -> usize {
        let rr = r.max(-self.clip_abs).min(self.clip_abs);
        let x = (rr + self.clip_abs) / (2.0 * self.clip_abs);
        (x * self.bins as f64)
            .floor()
            .clamp(0.0, (self.bins - 1) as f64) as usize
    }

    #[inline]
    fn idx3(&self, y: usize, y1: usize, x1: usize) -> usize {
        (y * self.bins + y1) * self.bins + x1
    }

    #[inline]
    pub fn reset(&mut self) {
        self.rx.fill(0.0);
        self.ry.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.joint.fill(0);
        self.count = 0;
        self.value = 0.0;
        self.prev_close = 0.0;
        self.prev_volume = 0.0;
        self.has_prev = false;
    }

    /// Update with OHLCV bar - computes returns internally
    /// X = volume return, Y = price return
    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        volume: f64,
    ) -> f64 {
        if !self.has_prev {
            self.prev_close = close;
            self.prev_volume = volume.max(1.0);
            self.has_prev = true;
            return self.value;
        }

        // Compute log returns
        let price_return = if self.prev_close > 0.0 {
            (close / self.prev_close).ln()
        } else {
            0.0
        };
        let volume_return = if self.prev_volume > 0.0 && volume > 0.0 {
            (volume / self.prev_volume).ln()
        } else {
            0.0
        };

        self.prev_close = close;
        self.prev_volume = volume.max(1.0);

        // Use volume as X (source), price as Y (target)
        // Transfer entropy measures: does past volume help predict future price?
        self.update_returns(volume_return, price_return)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.count >= (self.window - self.lag - 1)
    }

    pub fn update_returns(&mut self, r_x: f64, r_y: f64) -> f64 {
        self.rx[self.idx] = r_x;
        self.ry[self.idx] = r_y;
        let curr = self.idx;
        self.idx = (self.idx + 1) % self.rx.len();
        if self.idx == 0 {
            self.filled = true;
        }
        if !self.filled {
            return self.value;
        }

        let by = self.bin(self.ry[curr]);
        let by1 = self.bin(self.ry[(curr + self.ry.len() - 1) % self.ry.len()]);
        let bx1 = self.bin(self.rx[(curr + self.rx.len() - 1 - self.lag) % self.rx.len()]);
        let ins_idx = self.idx3(by, by1, bx1);
        self.joint[ins_idx] += 1;
        self.count += 1;
        if self.count > (self.window - self.lag - 1) {
            let old = (curr + self.rx.len() - (self.window - self.lag - 1)) % self.rx.len();
            let oy = self.bin(self.ry[old]);
            let oy1 = self.bin(self.ry[(old + self.ry.len() - 1) % self.ry.len()]);
            let ox1 = self.bin(self.rx[(old + self.rx.len() - 1 - self.lag) % self.rx.len()]);
            let oi = self.idx3(oy, oy1, ox1);
            if self.joint[oi] > 0 {
                self.joint[oi] -= 1;
            }
            self.count -= 1;
        }
        // TE proxy: sum p(y,y1,x1) log [ p(y|y1,x1) / p(y|y1) ]
        let total = (self.count as f64).max(1.0);
        let mut py_y1x1 = vec![0.0; self.bins * self.bins];
        let mut py_y1 = vec![0.0; self.bins * self.bins];
        for y in 0..self.bins {
            for y1 in 0..self.bins {
                let mut s = 0.0;
                for x1 in 0..self.bins {
                    s += self.joint[self.idx3(y, y1, x1)] as f64;
                }
                py_y1[y * self.bins + y1] += s;
                for x1 in 0..self.bins {
                    py_y1x1[y1 * self.bins + x1] += self.joint[self.idx3(y, y1, x1)] as f64;
                }
            }
        }
        let mut te = 0.0;
        for y in 0..self.bins {
            for y1 in 0..self.bins {
                for x1 in 0..self.bins {
                    let pyyx = (self.joint[self.idx3(y, y1, x1)] as f64) / total;
                    if pyyx <= 0.0 {
                        continue;
                    }
                    let py_given_y1x1 = pyyx / (py_y1x1[y1 * self.bins + x1] / total).max(1e-12);
                    let py_given_y1 = pyyx / (py_y1[y * self.bins + y1] / total).max(1e-12);
                    if py_given_y1x1 > 0.0 && py_given_y1 > 0.0 {
                        te += pyyx * (py_given_y1x1 / py_given_y1).ln();
                    }
                }
            }
        }
        self.value = te.max(0.0);
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
    fn test_transfer_entropy_creation() {
        let te = TransferEntropy::new(30, 1, 4, 0.05);
        assert!(!te.is_ready());
        assert_eq!(te.value().main(), 0.0);
    }

    #[test]
    fn test_transfer_entropy_warmup() {
        let mut te = TransferEntropy::new(20, 1, 4, 0.05);
        for i in 0..40 {
            let r_x = (i as f64 * 0.1).sin() * 0.01;
            let r_y = (i as f64 * 0.15).sin() * 0.01;
            te.update_returns(r_x, r_y);
        }
        assert!(te.is_ready());
    }

    #[test]
    fn test_transfer_entropy_values_finite() {
        let mut te = TransferEntropy::new(20, 1, 4, 0.05);
        for i in 0..50 {
            let r_x = (i as f64 * 0.1).sin() * 0.01;
            let r_y = (i as f64 * 0.15).sin() * 0.01;
            let value = te.update_returns(r_x, r_y);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_transfer_entropy_values_non_negative() {
        let mut te = TransferEntropy::new(20, 1, 4, 0.05);
        for i in 0..50 {
            let r_x = (i as f64 * 0.1).sin() * 0.01;
            let r_y = (i as f64 * 0.15).sin() * 0.01;
            let value = te.update_returns(r_x, r_y);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_transfer_entropy_reset() {
        let mut te = TransferEntropy::new(20, 1, 4, 0.05);
        for i in 0..40 {
            let r_x = (i as f64 * 0.1).sin() * 0.01;
            let r_y = (i as f64 * 0.15).sin() * 0.01;
            te.update_returns(r_x, r_y);
        }
        te.reset();
        assert!(!te.is_ready());
        assert_eq!(te.value().main(), 0.0);
    }
}
