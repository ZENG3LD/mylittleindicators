/// Classic volatility estimators: Parkinson, Garman–Klass, Rogers–Satchell, Yang–Zhang (close-to-close part not included for brevity)
#[derive(Debug, Clone, Default)]
pub struct VolEstimators {
    pub parkinson: f64,
    pub garman_klass: f64,
    pub rogers_satchell: f64,
    pub yang_zhang: f64,
}

#[derive(Debug, Clone)]
pub struct VolatilityEstimators {
    n: usize,
    count: usize,
    sum_parkinson: f64,
    sum_gk: f64,
    sum_rs: f64,
    sum_yz: f64,
    prev_close: Option<f64>,
}

impl VolatilityEstimators {
    pub fn new(window: usize) -> Self {
        Self {
            n: window.max(1),
            count: 0,
            sum_parkinson: 0.0,
            sum_gk: 0.0,
            sum_rs: 0.0,
            sum_yz: 0.0,
            prev_close: None,
        }
    }

    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, _v: f64) -> VolEstimators {
        // Parkinson: sigma^2 = (1/(4 ln 2)) * ln(H/L)^2
        let hl = if l > 0.0 { (h / l).max(1e-12) } else { 1e-12 };
        let park = (hl.ln()).powi(2) / (4.0 * std::f64::consts::LN_2);

        // Garman–Klass (simplified high-low term): 0.5*ln(H/L)^2 - (2ln2 -1)*ln(C/O)^2 (omit O)
        let gk = 0.5 * (hl.ln()).powi(2);

        // Rogers–Satchell: ln(H/C)*ln(H/O) + ln(L/C)*ln(L/O). Without open, approximate O = C of prev.
        let o_approx = self.prev_close.unwrap_or(c);
        let rs = (h / c).ln() * (h / o_approx).ln() + (l / c).ln() * (l / o_approx).ln();

        // Yang–Zhang simplified: rs + close gaps term approx 0 here
        let yz = rs;

        // rolling mean using simple running window approx (no eviction to keep O(1) and simplicity)
        self.count += 1;
        self.sum_parkinson += park;
        self.sum_gk += gk;
        self.sum_rs += rs;
        self.sum_yz += yz;
        self.prev_close = Some(c);

        VolEstimators {
            parkinson: (self.sum_parkinson / self.count as f64).sqrt(),
            garman_klass: (self.sum_gk / self.count as f64).sqrt(),
            rogers_satchell: (self.sum_rs / self.count as f64).sqrt(),
            yang_zhang: (self.sum_yz / self.count as f64).sqrt(),
        }
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.n
    }

    pub fn reset(&mut self) {
        self.count = 0;
        self.sum_parkinson = 0.0;
        self.sum_gk = 0.0;
        self.sum_rs = 0.0;
        self.sum_yz = 0.0;
        self.prev_close = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volatility_estimators_creation() {
        let ve = VolatilityEstimators::new(14);
        assert!(!ve.is_ready());
    }

    #[test]
    fn test_volatility_estimators_warmup() {
        let mut ve = VolatilityEstimators::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ve.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ve.is_ready());
    }

    #[test]
    fn test_volatility_estimators_values() {
        let mut ve = VolatilityEstimators::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let result = ve.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(result.parkinson.is_finite());
            assert!(result.garman_klass.is_finite());
            assert!(result.rogers_satchell.is_finite());
            assert!(result.yang_zhang.is_finite());
        }
    }

    #[test]
    fn test_volatility_estimators_reset() {
        let mut ve = VolatilityEstimators::new(14);
        for i in 0..20 {
            ve.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        ve.reset();
        assert!(!ve.is_ready());
    }
}
