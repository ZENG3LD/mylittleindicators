// Intraday Momentum Index (IMI)
// Placeholder implementation: windowed ratio of up-momentum to total momentum over period

use std::collections::VecDeque;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct IntradayMomentumIndex {
    period: usize,
    ups: VecDeque<f64>,
    downs: VecDeque<f64>,
    sum_up: f64,
    sum_down: f64,
    value: f64,
}

impl IntradayMomentumIndex {
    pub fn new(period: usize) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            ups: VecDeque::with_capacity(p + 1),
            downs: VecDeque::with_capacity(p + 1),
            sum_up: 0.0,
            sum_down: 0.0,
            value: 50.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.ups.clear();
        self.downs.clear();
        self.sum_up = 0.0;
        self.sum_down = 0.0;
        self.value = 50.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ups.len() >= self.period
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, _v: f64) -> f64 {
        // Basic intraday momentum proxy
        let range = (h - l).abs().max(1e-12);
        let delta = c - o;
        let up = if delta > 0.0 { delta / range } else { 0.0 };
        let down = if delta < 0.0 { (-delta) / range } else { 0.0 };

        self.ups.push_back(up);
        self.sum_up += up;
        self.downs.push_back(down);
        self.sum_down += down;
        if self.ups.len() > self.period {
            if let Some(x) = self.ups.pop_front() {
                self.sum_up -= x;
            }
        }
        if self.downs.len() > self.period {
            if let Some(x) = self.downs.pop_front() {
                self.sum_down -= x;
            }
        }

        let denom = (self.sum_up + self.sum_down).max(1e-12);
        self.value = 100.0 * (self.sum_up / denom);
        self.value
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_imi_creation() {
        let imi = IntradayMomentumIndex::new(14);
        assert!(!imi.is_ready());
        assert_eq!(imi.value().main(), 50.0); // default neutral
        assert_eq!(imi.period(), 14);
    }

    #[test]
    fn test_imi_uptrend() {
        let mut imi = IntradayMomentumIndex::new(10);
        for i in 1..=30 {
            let open = 100.0 + i as f64;
            let close = open + 2.0; // close > open = bullish
            imi.update_bar(open, close + 0.5, open - 0.5, close, 1000.0);
        }
        assert!(imi.is_ready());
        // More up momentum than down, IMI > 50
        assert!(imi.value().main() > 50.0, "IMI should be > 50 in uptrend, got {}", imi.value().main());
    }

    #[test]
    fn test_imi_downtrend() {
        let mut imi = IntradayMomentumIndex::new(10);
        for i in 1..=30 {
            let open = 200.0 - i as f64;
            let close = open - 2.0; // close < open = bearish
            imi.update_bar(open, open + 0.5, close - 0.5, close, 1000.0);
        }
        assert!(imi.is_ready());
        // More down momentum than up, IMI < 50
        assert!(imi.value().main() < 50.0, "IMI should be < 50 in downtrend, got {}", imi.value().main());
    }

    #[test]
    fn test_imi_range() {
        let mut imi = IntradayMomentumIndex::new(10);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = imi.update_bar(price - 1.0, price + 2.0, price - 2.0, price + 1.0, 1000.0);
            assert!(value >= 0.0 && value <= 100.0, "IMI should be in [0, 100], got {}", value);
        }
    }

    #[test]
    fn test_imi_reset() {
        let mut imi = IntradayMomentumIndex::new(10);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            imi.update_bar(price, price + 2.0, price - 2.0, price + 1.0, 1000.0);
        }
        assert!(imi.is_ready());
        imi.reset();
        assert!(!imi.is_ready());
        assert_eq!(imi.value().main(), 50.0);
    }
}
