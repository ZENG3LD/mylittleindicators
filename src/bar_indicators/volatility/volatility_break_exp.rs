// Volatility Break detector with exponential sensitivity

pub struct VolatilityBreakExp {
    alpha: f64,
    ema: f64,
    init: bool,
    threshold: f64,
    pub value: f64,
}

impl VolatilityBreakExp {
    pub fn new(alpha: f64, threshold_sigma: f64) -> Self {
        Self {
            alpha: alpha.clamp(0.01, 1.0),
            ema: 0.0,
            init: false,
            threshold: threshold_sigma.max(0.5),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.ema = 0.0;
        self.init = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.init
    }

    pub fn update_volatility(&mut self, vol: f64) -> f64 {
        if !self.init {
            self.ema = vol;
            self.init = true;
            self.value = 0.0;
            return self.value;
        }
        let prev = self.ema;
        self.ema = self.alpha * vol + (1.0 - self.alpha) * self.ema;
        let sigma = (vol - prev).abs().max(1e-9);
        self.value = if (vol - self.ema).abs() > self.threshold * sigma {
            1.0
        } else {
            0.0
        };
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volatility_break_exp_creation() {
        let vbe = VolatilityBreakExp::new(0.1, 2.0);
        assert!(!vbe.is_ready());
        assert_eq!(vbe.value, 0.0);
    }

    #[test]
    fn test_volatility_break_exp_warmup() {
        let mut vbe = VolatilityBreakExp::new(0.1, 2.0);
        vbe.update_volatility(0.02);
        assert!(vbe.is_ready());
    }

    #[test]
    fn test_volatility_break_exp_values() {
        let mut vbe = VolatilityBreakExp::new(0.1, 2.0);
        for i in 0..20 {
            let vol = 0.02 + (i as f64 * 0.1).sin() * 0.01;
            let value = vbe.update_volatility(vol);
            assert!(value == 0.0 || value == 1.0, "Break signal should be 0 or 1");
        }
    }

    #[test]
    fn test_volatility_break_exp_reset() {
        let mut vbe = VolatilityBreakExp::new(0.1, 2.0);
        vbe.update_volatility(0.02);
        vbe.update_volatility(0.03);
        vbe.reset();
        assert!(!vbe.is_ready());
        assert_eq!(vbe.value, 0.0);
    }
}
