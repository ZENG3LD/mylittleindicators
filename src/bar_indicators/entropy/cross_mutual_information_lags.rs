// Rolling Cross Mutual Information over multiple lags

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::entropy::mutual_information::MutualInformation;

#[derive(Clone)]
pub struct CrossMutualInformationLags {
    indicators: Vec<MutualInformation>,
    #[allow(dead_code)]
    lags: Vec<usize>,
    pub values: Vec<f64>,
}

impl CrossMutualInformationLags {
    pub fn new(window: usize, lags: &[usize], bins: usize, clip_abs: f64) -> Self {
        let mut indicators = Vec::new();
        for &lag in lags {
            indicators.push(MutualInformation::new(window, lag, bins, clip_abs));
        }
        Self {
            indicators,
            lags: lags.to_vec(),
            values: vec![0.0; lags.len()],
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        for mi in &mut self.indicators {
            mi.reset();
        }
        self.values.fill(0.0);
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.indicators.iter().all(|mi| mi.is_ready())
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> &[f64] {
        for (i, mi) in self.indicators.iter_mut().enumerate() {
            self.values[i] = mi.update_bar(o, h, l, c, v);
        }
        &self.values
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.values.first().copied().unwrap_or(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cross_mutual_information_lags_creation() {
        let cmi = CrossMutualInformationLags::new(30, &[1, 2, 3], 8, 0.05);
        assert!(!cmi.is_ready());
        assert_eq!(cmi.values.len(), 3);
    }

    #[test]
    fn test_cross_mutual_information_lags_warmup() {
        let mut cmi = CrossMutualInformationLags::new(20, &[1, 2], 8, 0.05);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            cmi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(cmi.is_ready());
    }

    #[test]
    fn test_cross_mutual_information_lags_values_finite() {
        let mut cmi = CrossMutualInformationLags::new(20, &[1, 2, 3], 8, 0.05);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let values = cmi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            for v in values {
                assert!(v.is_finite());
            }
        }
    }

    #[test]
    fn test_cross_mutual_information_lags_reset() {
        let mut cmi = CrossMutualInformationLags::new(20, &[1, 2], 8, 0.05);
        for i in 0..50 {
            cmi.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        cmi.reset();
        assert!(!cmi.is_ready());
    }
}
