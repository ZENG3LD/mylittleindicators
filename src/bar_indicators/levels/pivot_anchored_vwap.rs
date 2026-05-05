// Pivot-Anchored VWAP: anchor at last detected pivot high/low (simple HH/LL)

use crate::bar_indicators::indicator_value::IndicatorValue;
#[derive(Clone)]
pub struct PivotAnchoredVwap {
    lookback: usize,
    highs: Vec<f64>,
    lows: Vec<f64>,
    idx: usize,
    filled: bool,
    // accumulators since last pivot
    pv: f64,
    vv: f64,
    value: f64,
}

impl PivotAnchoredVwap {
    pub fn new(lookback: usize) -> Self {
        Self {
            lookback: lookback.max(3),
            highs: vec![0.0; lookback.max(3)],
            lows: vec![0.0; lookback.max(3)],
            idx: 0,
            filled: false,
            pv: 0.0,
            vv: 0.0,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.highs.fill(0.0);
        self.lows.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.pv = 0.0;
        self.vv = 0.0;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        // detect simple pivot: if current high is new max or current low new min over window-1 previous
        self.highs[self.idx] = high;
        self.lows[self.idx] = low;
        let len = self.lookback;
        let mut prev_max = f64::MIN;
        let mut prev_min = f64::MAX;
        for k in 1..len {
            let i = (self.idx + len - k) % len;
            prev_max = prev_max.max(self.highs[i]);
            prev_min = prev_min.min(self.lows[i]);
        }
        let is_pivot_high = high >= prev_max && self.filled;
        let is_pivot_low = low <= prev_min && self.filled;

        if is_pivot_high || is_pivot_low {
            self.pv = 0.0;
            self.vv = 0.0;
        }
        // accumulate
        self.pv += close * volume;
        self.vv += volume.max(1e-12);
        self.value = self.pv / self.vv;

        // advance ring
        self.idx = (self.idx + 1) % len;
        if self.idx == 0 {
            self.filled = true;
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pivot_anchored_vwap_creation() {
        let pav = PivotAnchoredVwap::new(10);
        assert!(!pav.is_ready());
        assert_eq!(pav.value().main(), 0.0);
    }

    #[test]
    fn test_pivot_anchored_vwap_warmup() {
        let mut pav = PivotAnchoredVwap::new(10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pav.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(pav.is_ready());
    }

    #[test]
    fn test_pivot_anchored_vwap_positive() {
        let mut pav = PivotAnchoredVwap::new(10);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            let value = pav.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value > 0.0, "VWAP should be positive");
        }
    }

    #[test]
    fn test_pivot_anchored_vwap_reset() {
        let mut pav = PivotAnchoredVwap::new(10);
        for i in 0..15 {
            pav.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0, 1000.0);
        }
        pav.reset();
        assert!(!pav.is_ready());
        assert_eq!(pav.value().main(), 0.0);
    }
}
