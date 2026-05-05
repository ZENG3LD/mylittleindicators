// Donchian Breakout: breakout signals relative to Donchian Channel

use crate::bar_indicators::channels::donchian_channel::DonchianChannel;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct DonchianBreakout {
    dc: DonchianChannel,
    breakout: i8,
}

impl DonchianBreakout {
    pub fn new(period: usize) -> Self {
        Self {
            dc: DonchianChannel::new(period.max(2)),
            breakout: 0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.dc.reset();
        self.breakout = 0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.dc.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.breakout)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> i8 {
        let (upper, lower, _mid) = self.dc.update_bar(o, h, l, c, v);
        self.breakout = if c > upper {
            1
        } else if c < lower {
            -1
        } else {
            0
        };
        self.breakout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_donchian_breakout_creation() {
        let ind = DonchianBreakout::new(20);
        assert!(!ind.is_ready());
        assert_eq!(ind.value().as_signal(), Some(0));
    }

    #[test]
    fn test_donchian_breakout_warmup() {
        let mut ind = DonchianBreakout::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_donchian_breakout_signals() {
        let mut ind = DonchianBreakout::new(5);
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let signal = ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(signal >= -1 && signal <= 1);
        }
    }

    #[test]
    fn test_donchian_breakout_reset() {
        let mut ind = DonchianBreakout::new(10);
        for i in 0..15 {
            ind.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().as_signal(), Some(0));
    }
}
