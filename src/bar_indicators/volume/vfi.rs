// Volume Flow Indicator (VFI) - improved money flow proxy

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct Vfi {
    window: usize,
    value: f64,
    sum_flow: f64,
    sum_vol: f64,
}

impl Vfi {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            value: 0.0,
            sum_flow: 0.0,
            sum_vol: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.sum_flow = 0.0;
        self.sum_vol = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let tp = (h + l + c) / 3.0;
        let flow = tp * v;
        self.sum_flow += flow - self.sum_flow / self.window as f64;
        self.sum_vol += v - self.sum_vol / self.window as f64;
        self.value = if self.sum_vol.abs() > 1e-12 {
            self.sum_flow / self.sum_vol
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
    fn test_vfi_creation() {
        let vfi = Vfi::new(20);
        assert!(vfi.is_ready()); // Always ready
        assert_eq!(vfi.value().main(), 0.0);
    }

    #[test]
    fn test_vfi_update() {
        let mut vfi = Vfi::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            let value = vfi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_vfi_values_finite() {
        let mut vfi = Vfi::new(20);
        for i in 0..50 {
            let price = 100.0 + i as f64;
            let value = vfi.update_bar(price, price + 2.0, price - 2.0, price, 500.0 + i as f64 * 10.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_vfi_reset() {
        let mut vfi = Vfi::new(20);
        for i in 0..30 {
            vfi.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        vfi.reset();
        assert_eq!(vfi.value().main(), 0.0);
    }
}
