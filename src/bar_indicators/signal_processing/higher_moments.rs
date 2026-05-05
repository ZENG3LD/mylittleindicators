// Rolling skewness and kurtosis of log returns

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct HigherMoments {
    window: usize,
    closes: Vec<f64>,
    idx: usize,
    filled: bool,
    pub skew: f64,
    pub kurt: f64,
}

impl HigherMoments {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(3),
            closes: vec![0.0; window.max(3) + 1],
            idx: 0,
            filled: false,
            skew: 0.0,
            kurt: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.closes.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.skew = 0.0;
        self.kurt = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> (f64, f64) {
        self.closes[self.idx] = close;
        self.idx = (self.idx + 1) % self.closes.len();
        if self.idx == 0 {
            self.filled = true;
        }
        if !self.filled {
            return (self.skew, self.kurt);
        }

        // compute returns for last window
        let len = self.closes.len();
        let mut rets: Vec<f64> = Vec::with_capacity(self.window);
        for k in 0..self.window {
            let i_curr = (self.idx + len + len - 1 - k) % len;
            let i_prev = (i_curr + len - 1) % len;
            let c1 = self.closes[i_prev].max(1e-12);
            let c2 = self.closes[i_curr].max(1e-12);
            rets.push((c2 / c1).ln());
        }
        let n = rets.len() as f64;
        if n < 3.0 {
            return (self.skew, self.kurt);
        }
        let mean = rets.iter().sum::<f64>() / n;
        let mut m2 = 0.0;
        let mut m3 = 0.0;
        let mut m4 = 0.0;
        for &r in &rets {
            let d = r - mean;
            let d2 = d * d;
            let d3 = d2 * d;
            let d4 = d3 * d;
            m2 += d2;
            m3 += d3;
            m4 += d4;
        }
        m2 /= n;
        m3 /= n;
        m4 /= n;
        let s2 = m2.max(1e-12);
        self.skew = m3 / s2.powf(1.5);
        self.kurt = m4 / (s2 * s2);
        (self.skew, self.kurt)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.skew, self.kurt)
    }

    pub fn window(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_higher_moments_creation() {
        let hm = HigherMoments::new(20);
        assert!(!hm.is_ready());
        assert_eq!(hm.skew, 0.0);
        assert_eq!(hm.kurt, 0.0);
        assert_eq!(hm.window(), 20);
    }

    #[test]
    fn test_higher_moments_finite() {
        let mut hm = HigherMoments::new(20);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let (skew, kurt) = hm.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(skew.is_finite(), "Skew should be finite");
            assert!(kurt.is_finite(), "Kurtosis should be finite");
        }
        assert!(hm.is_ready());
    }

    #[test]
    fn test_higher_moments_kurtosis_positive() {
        let mut hm = HigherMoments::new(20);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 0.5;
            hm.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(hm.is_ready());
        if let IndicatorValue::Double(_, kurt) = hm.value() {
            assert!(kurt >= 0.0, "Kurtosis should be non-negative, got {}", kurt);
        } else { panic!("Expected Double"); }
    }

    #[test]
    fn test_higher_moments_reset() {
        let mut hm = HigherMoments::new(20);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            hm.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(hm.is_ready());
        hm.reset();
        assert!(!hm.is_ready());
        assert_eq!(hm.skew, 0.0);
        assert_eq!(hm.kurt, 0.0);
    }
}
