// Center of Gravity (COG) - Ehlers proxy (placeholder)

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct CenterOfGravity {
    period: usize,
    buf: ArrayVec<f64, 512>,
    idx: usize,
    count: usize,
    value: f64,
}

impl CenterOfGravity {
    pub fn new(period: usize) -> Self {
        Self {
            period: period.clamp(2, 512),
            buf: ArrayVec::new(),
            idx: 0,
            count: 0,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.buf.clear();
        self.idx = 0;
        self.count = 0;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        if self.count < self.period {
            self.buf.push(c);
            self.count += 1;
            self.idx = self.count % self.period;
        } else {
            self.buf[self.idx] = c;
            self.idx = (self.idx + 1) % self.period;
        }
        if self.is_ready() {
            let mut num = 0.0;
            let mut den = 0.0;
            let n = self.period;
            for i in 0..n {
                let price = self.buf[(self.idx + i) % n];
                let w = (i + 1) as f64;
                num += w * price;
                den += price;
            }
            self.value = if den.abs() > 1e-12 {
                -(num / den) + (n as f64 + 1.0) / 2.0
            } else {
                0.0
            };
        }
        self.value
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cog_creation() {
        let cog = CenterOfGravity::new(10);
        assert!(!cog.is_ready());
        assert_eq!(cog.value().main(), 0.0);
        assert_eq!(cog.period(), 10);
    }

    #[test]
    fn test_cog_min_period() {
        let cog = CenterOfGravity::new(1);
        assert_eq!(cog.period(), 2); // min period is 2
    }

    #[test]
    fn test_cog_max_period() {
        let cog = CenterOfGravity::new(1000);
        assert_eq!(cog.period(), 512); // max period is 512
    }

    #[test]
    fn test_cog_is_ready_timing() {
        let mut cog = CenterOfGravity::new(5);
        for i in 1..=10 {
            let price = 100.0 + i as f64;
            cog.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if i < 5 {
                assert!(!cog.is_ready(), "COG should not be ready at bar {}", i);
            } else {
                assert!(cog.is_ready(), "COG should be ready at bar {}", i);
            }
        }
    }

    #[test]
    fn test_cog_constant_price() {
        let mut cog = CenterOfGravity::new(5);
        for _ in 0..20 {
            cog.update_bar(100.0, 105.0, 95.0, 100.0, 1000.0);
        }
        assert!(cog.is_ready());
        // For constant prices, COG = -(sum(i*p)/sum(p)) + (n+1)/2
        // = -(p*sum(i))/(p*n) + (n+1)/2 = -sum(i)/n + (n+1)/2
        // For n=5: sum(i)=1+2+3+4+5=15, -(15/5) + 3 = -3 + 3 = 0
        assert!((cog.value().main()).abs() < 1e-10, "COG for constant prices should be near 0");
    }

    #[test]
    fn test_cog_reset() {
        let mut cog = CenterOfGravity::new(5);
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            cog.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(cog.is_ready());
        cog.reset();
        assert!(!cog.is_ready());
        assert_eq!(cog.value().main(), 0.0);
    }

    #[test]
    fn test_cog_finite_values() {
        let mut cog = CenterOfGravity::new(10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = cog.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "COG should always be finite");
        }
    }

    #[test]
    fn test_cog_uptrend() {
        let mut cog = CenterOfGravity::new(10);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            cog.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(cog.is_ready());
        // In uptrend, recent prices are higher, COG should be negative (recent > average)
        assert!(cog.value().main() < 0.0, "COG should be negative in uptrend (recent prices weighted higher)");
    }

    #[test]
    fn test_cog_downtrend() {
        let mut cog = CenterOfGravity::new(10);
        for i in 1..=50 {
            let price = 200.0 - i as f64 * 2.0;
            cog.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(cog.is_ready());
        // In downtrend, recent prices are lower, COG should be positive
        assert!(cog.value().main() > 0.0, "COG should be positive in downtrend");
    }
}
