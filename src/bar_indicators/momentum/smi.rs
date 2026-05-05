use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Stochastic Momentum Index (SMI) simplified: SMI = 100 * (EMA(EMA(close - mid)) / (0.5*EMA(EMA(high-low))))
#[derive(Debug, Clone)]
pub struct Smi {
    ema1_diff: MovingAverageProvider,
    ema2_diff: MovingAverageProvider,
    ema1_range: MovingAverageProvider,
    ema2_range: MovingAverageProvider,
    signal_ma: MovingAverageProvider,
    value: f64,
    signal: f64,
    ready: bool,
}

impl Smi {
    pub fn new(period: usize, signal_period: usize) -> Self {
        let e = MovingAverageType::EMA;
        Self {
            ema1_diff: MovingAverageProvider::new(e, period.max(1)),
            ema2_diff: MovingAverageProvider::new(e, period.max(1)),
            ema1_range: MovingAverageProvider::new(e, period.max(1)),
            ema2_range: MovingAverageProvider::new(e, period.max(1)),
            signal_ma: MovingAverageProvider::new(e, signal_period.max(1)),
            value: 0.0,
            signal: 0.0,
            ready: false,
        }
    }

    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, _v: f64) -> f64 {
        let mid = 0.5 * (h + l);
        let diff = c - mid;
        let range = (h - l).max(1e-12);
        let d1 = self.ema1_diff.update_bar(0.0, 0.0, 0.0, diff, 0.0);
        let d2 = self.ema2_diff.update_bar(0.0, 0.0, 0.0, d1, 0.0);
        let r1 = self.ema1_range.update_bar(0.0, 0.0, 0.0, range, 0.0);
        let r2 = self.ema2_range.update_bar(0.0, 0.0, 0.0, r1, 0.0);
        let denom = (0.5 * r2).max(1e-12);
        self.value = 100.0 * d2 / denom;
        if self.ema2_diff.is_ready() && self.ema2_range.is_ready() {
            self.signal_ma.update_bar(0.0, 0.0, 0.0, self.value, 0.0);
            self.signal = self.signal_ma.value().main();
        }
        self.ready =
            self.ema2_diff.is_ready() && self.ema2_range.is_ready() && self.signal_ma.is_ready();
        self.value
    }

    /// Получить все значения SMI как типизированный IndicatorValue
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.value, self.signal)
    }

    /// Значение SMI линии (для обратной совместимости)
    pub fn value_smi(&self) -> f64 {
        self.value
    }

    /// Значение сигнальной линии
    pub fn value_signal(&self) -> f64 {
        self.signal
    }
    pub fn is_ready(&self) -> bool {
        self.ready
    }
    pub fn reset(&mut self) {
        self.ema1_diff.reset();
        self.ema2_diff.reset();
        self.ema1_range.reset();
        self.ema2_range.reset();
        self.signal_ma.reset();
        self.value = 0.0;
        self.signal = 0.0;
        self.ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smi_creation() {
        let smi = Smi::new(14, 3);
        assert!(!smi.is_ready());
        assert_eq!(smi.value_smi(), 0.0);
        assert_eq!(smi.value_signal(), 0.0);
    }

    #[test]
    fn test_smi_value_types() {
        let smi = Smi::new(14, 3);
        match smi.value() {
            IndicatorValue::Double(v, s) => {
                assert_eq!(v, 0.0);
                assert_eq!(s, 0.0);
            }
            _ => panic!("SMI should return Double value"),
        }
    }

    #[test]
    fn test_smi_uptrend() {
        let mut smi = Smi::new(5, 3);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            smi.update_bar(price, price + 3.0, price - 1.0, price + 2.0, 1000.0);
        }
        assert!(smi.is_ready());
        // In uptrend with close near high, SMI should be positive
        assert!(smi.value_smi() > 0.0, "SMI should be > 0 in uptrend, got {}", smi.value_smi());
    }

    #[test]
    fn test_smi_downtrend() {
        let mut smi = Smi::new(5, 3);
        for i in 1..=50 {
            let price = 200.0 - i as f64 * 2.0;
            smi.update_bar(price, price + 1.0, price - 3.0, price - 2.0, 1000.0);
        }
        assert!(smi.is_ready());
        // In downtrend with close near low, SMI should be negative
        assert!(smi.value_smi() < 0.0, "SMI should be < 0 in downtrend, got {}", smi.value_smi());
    }

    #[test]
    fn test_smi_range_bounds() {
        let mut smi = Smi::new(5, 3);
        for i in 1..=100 {
            let base = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let h = base + 5.0;
            let l = base - 5.0;
            let c = base + (i as f64 * 0.3).cos() * 3.0;
            let value = smi.update_bar(base, h, l, c, 1000.0);
            if smi.is_ready() {
                // SMI typically ranges from -100 to +100 but can exceed in edge cases
                assert!(value >= -150.0 && value <= 150.0, "SMI should be roughly in [-100, 100], got {}", value);
            }
        }
    }

    #[test]
    fn test_smi_reset() {
        let mut smi = Smi::new(5, 3);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            smi.update_bar(price, price + 2.0, price - 1.0, price + 1.0, 1000.0);
        }
        assert!(smi.is_ready());
        smi.reset();
        assert!(!smi.is_ready());
        assert_eq!(smi.value_smi(), 0.0);
        assert_eq!(smi.value_signal(), 0.0);
    }

    #[test]
    fn test_smi_signal_line() {
        let mut smi = Smi::new(5, 3);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            smi.update_bar(price, price + 2.0, price - 1.0, price + 1.0, 1000.0);
        }
        assert!(smi.is_ready());
        // Signal should be smoothed SMI
        assert!(smi.value_signal().is_finite());
        // Signal should follow SMI direction
        if smi.value_smi() > 0.0 {
            assert!(smi.value_signal() > 0.0, "Signal should follow SMI direction");
        }
    }

    #[test]
    fn test_smi_finite_values() {
        let mut smi = Smi::new(5, 3);
        for i in 1..=100 {
            let base = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            let h = base + 3.0;
            let l = base - 3.0;
            let c = base + (i as f64 * 0.5).cos() * 2.0;
            let value = smi.update_bar(base, h, l, c, 1000.0);
            assert!(value.is_finite(), "SMI should always be finite");
            assert!(smi.value_signal().is_finite(), "SMI signal should always be finite");
        }
    }
}
