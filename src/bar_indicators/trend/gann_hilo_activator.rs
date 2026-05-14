// Gann HiLo Activator — stateful trailing stop that switches between HI and LO channels.
//
// Algorithm:
//   sma_high[i] = SMA(high, period)
//   sma_low[i]  = SMA(low,  period)
//
//   State starts as Long (activator = sma_low).
//   While Long:
//     activator = sma_low
//     if close < sma_low  → switch to Short, activator = sma_high
//   While Short:
//     activator = sma_high
//     if close > sma_high → switch to Long,  activator = sma_low
//
// Output: Double(activator, side)  where side = +1 (long) / -1 (short)

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone, PartialEq)]
enum Side {
    Long,
    Short,
}

#[derive(Debug, Clone)]
pub struct GannHiLoActivator {
    ma_high: MovingAverageProvider,
    ma_low: MovingAverageProvider,
    side: Side,
    activator: f64,
    upper: f64,
    lower: f64,
}

impl GannHiLoActivator {
    pub fn new(period: usize) -> Self {
        Self {
            ma_high: MovingAverageProvider::new(MovingAverageType::SMA, period.max(1)),
            ma_low: MovingAverageProvider::new(MovingAverageType::SMA, period.max(1)),
            side: Side::Long,
            activator: 0.0,
            upper: 0.0,
            lower: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ma_high.reset();
        self.ma_low.reset();
        self.side = Side::Long;
        self.activator = 0.0;
        self.upper = 0.0;
        self.lower = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ma_high.is_ready() && self.ma_low.is_ready()
    }

    /// Returns `Double(activator, side)` where side = +1 (long) or -1 (short).
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        let side_val = if self.side == Side::Long { 1.0 } else { -1.0 };
        IndicatorValue::Double(self.activator, side_val)
    }

    /// Returns `(upper_sma, lower_sma)` — raw SMA lines regardless of state.
    #[inline]
    pub fn bands(&self) -> (f64, f64) {
        (self.upper, self.lower)
    }

    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, _v: f64) -> (f64, f64) {
        let uh = self.ma_high.update_bar(0.0, h, h, h, 0.0);
        let dl = self.ma_low.update_bar(0.0, l, l, l, 0.0);
        self.upper = uh;
        self.lower = dl;

        if self.is_ready() {
            // State machine: switch side on close crossing activator
            match self.side {
                Side::Long => {
                    if c < dl {
                        self.side = Side::Short;
                        self.activator = uh;
                    } else {
                        self.activator = dl;
                    }
                }
                Side::Short => {
                    if c > uh {
                        self.side = Side::Long;
                        self.activator = dl;
                    } else {
                        self.activator = uh;
                    }
                }
            }
        }

        let side_val = if self.side == Side::Long { 1.0 } else { -1.0 };
        (self.activator, side_val)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gann_hilo_creation() {
        let gann = GannHiLoActivator::new(10);
        assert!(!gann.is_ready());
        if let IndicatorValue::Double(act, side) = gann.value() {
            assert_eq!(act, 0.0);
            assert_eq!(side, 1.0); // starts Long
        }
    }

    #[test]
    fn test_gann_hilo_warmup() {
        let mut gann = GannHiLoActivator::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            gann.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(gann.is_ready());
    }

    #[test]
    fn test_gann_hilo_uptrend_stays_long() {
        let mut gann = GannHiLoActivator::new(5);
        // Strong uptrend — close always above sma_low → stays Long
        for i in 0..20 {
            let price = 100.0 + i as f64 * 3.0;
            gann.update_bar(price, price + 2.0, price - 0.5, price, 1000.0);
        }
        assert!(gann.is_ready());
        if let IndicatorValue::Double(_, side) = gann.value() {
            assert_eq!(side, 1.0, "Strong uptrend should keep side = Long (+1)");
        }
    }

    #[test]
    fn test_gann_hilo_downtrend_goes_short() {
        let mut gann = GannHiLoActivator::new(5);
        // Strong downtrend — close always below sma_low → switches to Short
        for i in 0..20 {
            let price = 200.0 - i as f64 * 3.0;
            gann.update_bar(price, price + 0.5, price - 2.0, price, 1000.0);
        }
        assert!(gann.is_ready());
        if let IndicatorValue::Double(_, side) = gann.value() {
            assert_eq!(side, -1.0, "Strong downtrend should flip side = Short (-1)");
        }
    }

    #[test]
    fn test_gann_hilo_reset() {
        let mut gann = GannHiLoActivator::new(10);
        for i in 0..15 {
            gann.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        gann.reset();
        assert!(!gann.is_ready());
        if let IndicatorValue::Double(act, side) = gann.value() {
            assert_eq!(act, 0.0);
            assert_eq!(side, 1.0);
        }
    }

    #[test]
    fn test_gann_hilo_finite() {
        let mut gann = GannHiLoActivator::new(10);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 10.0;
            let (act, side) = gann.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(act.is_finite());
            assert!(side == 1.0 || side == -1.0);
        }
    }
}
