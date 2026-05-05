// Intraday Intensity Ratio (IIR)

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct IntradayIntensityRatio {
    window: usize,
    sum_iip: f64,
    count: usize,
    value: f64,
}

impl IntradayIntensityRatio {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            sum_iip: 0.0,
            count: 0,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.sum_iip = 0.0;
        self.count = 0;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.window
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let denom = (h - l).abs().max(1e-9);
        let iip = ((2.0 * c - h - l) / denom) * v; // Intraday Intensity Percent proxy summed as raw
                                                   // simple rolling sum via decay proxy (placeholder; not exact windowed)
        let alpha = 1.0 / (self.window as f64);
        self.sum_iip = (1.0 - alpha) * self.sum_iip + alpha * iip;
        self.count += 1;
        self.value = self.sum_iip.clamp(-1e9, 1e9);
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intraday_intensity_ratio_creation() {
        let iir = IntradayIntensityRatio::new(21);
        assert!(!iir.is_ready());
        assert_eq!(iir.value().main(), 0.0);
    }

    #[test]
    fn test_intraday_intensity_ratio_warmup() {
        let mut iir = IntradayIntensityRatio::new(14);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            iir.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(iir.is_ready());
    }

    #[test]
    fn test_intraday_intensity_ratio_values_finite() {
        let mut iir = IntradayIntensityRatio::new(14);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = iir.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_intraday_intensity_ratio_reset() {
        let mut iir = IntradayIntensityRatio::new(14);
        for i in 0..20 {
            iir.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        iir.reset();
        assert!(!iir.is_ready());
        assert_eq!(iir.value().main(), 0.0);
    }
}
