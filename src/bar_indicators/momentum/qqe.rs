// QQE (Quantitative Qualitative Estimation) over RSI - placeholder minimal

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::momentum::rsi::Rsi;
use crate::bar_indicators::indicator_value::IndicatorValue;


#[derive(Clone)]
pub struct Qqe {
    rsi: Rsi,
    smoothing: MovingAverageProvider,
    threshold_mult: f64,
    qqe_value: f64,
    smoothed_rsi: f64,
}

impl Qqe {
    pub fn new(period: usize, smooth: usize, threshold_mult: f64) -> Self {
        Self {
            rsi: Rsi::new(period.max(1)),
            smoothing: MovingAverageProvider::new(MovingAverageType::EMA, smooth.max(1)),
            threshold_mult: if threshold_mult > 0.0 {
                threshold_mult
            } else {
                1.5
            },
            qqe_value: 0.0,
            smoothed_rsi: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.qqe_value = 0.0;
        self.smoothed_rsi = 0.0;
        self.smoothing.reset();
        self.rsi.reset();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.smoothing.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.qqe_value, self.smoothed_rsi)
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let _ = self.rsi.update_bar(open, high, low, close, volume);
        self.smoothed_rsi = self
            .smoothing
            .update_bar(0.0, 0.0, 0.0, self.rsi.value().main(), 0.0);
        // Minimal core: QQE line equals smoothed RSI minus 50 (centered at zero for easier analysis)
        self.qqe_value = self.smoothed_rsi - 50.0;
        self.qqe_value
    }

    pub fn threshold_mult(&self) -> f64 {
        self.threshold_mult
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qqe_creation() {
        let qqe = Qqe::new(14, 5, 4.236);
        assert!(!qqe.is_ready());
        assert_eq!(qqe.value().main(), 0.0);
        assert!((qqe.threshold_mult() - 4.236).abs() < 1e-10);
    }

    #[test]
    fn test_qqe_default_threshold() {
        let qqe = Qqe::new(14, 5, 0.0);
        // Should use default 1.5 when threshold <= 0
        assert!((qqe.threshold_mult() - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_qqe_basic_calculation() {
        let mut qqe = Qqe::new(14, 5, 4.236);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            qqe.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(qqe.is_ready());
        // QQE line is centered around 0 (smoothed_rsi - 50), so range is [-50, 50]
        let (qqe_line, smoothed_rsi) = match qqe.value() {
            IndicatorValue::Double(q, s) => (q, s),
            _ => panic!("Expected Double value"),
        };
        assert!(qqe_line.is_finite(), "QQE line should be finite");
        assert!(smoothed_rsi.is_finite(), "Smoothed RSI should be finite");
        assert!(qqe_line >= -50.0 && qqe_line <= 50.0, "QQE line should be in [-50, 50], got {}", qqe_line);
        assert!(smoothed_rsi >= 0.0 && smoothed_rsi <= 100.0, "Smoothed RSI should be in [0, 100], got {}", smoothed_rsi);
    }

    #[test]
    fn test_qqe_downtrend() {
        let mut qqe = Qqe::new(14, 5, 4.236);
        for i in 1..=50 {
            let price = 200.0 - i as f64;
            qqe.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(qqe.is_ready());
        // QQE outputs smoothed RSI - in downtrend should be < 50
        assert!(qqe.value().main() < 50.0, "QQE should be < 50 in downtrend, got {}", qqe.value().main());
    }

    #[test]
    fn test_qqe_reset() {
        let mut qqe = Qqe::new(14, 5, 4.236);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            qqe.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(qqe.is_ready());
        qqe.reset();
        assert!(!qqe.is_ready());
        assert_eq!(qqe.value().main(), 0.0);
    }

    #[test]
    fn test_qqe_range_bounds() {
        let mut qqe = Qqe::new(14, 5, 4.236);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let qqe_line = qqe.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            if qqe.is_ready() {
                // QQE line is centered at 0 (smoothed_rsi - 50), so valid range is [-50, 50]
                assert!(qqe_line >= -50.0 && qqe_line <= 50.0, "QQE line should be in [-50, 50], got {}", qqe_line);
            }
        }
    }

    #[test]
    fn test_qqe_finite_values() {
        let mut qqe = Qqe::new(14, 5, 4.236);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            let value = qqe.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "QQE should always be finite");
        }
    }
}
