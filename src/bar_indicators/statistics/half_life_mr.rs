// Half-Life of Mean Reversion (Ornstein–Uhlenbeck proxy) from AR(1) phi: hl = -ln(2)/ln(phi)

#[derive(Clone)]
pub struct HalfLifeMr {
    window: usize,
    vals: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    pub half_life: f64,
}

impl HalfLifeMr {
    pub fn new(window: usize) -> Self {
        let w = window.max(20);
        Self {
            window: w,
            vals: vec![0.0; w],
            idx: 0,
            filled: false,
            last_close: None,
            half_life: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.vals.fill(0.0);
        self.last_close = None;
        self.half_life = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    #[inline]
    pub fn value(&self) -> crate::IndicatorValue {
        crate::IndicatorValue::Single(self.half_life)
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
            self.half_life = self.compute_hl();
        }
        self.half_life
    }

    fn compute_hl(&self) -> f64 {
        let n = self.window;
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut sxx = 0.0;
        let mut sxy = 0.0;
        let mut count = 0.0;
        for i in 1..n {
            let y = self.vals[(self.idx + i) % n];
            let x = self.vals[(self.idx + i - 1) % n];
            sx += x;
            sy += y;
            sxx += x * x;
            sxy += x * y;
            count += 1.0;
        }
        let denom = count * sxx - sx * sx;
        let phi = if denom.abs() > 1e-12 {
            (count * sxy - sx * sy) / denom
        } else {
            0.0
        };
        if phi > 0.0 && phi < 1.0 {
            (-(2.0_f64).ln() / phi.ln()).max(0.0)
        } else {
            f64::INFINITY
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_half_life_mr_creation() {
        let hlmr = HalfLifeMr::new(50);
        assert!(!hlmr.is_ready());
        assert_eq!(hlmr.half_life, 0.0);
    }

    #[test]
    fn test_half_life_mr_warmup() {
        let mut hlmr = HalfLifeMr::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            hlmr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(hlmr.is_ready());
    }

    #[test]
    fn test_half_life_mr_values() {
        let mut hlmr = HalfLifeMr::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = hlmr.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 || value.is_infinite(), "Half-life should be non-negative or inf");
        }
    }

    #[test]
    fn test_half_life_mr_reset() {
        let mut hlmr = HalfLifeMr::new(50);
        for i in 0..60 {
            hlmr.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        hlmr.reset();
        assert!(!hlmr.is_ready());
        assert_eq!(hlmr.half_life, 0.0);
    }
}
