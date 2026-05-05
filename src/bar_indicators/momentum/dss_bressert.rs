// DSS Bressert (Double Smoothed Stochastic) - placeholder

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct DssBressert {
    k_period: usize,
    smooth_period: usize,
    highs: ArrayVec<f64, 512>,
    lows: ArrayVec<f64, 512>,
    idx: usize,
    count: usize,
    ema1: MovingAverageProvider,
    ema2: MovingAverageProvider,
    value: f64,
}

impl DssBressert {
    pub fn new(k_period: usize, smooth_period: usize) -> Self {
        let k = k_period.clamp(2, 512);
        let s = smooth_period.max(1);
        Self {
            k_period: k,
            smooth_period: s,
            highs: ArrayVec::new(),
            lows: ArrayVec::new(),
            idx: 0,
            count: 0,
            ema1: MovingAverageProvider::new(MovingAverageType::EMA, s),
            ema2: MovingAverageProvider::new(MovingAverageType::EMA, s),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.idx = 0;
        self.count = 0;
        self.ema1.reset();
        self.ema2.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.k_period && self.ema2.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, _v: f64) -> f64 {
        if self.count < self.k_period {
            self.highs.push(h);
            self.lows.push(l);
            self.count += 1;
        } else {
            self.highs[self.idx] = h;
            self.lows[self.idx] = l;
        }
        self.idx = (self.idx + 1) % self.k_period;

        // raw %K over window
        let len = self.count.min(self.k_period);
        let mut max_h = f64::NEG_INFINITY;
        let mut min_l = f64::INFINITY;
        for i in 0..len {
            max_h = max_h.max(self.highs[i]);
            min_l = min_l.min(self.lows[i]);
        }
        let range = (max_h - min_l).abs().max(1e-12);
        let k = (c - min_l) / range * 100.0;

        let s1 = self.ema1.update_bar(k, k, k, k, 0.0);
        let s2 = self.ema2.update_bar(s1, s1, s1, s1, 0.0);
        self.value = s2;
        self.value
    }

    pub fn k_period(&self) -> usize {
        self.k_period
    }

    pub fn smooth_period(&self) -> usize {
        self.smooth_period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dss_creation() {
        let dss = DssBressert::new(10, 3);
        assert!(!dss.is_ready());
        assert_eq!(dss.value().main(), 0.0);
        assert_eq!(dss.k_period(), 10);
        assert_eq!(dss.smooth_period(), 3);
    }

    #[test]
    fn test_dss_uptrend() {
        let mut dss = DssBressert::new(10, 3);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            dss.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(dss.is_ready());
        // In uptrend with close near high, DSS should be high
        assert!(dss.value().main() > 50.0, "DSS should be > 50 in uptrend, got {}", dss.value().main());
    }

    #[test]
    fn test_dss_downtrend() {
        let mut dss = DssBressert::new(10, 3);
        for i in 1..=30 {
            let price = 200.0 - i as f64 * 2.0;
            dss.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(dss.is_ready());
        // In downtrend with close near low, DSS should be low
        assert!(dss.value().main() < 50.0, "DSS should be < 50 in downtrend, got {}", dss.value().main());
    }

    #[test]
    fn test_dss_range() {
        let mut dss = DssBressert::new(10, 3);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = dss.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            if dss.is_ready() {
                assert!(value >= 0.0 && value <= 100.0, "DSS should be in [0, 100], got {}", value);
            }
        }
    }

    #[test]
    fn test_dss_reset() {
        let mut dss = DssBressert::new(10, 3);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            dss.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(dss.is_ready());
        dss.reset();
        assert!(!dss.is_ready());
        assert_eq!(dss.value().main(), 0.0);
    }
}
