// AVWAP Distance: relative distance of close to anchored VWAP (monthly)

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::levels::anchored_vwap::{
    AnchoredVwap, AnchoredVwapParams, AvwapAnchorMode,
};

#[derive(Clone)]
pub struct AvwapDistance {
    avwap: AnchoredVwap,
    pub value: f64,
}

impl Default for AvwapDistance {
    fn default() -> Self {
        Self::new()
    }
}

impl AvwapDistance {
    pub fn new() -> Self {
        let params = AnchoredVwapParams {
            mode: AvwapAnchorMode::Monthly,
        };
        Self {
            avwap: AnchoredVwap::new(params),
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.avwap.reset();
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.avwap.is_ready()
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn update_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
        unix_time_secs: i64,
    ) -> f64 {
        let v = self
            .avwap
            .update_bar(open, high, low, close, volume, unix_time_secs);
        self.value = if v != 0.0 { (close - v) / v } else { 0.0 };
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avwap_distance_creation() {
        let ad = AvwapDistance::new();
        assert!(!ad.is_ready());
        assert_eq!(ad.value, 0.0);
    }

    #[test]
    fn test_avwap_distance_update() {
        let mut ad = AvwapDistance::new();
        let ts = 1700000000_i64;
        let value = ad.update_bar(100.0, 102.0, 98.0, 101.0, 1000.0, ts);
        assert!(ad.is_ready());
        assert!(value.is_finite());
    }

    #[test]
    fn test_avwap_distance_values() {
        let mut ad = AvwapDistance::new();
        let ts = 1700000000_i64;
        for i in 0..10 {
            let price = 100.0 + i as f64;
            let value = ad.update_bar(price, price + 1.0, price - 1.0, price, 1000.0, ts + i * 86400);
            assert!(value.is_finite(), "Distance should be finite");
        }
    }

    #[test]
    fn test_avwap_distance_reset() {
        let mut ad = AvwapDistance::new();
        ad.update_bar(100.0, 102.0, 98.0, 101.0, 1000.0, 1700000000);
        ad.reset();
        assert!(!ad.is_ready());
        assert_eq!(ad.value, 0.0);
    }
}
