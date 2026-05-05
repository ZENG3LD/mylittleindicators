// Variance Ratio Z-Aggregate: z-score over mean of multiple VR(m)

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::statistics::variance_ratio::VarianceRatio;

#[derive(Clone)]
pub struct VarianceRatioZAggregate {
    vr_list: Vec<VarianceRatio>,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl VarianceRatioZAggregate {
    pub fn new(configs: &[(usize, usize)], z_window: usize) -> Self {
        let mut vr_list = Vec::new();
        for &(w, m) in configs {
            vr_list.push(VarianceRatio::new(w, m));
        }
        let w = z_window.max(20);
        Self {
            vr_list,
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        for v in &mut self.vr_list {
            v.reset();
        }
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.vr_list.iter().all(|v| v.is_ready())
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let mut sum = 0.0;
        let mut cnt = 0.0;
        for vr in &mut self.vr_list {
            sum += vr.update_bar(o, h, l, c, v);
            cnt += 1.0;
        }
        let mean = if cnt > 0.0 { sum / cnt } else { 1.0 };
        self.buf[self.idx] = mean;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut m = 0.0;
            for &x in &self.buf {
                m += x;
            }
            m /= self.window as f64;
            let mut s = 0.0;
            for &x in &self.buf {
                let d = x - m;
                s += d * d;
            }
            s = (s / (self.window as f64)).sqrt().max(1e-9);
            self.value = (mean - m) / s;
        } else {
            self.value = 0.0;
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
    fn test_variance_ratio_z_aggregate_creation() {
        let vrza = VarianceRatioZAggregate::new(&[(50, 5), (50, 10)], 30);
        assert!(!vrza.is_ready());
        assert_eq!(vrza.value, 0.0);
    }

    #[test]
    fn test_variance_ratio_z_aggregate_warmup() {
        let mut vrza = VarianceRatioZAggregate::new(&[(50, 5), (50, 10)], 30);
        for i in 0..90 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vrza.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vrza.is_ready());
    }

    #[test]
    fn test_variance_ratio_z_aggregate_values() {
        let mut vrza = VarianceRatioZAggregate::new(&[(50, 5), (50, 10)], 30);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vrza.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "Z-score should be finite");
        }
    }

    #[test]
    fn test_variance_ratio_z_aggregate_reset() {
        let mut vrza = VarianceRatioZAggregate::new(&[(50, 5), (50, 10)], 30);
        for i in 0..90 {
            vrza.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        vrza.reset();
        assert!(!vrza.is_ready());
        assert_eq!(vrza.value, 0.0);
    }
}
