// Ichimoku Cloud Thickness: current_cloud_top - current_cloud_bottom

use crate::bar_indicators::channels::ichimoku_cloud::IchimokuCloud;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct IchimokuCloudThickness {
    cloud: IchimokuCloud,
    value: f64,
}

impl Default for IchimokuCloudThickness {
    fn default() -> Self {
        Self::new()
    }
}

impl IchimokuCloudThickness {
    pub fn new() -> Self {
        Self {
            cloud: IchimokuCloud::new(),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.cloud.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.cloud.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let _ = self.cloud.update_bar(o, h, l, c, v);
        let (top, bottom) = self.cloud.current_cloud();
        self.value = (top - bottom).abs();
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ichimoku_cloud_thickness_creation() {
        let ict = IchimokuCloudThickness::new();
        assert!(!ict.is_ready());
        assert_eq!(ict.value().main(), 0.0);
    }

    #[test]
    fn test_ichimoku_cloud_thickness_warmup() {
        let mut ict = IchimokuCloudThickness::new();
        for i in 0..55 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ict.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ict.is_ready());
    }

    #[test]
    fn test_ichimoku_cloud_thickness_positive() {
        let mut ict = IchimokuCloudThickness::new();
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ict.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "Thickness should be non-negative");
        }
    }

    #[test]
    fn test_ichimoku_cloud_thickness_reset() {
        let mut ict = IchimokuCloudThickness::new();
        for i in 0..60 {
            ict.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        ict.reset();
        assert!(!ict.is_ready());
        assert_eq!(ict.value().main(), 0.0);
    }
}
