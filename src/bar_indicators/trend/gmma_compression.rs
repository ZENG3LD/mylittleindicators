// GMMA Compression/Expansion score based on MA cluster spreads

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct GmmaCompression {
    ma_type: MovingAverageType,
    fast_periods: Vec<usize>,
    slow_periods: Vec<usize>,
    fast_emas: Vec<MovingAverageProvider>,
    slow_emas: Vec<MovingAverageProvider>,
    score: f64,
}

impl GmmaCompression {
    /// Create GMMA Compression with default MA type (EMA)
    pub fn new() -> Self {
        Self::new_with_ma_type(MovingAverageType::EMA)
    }

    /// Create GMMA Compression with specified MA type
    pub fn new_with_ma_type(ma_type: MovingAverageType) -> Self {
        // Default GMMA sets
        let fast_periods = vec![3usize, 5, 8, 10, 12, 15];
        let slow_periods = vec![30usize, 35, 40, 45, 50, 60];
        let fast_emas = fast_periods
            .iter()
            .map(|&p| MovingAverageProvider::new(ma_type, p))
            .collect();
        let slow_emas = slow_periods
            .iter()
            .map(|&p| MovingAverageProvider::new(ma_type, p))
            .collect();
        Self {
            ma_type,
            fast_periods,
            slow_periods,
            fast_emas,
            slow_emas,
            score: 0.0,
        }
    }

    /// Set the MA type and reset the indicator
    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> f64 {
        for ma in &mut self.fast_emas {
            ma.update_bar(0.0, 0.0, 0.0, close, 0.0);
        }
        for ma in &mut self.slow_emas {
            ma.update_bar(0.0, 0.0, 0.0, close, 0.0);
        }
        // Compute average spread within clusters and between clusters
        if self.is_ready() {
            let (fast_min, fast_max) = Self::min_max(&self.fast_emas);
            let (slow_min, slow_max) = Self::min_max(&self.slow_emas);
            let intra_fast = fast_max - fast_min;
            let intra_slow = slow_max - slow_min;
            let inter = (Self::avg(&self.fast_emas) - Self::avg(&self.slow_emas)).abs();
            // Compression score: smaller intra spreads and larger inter separation => trend
            self.score = inter / (1e-9 + intra_fast + intra_slow);
        }
        self.score
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.score)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.fast_emas.iter().all(|m| m.is_ready()) && self.slow_emas.iter().all(|m| m.is_ready())
    }

    pub fn reset(&mut self) {
        // Recreate all MAs with the current MA type
        self.fast_emas = self.fast_periods
            .iter()
            .map(|&p| MovingAverageProvider::new(self.ma_type, p))
            .collect();
        self.slow_emas = self.slow_periods
            .iter()
            .map(|&p| MovingAverageProvider::new(self.ma_type, p))
            .collect();
        self.score = 0.0;
    }

    fn min_max(mas: &Vec<MovingAverageProvider>) -> (f64, f64) {
        let mut mn = f64::INFINITY;
        let mut mx = f64::NEG_INFINITY;
        for m in mas {
            let v = m.value().main();
            if v < mn {
                mn = v;
            }
            if v > mx {
                mx = v;
            }
        }
        (mn, mx)
    }
    fn avg(mas: &Vec<MovingAverageProvider>) -> f64 {
        let mut s = 0.0;
        for m in mas {
            s += m.value().main();
        }
        s / (mas.len() as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gmma_compression_creation() {
        let gmma = GmmaCompression::new();
        assert!(!gmma.is_ready());
        assert_eq!(gmma.value().main(), 0.0);
    }

    #[test]
    fn test_gmma_compression_warmup() {
        let mut gmma = GmmaCompression::new();
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            gmma.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(gmma.is_ready());
    }

    #[test]
    fn test_gmma_compression_values_finite() {
        let mut gmma = GmmaCompression::new();
        for i in 0..80 {
            let price = 100.0 + i as f64;
            let value = gmma.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_gmma_compression_reset() {
        let mut gmma = GmmaCompression::new();
        for i in 0..80 {
            gmma.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        gmma.reset();
        assert!(!gmma.is_ready());
        assert_eq!(gmma.value().main(), 0.0);
    }
}
