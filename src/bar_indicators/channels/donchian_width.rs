// Donchian Width: upper - lower band of Donchian Channel

use crate::bar_indicators::channels::donchian_channel::DonchianChannel;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct DonchianWidth {
    dc: DonchianChannel,
    value: f64,
}

impl DonchianWidth {
    pub fn new(period: usize) -> Self {
        Self {
            dc: DonchianChannel::new(period.max(2)),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.dc.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.dc.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let (upper, lower, _mid) = self.dc.update_bar(o, h, l, c, v);
        self.value = upper - lower;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_donchian_width_creation() {
        let dw = DonchianWidth::new(20);
        assert!(!dw.is_ready());
        assert_eq!(dw.value().main(), 0.0);
    }

    #[test]
    fn test_donchian_width_warmup() {
        let mut dw = DonchianWidth::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            dw.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(dw.is_ready());
    }

    #[test]
    fn test_donchian_width_positive() {
        let mut dw = DonchianWidth::new(20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = dw.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            if dw.is_ready() {
                assert!(value >= 0.0, "Width should be non-negative");
            }
        }
    }

    #[test]
    fn test_donchian_width_reset() {
        let mut dw = DonchianWidth::new(20);
        for i in 0..25 {
            dw.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        dw.reset();
        assert!(!dw.is_ready());
        assert_eq!(dw.value().main(), 0.0);
    }
}
