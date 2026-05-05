// Bollinger Bands (stddev по последнему окну period, классика)
// (c) 2024

use arrayvec::ArrayVec;
use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct BbPeriod {
    period: usize,
    stddev_mult: f64,
    buffer: ArrayVec<f64, 512>,
    ma: MovingAverageProvider,
    middle: f64,
    upper: f64,
    lower: f64,
}

impl BbPeriod {
    pub fn new(period: usize, stddev_mult: f64, ma_type: MovingAverageType) -> Self {
        assert!(period <= 64, "BbPeriod: period too large for ArrayVec window");
        let ma = MovingAverageProvider::new(ma_type, period);
        Self {
            period,
            stddev_mult,
            buffer: ArrayVec::new(),
            ma,
            middle: 0.0,
            upper: 0.0,
            lower: 0.0,
        }
    }
    /// Обновить BB новым баром (используется typical price)
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, _volume: f64) -> (f64, f64, f64) {
        let typical = (high + low + close) / 3.0;
        if self.buffer.len() == self.period {
            self.buffer.remove(0);
        }
        self.buffer.push(typical); // буфер ограничен period
        self.ma.update_bar(0.0, 0.0, 0.0, typical, 0.0);
        let len = self.buffer.len();
        if len < self.period {
            self.middle = 0.0;
            self.upper = 0.0;
            self.lower = 0.0;
            return (self.middle, self.upper, self.lower);
        }
        let ma_value = self.ma.value().main();
        let stddev = (self.buffer.iter().map(|&v| (v - ma_value).powi(2)).sum::<f64>() / len as f64).sqrt();
        self.middle = ma_value;
        self.upper = ma_value + self.stddev_mult * stddev;
        self.lower = ma_value - self.stddev_mult * stddev;
        (self.middle, self.upper, self.lower)
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.middle, self.upper, self.lower)
    }
    pub fn is_ready(&self) -> bool {
        self.buffer.len() == self.period
    }
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.ma.reset();
        self.middle = 0.0;
        self.upper = 0.0;
        self.lower = 0.0;
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bb_period_creation() {
        let bb = BbPeriod::new(20, 2.0, MovingAverageType::SMA);
        assert!(!bb.is_ready());
        assert_eq!(bb.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
        assert_eq!(bb.period(), 20);
    }

    #[test]
    fn test_bb_period_bands() {
        let mut bb = BbPeriod::new(20, 2.0, MovingAverageType::SMA);
        for i in 1..=30 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 5.0;
            bb.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
        }
        assert!(bb.is_ready());
        if let IndicatorValue::Triple(middle, upper, lower) = bb.value() {
            assert!(upper > middle, "Upper band should be > middle");
            assert!(lower < middle, "Lower band should be < middle");
        } else { panic!("Expected Triple"); }
    }

    #[test]
    fn test_bb_period_finite() {
        let mut bb = BbPeriod::new(20, 2.0, MovingAverageType::EMA);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let (m, u, l) = bb.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(m.is_finite() && u.is_finite() && l.is_finite());
        }
    }

    #[test]
    fn test_bb_period_reset() {
        let mut bb = BbPeriod::new(20, 2.0, MovingAverageType::SMA);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            bb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(bb.is_ready());
        bb.reset();
        assert!(!bb.is_ready());
        assert_eq!(bb.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }
}






















