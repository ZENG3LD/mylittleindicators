// Chande Kroll Stop - ATR-based stop levels variant

use crate::bar_indicators::average::MovingAverageType;
use crate::bar_indicators::volatility::atr::Atr;
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct ChandeKrollStop {
    #[allow(dead_code)]
    atr_period: usize,
    k: f64,
    hh_period: usize,
    ll_period: usize,
    atr: Atr,
    highs: ArrayVec<f64, 512>,
    lows: ArrayVec<f64, 512>,
    long_stop: f64,
    short_stop: f64,
    ready: bool,
}

impl ChandeKrollStop {
    pub fn new(atr_period: usize, k: f64, hh_period: usize, ll_period: usize) -> Self {
        Self {
            atr_period: atr_period.max(1),
            k: if k > 0.0 { k } else { 1.5 },
            hh_period: hh_period.max(1),
            ll_period: ll_period.max(1),
            atr: Atr::new(atr_period.max(1), MovingAverageType::RMA),
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            long_stop: 0.0,
            short_stop: 0.0,
            ready: false,
        }
    }

    pub fn update_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> (f64, f64) {
        let atr = self.atr.update_bar(open, high, low, close, volume);
        if self.highs.len() >= self.hh_period {
            self.highs.remove(0);
        }
        if self.lows.len() >= self.ll_period {
            self.lows.remove(0);
        }
        self.highs.push(high);
        self.lows.push(low);
        let hh = self.highs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let ll = self.lows.iter().cloned().fold(f64::INFINITY, f64::min);
        self.long_stop = hh - self.k * atr;
        self.short_stop = ll + self.k * atr;
        self.ready = self.atr.is_ready()
            && self.highs.len() >= self.hh_period
            && self.lows.len() >= self.ll_period;
        (self.long_stop, self.short_stop)
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.long_stop)
    }
    #[inline]
    pub fn levels(&self) -> (f64, f64) {
        (self.long_stop, self.short_stop)
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    pub fn reset(&mut self) {
        self.atr.reset();
        self.highs.clear();
        self.lows.clear();
        self.long_stop = 0.0;
        self.short_stop = 0.0;
        self.ready = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chande_kroll_stop_creation() {
        let ind = ChandeKrollStop::new(10, 1.5, 10, 10);
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }

    #[test]
    fn test_chande_kroll_stop_warmup() {
        let mut ind = ChandeKrollStop::new(10, 1.5, 10, 10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_chande_kroll_stop_values_finite() {
        let mut ind = ChandeKrollStop::new(10, 1.5, 10, 10);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (long, short) = ind.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(long.is_finite());
            assert!(short.is_finite());
        }
    }

    #[test]
    fn test_chande_kroll_stop_reset() {
        let mut ind = ChandeKrollStop::new(10, 1.5, 10, 10);
        for i in 0..20 {
            ind.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value().main(), 0.0);
    }
}
