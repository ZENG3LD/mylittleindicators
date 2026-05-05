use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::channels::bollinger_bands::{BollingerBands, BollingerMode};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Lightweight metrics over Bollinger Bands: %B and Bandwidth
#[derive(Debug, Clone)]
pub struct BollingerMetrics {
    bb: BollingerBands,
    percent_b: f64,
    bandwidth: f64,
}

impl BollingerMetrics {
    pub fn new(period: usize, k: f64) -> Self {
        Self {
            bb: BollingerBands::new(
                period.max(1),
                k,
                BollingerMode::Close,
                MovingAverageType::SMA,
            ),
            percent_b: 0.5,
            bandwidth: 0.0,
        }
    }

    /// Update with OHLCV; returns (%B, bandwidth)
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> (f64, f64) {
        let (upper, middle, lower) = self.bb.update_bar(o, h, l, c, v);
        let width = (upper - lower).max(0.0);
        self.percent_b = if width > 0.0 {
            (c - lower) / width
        } else {
            0.5
        };
        // Bandwidth defined as (Upper-Lower)/Middle; guard middle
        self.bandwidth = if middle.abs() > 1e-12 {
            width / middle.abs()
        } else {
            0.0
        };
        (self.percent_b, self.bandwidth)
    }

    pub fn percent_b(&self) -> f64 {
        self.percent_b
    }
    pub fn bandwidth(&self) -> f64 {
        self.bandwidth
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.percent_b, self.bandwidth)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.bb.is_ready()
    }

    pub fn reset(&mut self) {
        self.bb.reset();
        self.percent_b = 0.5;
        self.bandwidth = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bollinger_metrics_creation() {
        let bm = BollingerMetrics::new(20, 2.0);
        assert!(!bm.is_ready());
        assert_eq!(bm.percent_b(), 0.5);
        assert_eq!(bm.bandwidth(), 0.0);
    }

    #[test]
    fn test_bollinger_metrics_warmup() {
        let mut bm = BollingerMetrics::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            bm.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(bm.is_ready());
    }

    #[test]
    fn test_bollinger_metrics_values() {
        let mut bm = BollingerMetrics::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (pct_b, bw) = bm.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(pct_b.is_finite(), "%B should be finite");
            assert!(bw >= 0.0, "Bandwidth should be non-negative");
        }
    }

    #[test]
    fn test_bollinger_metrics_reset() {
        let mut bm = BollingerMetrics::new(20, 2.0);
        for i in 0..25 {
            bm.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        bm.reset();
        assert!(!bm.is_ready());
        assert_eq!(bm.percent_b(), 0.5);
    }
}
