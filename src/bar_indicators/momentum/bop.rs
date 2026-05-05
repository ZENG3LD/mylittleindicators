/// Balance of Power (BOP) = (Close - Open) / (High - Low)
use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct Bop {
    value: f64,
}

impl Default for Bop {
    fn default() -> Self {
        Self::new()
    }
}

impl Bop {
    pub fn new() -> Self {
        Self { value: 0.0 }
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, _v: f64) -> f64 {
        let denom = (h - l).abs().max(1e-12);
        self.value = (c - o) / denom;
        self.value
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        true
    }
    pub fn reset(&mut self) {
        self.value = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bop_creation() {
        let bop = Bop::new();
        assert!(bop.is_ready()); // BOP always ready (stateless)
        assert_eq!(bop.value().main(), 0.0);
    }

    #[test]
    fn test_bop_bullish_bar() {
        let mut bop = Bop::new();
        // Close > Open = bullish
        let value = bop.update_bar(100.0, 110.0, 95.0, 108.0, 1000.0);
        // (108 - 100) / (110 - 95) = 8 / 15 ≈ 0.533
        assert!(value > 0.0, "BOP should be positive for bullish bar");
        assert!((value - 8.0 / 15.0).abs() < 1e-10);
    }

    #[test]
    fn test_bop_bearish_bar() {
        let mut bop = Bop::new();
        // Close < Open = bearish
        let value = bop.update_bar(100.0, 105.0, 90.0, 92.0, 1000.0);
        // (92 - 100) / (105 - 90) = -8 / 15 ≈ -0.533
        assert!(value < 0.0, "BOP should be negative for bearish bar");
        assert!((value - (-8.0 / 15.0)).abs() < 1e-10);
    }

    #[test]
    fn test_bop_doji() {
        let mut bop = Bop::new();
        // Close == Open
        let value = bop.update_bar(100.0, 105.0, 95.0, 100.0, 1000.0);
        assert_eq!(value, 0.0, "BOP should be 0 for doji (close == open)");
    }

    #[test]
    fn test_bop_full_range_bullish() {
        let mut bop = Bop::new();
        // Close = High, Open = Low
        let value = bop.update_bar(90.0, 110.0, 90.0, 110.0, 1000.0);
        // (110 - 90) / (110 - 90) = 1.0
        assert!((value - 1.0).abs() < 1e-10, "BOP should be 1.0 for full bullish bar");
    }

    #[test]
    fn test_bop_full_range_bearish() {
        let mut bop = Bop::new();
        // Close = Low, Open = High
        let value = bop.update_bar(110.0, 110.0, 90.0, 90.0, 1000.0);
        // (90 - 110) / (110 - 90) = -1.0
        assert!((value - (-1.0)).abs() < 1e-10, "BOP should be -1.0 for full bearish bar");
    }

    #[test]
    fn test_bop_reset() {
        let mut bop = Bop::new();
        bop.update_bar(100.0, 110.0, 95.0, 105.0, 1000.0);
        assert!(bop.value().main() != 0.0);
        bop.reset();
        assert_eq!(bop.value().main(), 0.0);
    }

    #[test]
    fn test_bop_sequence() {
        let mut bop = Bop::new();
        // Alternating bullish/bearish bars
        let v1 = bop.update_bar(100.0, 110.0, 95.0, 108.0, 1000.0);
        assert!(v1 > 0.0);
        let v2 = bop.update_bar(108.0, 112.0, 100.0, 102.0, 1000.0);
        assert!(v2 < 0.0);
        let v3 = bop.update_bar(102.0, 115.0, 100.0, 113.0, 1000.0);
        assert!(v3 > 0.0);
    }

    #[test]
    fn test_bop_range_bounds() {
        let mut bop = Bop::new();
        // BOP is bounded between -1 and 1
        for i in 0..100 {
            let o = 100.0 + (i % 20) as f64;
            let c = 100.0 + ((i + 10) % 20) as f64;
            let h = o.max(c) + 5.0;
            let l = o.min(c) - 5.0;
            let value = bop.update_bar(o, h, l, c, 1000.0);
            assert!(value >= -1.0 && value <= 1.0, "BOP should be in [-1, 1], got {}", value);
        }
    }
}
