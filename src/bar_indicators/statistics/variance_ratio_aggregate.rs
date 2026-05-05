// Variance Ratio Aggregate: combine multiple VR(m) into a single score

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::statistics::variance_ratio::VarianceRatio;

#[derive(Clone)]
pub struct VarianceRatioAggregate {
    #[allow(dead_code)]
    windows: Vec<usize>,
    #[allow(dead_code)]
    ms: Vec<usize>,
    vr_list: Vec<VarianceRatio>,
    pub value: f64,
}

impl VarianceRatioAggregate {
    pub fn new(configs: &[(usize, usize)]) -> Self {
        let mut windows = Vec::new();
        let mut ms = Vec::new();
        let mut vr_list = Vec::new();
        for &(w, m) in configs {
            windows.push(w);
            ms.push(m);
            vr_list.push(VarianceRatio::new(w, m));
        }
        Self {
            windows,
            ms,
            vr_list,
            value: 1.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        for vr in &mut self.vr_list {
            vr.reset();
        }
        self.value = 1.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.vr_list.iter().all(|v| v.is_ready())
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let mut sum = 0.0;
        let mut cnt = 0.0;
        for vr in &mut self.vr_list {
            sum += vr.update_bar(o, h, l, c, v);
            cnt += 1.0;
        }
        if cnt > 0.0 {
            self.value = sum / cnt;
        }
        self.value
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variance_ratio_aggregate_creation() {
        let vra = VarianceRatioAggregate::new(&[(50, 5), (50, 10)]);
        assert!(!vra.is_ready());
        assert_eq!(vra.value, 1.0);
    }

    #[test]
    fn test_variance_ratio_aggregate_warmup() {
        let mut vra = VarianceRatioAggregate::new(&[(50, 5), (50, 10)]);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vra.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vra.is_ready());
    }

    #[test]
    fn test_variance_ratio_aggregate_positive() {
        let mut vra = VarianceRatioAggregate::new(&[(50, 5), (50, 10)]);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vra.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value > 0.0, "VR aggregate should be positive");
        }
    }

    #[test]
    fn test_variance_ratio_aggregate_reset() {
        let mut vra = VarianceRatioAggregate::new(&[(50, 5), (50, 10)]);
        for i in 0..60 {
            vra.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        vra.reset();
        assert!(!vra.is_ready());
        assert_eq!(vra.value, 1.0);
    }
}
