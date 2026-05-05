// VPIN (Volume-Synchronized Probability of Informed Trading) - simplified proxy

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Debug, Clone)]
pub struct Vpin {
    buckets: usize,
    bucket_volume: f64,
    pos_imbalance_sum: f64,
    total_buckets: usize,
    value: f64,
}

impl Vpin {
    pub fn new(buckets: usize, bucket_volume: f64) -> Self {
        Self {
            buckets: buckets.max(1),
            bucket_volume: bucket_volume.max(1.0),
            pos_imbalance_sum: 0.0,
            total_buckets: 0,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.pos_imbalance_sum = 0.0;
        self.total_buckets = 0;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.total_buckets >= self.buckets
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    // Placeholder: treat up-close volume as buys, down-close as sells, accumulate into fixed buckets
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        // Direction proxy
        let mid = 0.5 * (h + l);
        let buy_vol = if c >= mid { v } else { 0.0 };
        let sell_vol = if c < mid { v } else { 0.0 };
        let imbalance = (buy_vol - sell_vol).abs();

        // Fill buckets by fixed volume; for bars, approximate 1 bar ~ 1 bucket scaled by volume
        let bucket_count = (v / self.bucket_volume).max(1.0).floor() as usize;
        self.pos_imbalance_sum += imbalance * bucket_count as f64;
        self.total_buckets += bucket_count;
        let denom = self.total_buckets.max(self.buckets) as f64;
        self.value = (self.pos_imbalance_sum / denom) / self.bucket_volume;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vpin_creation() {
        let vpin = Vpin::new(50, 1000.0);
        assert!(!vpin.is_ready());
        assert_eq!(vpin.value().main(), 0.0);
    }

    #[test]
    fn test_vpin_warmup() {
        let mut vpin = Vpin::new(10, 100.0);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vpin.update_bar(price, price + 1.0, price - 1.0, price, 500.0);
        }
        assert!(vpin.is_ready());
    }

    #[test]
    fn test_vpin_values() {
        let mut vpin = Vpin::new(10, 100.0);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let value = vpin.update_bar(price, price + 1.0, price - 1.0, price, 500.0);
            assert!(value.is_finite());
            assert!(value >= 0.0, "VPIN should be non-negative");
        }
    }

    #[test]
    fn test_vpin_reset() {
        let mut vpin = Vpin::new(10, 100.0);
        for i in 0..20 {
            vpin.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 500.0);
        }
        vpin.reset();
        assert!(!vpin.is_ready());
        assert_eq!(vpin.value().main(), 0.0);
    }
}
