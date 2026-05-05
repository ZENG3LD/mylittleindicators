// Price Volume Trend (PVT)

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct PriceVolumeTrend {
    prev_close: f64,
    initialized: bool,
    value: f64,
}

impl Default for PriceVolumeTrend {
    fn default() -> Self {
        Self::new()
    }
}

impl PriceVolumeTrend {
    pub fn new() -> Self {
        Self {
            prev_close: 0.0,
            initialized: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.prev_close = 0.0;
        self.initialized = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, v: f64) -> f64 {
        if !self.initialized {
            self.prev_close = c;
            self.initialized = true;
            return self.value;
        }
        let pct_change = if self.prev_close.abs() > 1e-12 {
            (c - self.prev_close) / self.prev_close
        } else {
            0.0
        };
        self.value += pct_change * v;
        self.prev_close = c;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pvt_creation() {
        let pvt = PriceVolumeTrend::new();
        assert!(!pvt.is_ready());
        assert_eq!(pvt.value().main(), 0.0);
    }

    #[test]
    fn test_pvt_warmup() {
        let mut pvt = PriceVolumeTrend::new();
        pvt.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0);
        assert!(pvt.is_ready());
    }

    #[test]
    fn test_pvt_accumulation() {
        let mut pvt = PriceVolumeTrend::new();
        // First bar initializes
        pvt.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0);
        // Price goes up - positive PVT
        let value = pvt.update_bar(101.0, 102.0, 100.0, 101.0, 1000.0);
        assert!(value > 0.0, "Rising price should add positive PVT");
    }

    #[test]
    fn test_pvt_values_finite() {
        let mut pvt = PriceVolumeTrend::new();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = pvt.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_pvt_reset() {
        let mut pvt = PriceVolumeTrend::new();
        for i in 0..10 {
            pvt.update_bar(100.0 + i as f64, 102.0, 98.0, 101.0, 1000.0);
        }
        pvt.reset();
        assert!(!pvt.is_ready());
        assert_eq!(pvt.value().main(), 0.0);
    }
}
