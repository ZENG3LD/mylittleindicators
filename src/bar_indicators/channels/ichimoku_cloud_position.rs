// Ichimoku Cloud Position: normalized position of price within current cloud (0..1)

use crate::bar_indicators::channels::ichimoku_cloud::IchimokuCloud;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct IchimokuCloudPosition {
    cloud: IchimokuCloud,
    value: f64,
}

impl Default for IchimokuCloudPosition {
    fn default() -> Self {
        Self::new()
    }
}

impl IchimokuCloudPosition {
    pub fn new() -> Self {
        Self {
            cloud: IchimokuCloud::new(),
            value: 0.5,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.cloud.reset();
        self.value = 0.5;
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
        let width = (top - bottom).abs();
        if width > 0.0 {
            self.value = ((c - bottom) / width).clamp(0.0, 1.0);
        } else {
            self.value = 0.5;
        }
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ichimoku_cloud_position_creation() {
        let icp = IchimokuCloudPosition::new();
        assert!(!icp.is_ready());
        assert_eq!(icp.value().main(), 0.5);
    }

    #[test]
    fn test_ichimoku_cloud_position_warmup() {
        let mut icp = IchimokuCloudPosition::new();
        for i in 0..55 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            icp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(icp.is_ready());
    }

    #[test]
    fn test_ichimoku_cloud_position_range() {
        let mut icp = IchimokuCloudPosition::new();
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = icp.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0 && value <= 1.0, "Position should be in [0, 1]");
        }
    }

    #[test]
    fn test_ichimoku_cloud_position_reset() {
        let mut icp = IchimokuCloudPosition::new();
        for i in 0..60 {
            icp.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        icp.reset();
        assert!(!icp.is_ready());
        assert_eq!(icp.value().main(), 0.5);
    }
}
