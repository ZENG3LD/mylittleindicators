// Liquidity gap density (simplified FVG heat): density of recent gaps by amplitude and duration

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct LiquidityGapDensity {
    window: usize,
    threshold: f64,
    value: f64,
    last_high: f64,
    last_low: f64,
    gaps: Vec<f64>,
    idx: usize,
    filled: bool,
}

impl LiquidityGapDensity {
    pub fn new(window: usize, threshold: f64) -> Self {
        let w = window.clamp(20, 512);
        Self {
            window: w,
            threshold,
            value: 0.0,
            last_high: 0.0,
            last_low: 0.0,
            gaps: vec![0.0; w],
            idx: 0,
            filled: false,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.idx = 0;
        self.filled = false;
        self.gaps.fill(0.0);
        self.last_high = 0.0;
        self.last_low = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, _c: f64, _v: f64) -> f64 {
        if self.last_high > 0.0 || self.last_low > 0.0 {
            // upward gap if current low > last high; downward if current high < last low
            let up_gap = (l - self.last_high).max(0.0);
            let dn_gap = (self.last_low - h).max(0.0);
            let amp = up_gap.max(dn_gap);
            let score = if amp > self.threshold {
                (amp / self.threshold).min(5.0)
            } else {
                0.0
            };
            self.gaps[self.idx] = score;
        }
        self.last_high = h;
        self.last_low = l;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        let len = if self.filled { self.window } else { self.idx };
        if len > 0 {
            let mut s = 0.0;
            for i in 0..len {
                s += self.gaps[i];
            }
            self.value = s / len as f64;
        }
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
    fn test_liquidity_gap_density_creation() {
        let lgd = LiquidityGapDensity::new(30, 0.5);
        assert!(!lgd.is_ready());
    }

    #[test]
    fn test_liquidity_gap_density_warmup() {
        let mut lgd = LiquidityGapDensity::new(30, 0.5);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            lgd.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(lgd.is_ready());
    }

    #[test]
    fn test_liquidity_gap_density_non_negative() {
        let mut lgd = LiquidityGapDensity::new(30, 0.5);
        for i in 0..50 {
            let price = 100.0 + i as f64;
            let value = lgd.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "Density should be non-negative");
        }
    }

    #[test]
    fn test_liquidity_gap_density_reset() {
        let mut lgd = LiquidityGapDensity::new(30, 0.5);
        for i in 0..40 {
            lgd.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0, 1000.0);
        }
        lgd.reset();
        assert!(!lgd.is_ready());
    }
}
