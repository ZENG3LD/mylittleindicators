// Central Pivot Range (CPR): TC, BC, Pivot

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct CentralPivotRange {
    pivot: f64,
    tc: f64,
    bc: f64,
    ready: bool,
}

impl Default for CentralPivotRange {
    fn default() -> Self {
        Self::new()
    }
}

impl CentralPivotRange {
    pub fn new() -> Self {
        Self {
            pivot: 0.0,
            tc: 0.0,
            bc: 0.0,
            ready: false,
        }
    }
    pub fn reset(&mut self) {
        self.pivot = 0.0;
        self.tc = 0.0;
        self.bc = 0.0;
        self.ready = false;
    }
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.bc, self.pivot, self.tc)
    }
    // classic daily CPR; uses current bar's HLC as placeholder
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, _v: f64) -> (f64, f64, f64) {
        self.pivot = (h + l + c) / 3.0;
        let tc = self.pivot - (l);
        let bc = (h) - (self.pivot);
        // ensure tc >= bc by convention
        self.tc = self.pivot + tc.abs().max(bc.abs());
        self.bc = self.pivot - tc.abs().max(bc.abs());
        self.ready = true;
        (self.bc, self.pivot, self.tc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_central_pivot_range_creation() {
        let cpr = CentralPivotRange::new();
        assert!(!cpr.is_ready());
    }

    #[test]
    fn test_central_pivot_range_update() {
        let mut cpr = CentralPivotRange::new();
        let (bc, pivot, tc) = cpr.update_bar(100.0, 105.0, 95.0, 100.0, 1000.0);
        assert!(cpr.is_ready());
        // pivot = (105 + 95 + 100) / 3 = 100
        assert!((pivot - 100.0).abs() < 0.001);
        assert!(bc < pivot);
        assert!(pivot < tc);
    }

    #[test]
    fn test_central_pivot_range_order() {
        let mut cpr = CentralPivotRange::new();
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (bc, pivot, tc) = cpr.update_bar(price, price + 5.0, price - 5.0, price, 1000.0);
            assert!(bc <= pivot, "BC should be <= pivot");
            assert!(pivot <= tc, "Pivot should be <= TC");
        }
    }

    #[test]
    fn test_central_pivot_range_reset() {
        let mut cpr = CentralPivotRange::new();
        cpr.update_bar(100.0, 105.0, 95.0, 100.0, 1000.0);
        cpr.reset();
        assert!(!cpr.is_ready());
    }
}
