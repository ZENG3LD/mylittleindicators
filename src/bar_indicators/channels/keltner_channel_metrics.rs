use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::channels::keltner_channel::{KeltnerChannel, KeltnerMode};
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Lightweight metrics over Keltner Channel: width and position
#[derive(Debug, Clone)]
pub struct KeltnerMetrics {
    kc: KeltnerChannel,
    width: f64,
    position: f64,
}

impl KeltnerMetrics {
    pub fn new(period: usize, atr_mult: f64) -> Self {
        Self {
            kc: KeltnerChannel::new(
                period,
                atr_mult,
                KeltnerMode::Classic,
                MovingAverageType::SMA,
                MovingAverageType::RMA,
            ),
            width: 0.0,
            position: 0.5,
        }
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> (f64, f64) {
        let (upper, _middle, lower) = self.kc.update_bar(o, h, l, c, v);
        self.width = upper - lower;
        self.position = if self.width > 0.0 {
            (c - lower) / self.width
        } else {
            0.5
        };
        (self.width, self.position)
    }
    pub fn width(&self) -> f64 {
        self.width
    }
    pub fn position(&self) -> f64 {
        self.position
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.width, self.position)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.kc.is_ready()
    }

    pub fn reset(&mut self) {
        self.kc.reset();
        self.width = 0.0;
        self.position = 0.5;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keltner_metrics_creation() {
        let km = KeltnerMetrics::new(20, 2.0);
        assert!(!km.is_ready());
        assert_eq!(km.width(), 0.0);
        assert_eq!(km.position(), 0.5);
    }

    #[test]
    fn test_keltner_metrics_warmup() {
        let mut km = KeltnerMetrics::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            km.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(km.is_ready());
    }

    #[test]
    fn test_keltner_metrics_values() {
        let mut km = KeltnerMetrics::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (width, position) = km.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(width >= 0.0, "Width should be non-negative");
            assert!(position.is_finite(), "Position should be finite");
        }
    }

    #[test]
    fn test_keltner_metrics_reset() {
        let mut km = KeltnerMetrics::new(20, 2.0);
        for i in 0..25 {
            km.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        km.reset();
        assert!(!km.is_ready());
        assert_eq!(km.position(), 0.5);
    }
}
