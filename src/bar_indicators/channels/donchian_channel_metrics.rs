use crate::bar_indicators::channels::donchian_channel::DonchianChannel;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Lightweight metrics over Donchian Channel: width and price position in channel
#[derive(Debug, Clone)]
pub struct DonchianMetrics {
    dc: DonchianChannel,
    width: f64,
    position: f64,
}

impl DonchianMetrics {
    pub fn new(period: usize) -> Self {
        Self {
            dc: DonchianChannel::new(period),
            width: 0.0,
            position: 0.5,
        }
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> (f64, f64) {
        let (upper, _mid, lower) = self.dc.update_bar(o, h, l, c, v);
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
        self.dc.is_ready()
    }

    pub fn reset(&mut self) {
        self.dc.reset();
        self.width = 0.0;
        self.position = 0.5;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_donchian_metrics_creation() {
        let dm = DonchianMetrics::new(20);
        assert!(!dm.is_ready());
        assert_eq!(dm.width(), 0.0);
        assert_eq!(dm.position(), 0.5);
    }

    #[test]
    fn test_donchian_metrics_warmup() {
        let mut dm = DonchianMetrics::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            dm.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(dm.is_ready());
    }

    #[test]
    fn test_donchian_metrics_values() {
        let mut dm = DonchianMetrics::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (width, position) = dm.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(width >= 0.0, "Width should be non-negative");
            assert!(position.is_finite(), "Position should be finite");
        }
    }

    #[test]
    fn test_donchian_metrics_reset() {
        let mut dm = DonchianMetrics::new(20);
        for i in 0..25 {
            dm.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        dm.reset();
        assert!(!dm.is_ready());
        assert_eq!(dm.position(), 0.5);
    }
}
