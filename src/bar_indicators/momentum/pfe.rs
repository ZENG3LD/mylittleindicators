// Polarized Fractal Efficiency (PFE) - placeholder implementation

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct Pfe {
    window: usize,
    closes: ArrayVec<f64, 1024>,
    idx: usize,
    count: usize,
    value: f64,
}

impl Pfe {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.clamp(5, 1024),
            closes: ArrayVec::new(),
            idx: 0,
            count: 0,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.closes.clear();
        self.idx = 0;
        self.count = 0;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.window
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        // Store current close in ring buffer
        if self.count < self.window {
            self.closes.push(c);
            self.count += 1;
        } else {
            self.closes[self.idx] = c;
            self.idx = (self.idx + 1) % self.window;
        }

        if self.is_ready() {
            // Current close position (just written)
            // After first fill: idx points to next write position (oldest value)
            // So newest value is at (idx + window - 1) % window
            let newest_idx = (self.idx + self.window - 1) % self.window;
            let oldest_idx = self.idx; // oldest value in buffer

            let c_newest = self.closes[newest_idx];
            let c_oldest = self.closes[oldest_idx];

            // Numerator: straight-line distance from oldest to newest
            let numerator = (c_newest - c_oldest).abs();

            // Denominator: path length (sum of absolute deltas)
            let mut denom = 0.0;
            for i in 0..(self.window - 1) {
                let a_idx = (self.idx + i) % self.window;
                let b_idx = (self.idx + i + 1) % self.window;
                denom += (self.closes[b_idx] - self.closes[a_idx]).abs();
            }

            let eff = if denom > 1e-12 { numerator / denom } else { 0.0 };
            // Preserve direction: positive if price went up, negative if down
            let sign = if c_newest >= c_oldest { 1.0 } else { -1.0 };
            self.value = 100.0 * eff * sign;
        } else {
            self.value = 0.0;
        }
        self.value
    }

    pub fn period(&self) -> usize {
        self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pfe_creation() {
        let pfe = Pfe::new(10);
        assert!(!pfe.is_ready());
        assert_eq!(pfe.value().main(), 0.0);
        assert_eq!(pfe.period(), 10);
    }

    #[test]
    fn test_pfe_min_period() {
        let pfe = Pfe::new(2);
        assert_eq!(pfe.period(), 5); // min period is 5
    }

    #[test]
    fn test_pfe_max_period() {
        let pfe = Pfe::new(2000);
        assert_eq!(pfe.period(), 1024); // max period is 1024
    }

    #[test]
    fn test_pfe_is_ready_timing() {
        let mut pfe = Pfe::new(5);
        for i in 1..=10 {
            let price = 100.0 + i as f64;
            pfe.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if i < 5 {
                assert!(!pfe.is_ready(), "PFE should not be ready at bar {}", i);
            } else {
                assert!(pfe.is_ready(), "PFE should be ready at bar {}", i);
            }
        }
    }

    #[test]
    fn test_pfe_straight_uptrend() {
        let mut pfe = Pfe::new(5);
        // Perfectly straight uptrend: efficiency should be 100%
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            pfe.update_bar(price, price + 0.1, price - 0.1, price, 1000.0);
        }
        assert!(pfe.is_ready());
        // For straight line: numerator == denominator, so PFE = 100
        assert!(pfe.value().main() > 90.0, "PFE for straight uptrend should be near 100, got {}", pfe.value().main());
    }

    #[test]
    fn test_pfe_straight_downtrend() {
        let mut pfe = Pfe::new(5);
        // Perfectly straight downtrend: efficiency should be -100%
        for i in 1..=20 {
            let price = 200.0 - i as f64;
            pfe.update_bar(price, price + 0.1, price - 0.1, price, 1000.0);
        }
        assert!(pfe.is_ready());
        // For straight line down: numerator == denominator, so PFE = -100
        assert!(pfe.value().main() < -90.0, "PFE for straight downtrend should be near -100, got {}", pfe.value().main());
    }

    #[test]
    fn test_pfe_choppy_market() {
        let mut pfe = Pfe::new(5);
        // Choppy market: alternating up/down - low efficiency
        for i in 1..=20 {
            let price = 100.0 + if i % 2 == 0 { 5.0 } else { -5.0 };
            pfe.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pfe.is_ready());
        // In choppy market, path length >> straight distance, so |PFE| is low
        assert!(pfe.value().main().abs() < 50.0, "PFE in choppy market should be low, got {}", pfe.value().main());
    }

    #[test]
    fn test_pfe_range_bounds() {
        let mut pfe = Pfe::new(10);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = pfe.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            if pfe.is_ready() {
                assert!(value >= -100.0 && value <= 100.0, "PFE should be in [-100, 100], got {}", value);
            }
        }
    }

    #[test]
    fn test_pfe_reset() {
        let mut pfe = Pfe::new(10);
        for i in 1..=20 {
            let price = 100.0 + i as f64;
            pfe.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pfe.is_ready());
        pfe.reset();
        assert!(!pfe.is_ready());
        assert_eq!(pfe.value().main(), 0.0);
    }

    #[test]
    fn test_pfe_finite_values() {
        let mut pfe = Pfe::new(10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            let value = pfe.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "PFE should always be finite");
        }
    }
}
