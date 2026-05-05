use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Qstick = SMA_n(Close - Open)
#[derive(Debug, Clone)]
pub struct Qstick {
    ma: MovingAverageProvider,
    value: f64,
}

impl Qstick {
    pub fn new(period: usize) -> Self {
        Self {
            ma: MovingAverageProvider::new(MovingAverageType::SMA, period.max(1)),
            value: 0.0,
        }
    }

    pub fn update_bar(&mut self, o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let diff = c - o;
        self.value = self.ma.update_bar(0.0, 0.0, 0.0, diff, 0.0);
        self.value
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.ma.is_ready()
    }
    pub fn reset(&mut self) {
        self.ma.reset();
        self.value = 0.0;
    }

    pub fn period(&self) -> usize {
        self.ma.period()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qstick_creation() {
        let qstick = Qstick::new(10);
        assert!(!qstick.is_ready());
        assert_eq!(qstick.value().main(), 0.0);
        assert_eq!(qstick.period(), 10);
    }

    #[test]
    fn test_qstick_min_period() {
        let qstick = Qstick::new(0);
        assert_eq!(qstick.period(), 1); // min period is 1
    }

    #[test]
    fn test_qstick_bullish_bars() {
        let mut qstick = Qstick::new(5);
        // Bullish bars: close > open
        for _ in 0..20 {
            qstick.update_bar(100.0, 110.0, 95.0, 108.0, 1000.0);
        }
        assert!(qstick.is_ready());
        // SMA of (close - open) = SMA(8) = 8
        assert!((qstick.value().main() - 8.0).abs() < 1e-10, "Qstick for bullish bars should be 8, got {}", qstick.value().main());
    }

    #[test]
    fn test_qstick_bearish_bars() {
        let mut qstick = Qstick::new(5);
        // Bearish bars: close < open
        for _ in 0..20 {
            qstick.update_bar(108.0, 110.0, 95.0, 100.0, 1000.0);
        }
        assert!(qstick.is_ready());
        // SMA of (close - open) = SMA(-8) = -8
        assert!((qstick.value().main() - (-8.0)).abs() < 1e-10, "Qstick for bearish bars should be -8, got {}", qstick.value().main());
    }

    #[test]
    fn test_qstick_doji() {
        let mut qstick = Qstick::new(5);
        // Doji bars: close == open
        for _ in 0..20 {
            qstick.update_bar(100.0, 110.0, 90.0, 100.0, 1000.0);
        }
        assert!(qstick.is_ready());
        assert!((qstick.value().main()).abs() < 1e-10, "Qstick for doji should be 0");
    }

    #[test]
    fn test_qstick_reset() {
        let mut qstick = Qstick::new(5);
        for _ in 0..20 {
            qstick.update_bar(100.0, 110.0, 95.0, 108.0, 1000.0);
        }
        assert!(qstick.is_ready());
        qstick.reset();
        assert!(!qstick.is_ready());
        assert_eq!(qstick.value().main(), 0.0);
    }

    #[test]
    fn test_qstick_is_ready_timing() {
        let mut qstick = Qstick::new(5);
        for i in 1..=10 {
            qstick.update_bar(100.0, 110.0, 95.0, 105.0, 1000.0);
            if i < 5 {
                assert!(!qstick.is_ready(), "Qstick should not be ready at bar {}", i);
            } else {
                assert!(qstick.is_ready(), "Qstick should be ready at bar {}", i);
            }
        }
    }

    #[test]
    fn test_qstick_mixed_bars() {
        let mut qstick = Qstick::new(4);
        // 2 bullish (+5), 2 bearish (-5) = average 0
        qstick.update_bar(100.0, 110.0, 95.0, 105.0, 1000.0); // +5
        qstick.update_bar(100.0, 110.0, 95.0, 105.0, 1000.0); // +5
        qstick.update_bar(105.0, 110.0, 95.0, 100.0, 1000.0); // -5
        qstick.update_bar(105.0, 110.0, 95.0, 100.0, 1000.0); // -5
        assert!(qstick.is_ready());
        assert!((qstick.value().main()).abs() < 1e-10, "Qstick for mixed equal bars should be 0");
    }

    #[test]
    fn test_qstick_finite_values() {
        let mut qstick = Qstick::new(10);
        for i in 1..=100 {
            let open = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let close = open + (i as f64 * 0.5).cos() * 5.0;
            let high = open.max(close) + 2.0;
            let low = open.min(close) - 2.0;
            let value = qstick.update_bar(open, high, low, close, 1000.0);
            assert!(value.is_finite(), "Qstick should always be finite");
        }
    }
}
