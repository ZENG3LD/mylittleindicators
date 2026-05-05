// FVG duration/intensity score based on recent detected gaps

use crate::bar_indicators::levels::fvg_detector::FvgDetector;
use crate::bar_indicators::indicator_value::IndicatorValue;
use std::collections::VecDeque;

#[derive(Clone)]
pub struct FvgDurationIntensityScore {
    det: FvgDetector,
    window: usize,
    hits: Vec<f64>,
    idx: usize,
    filled: bool,
    pub current_value: f64,
    buffer: VecDeque<(f64, f64, f64, f64)>,  // (open, high, low, close)
}

impl FvgDurationIntensityScore {
    pub fn new(lookback: usize, agg_window: usize) -> Self {
        let _ = lookback;
        let w = agg_window.max(20);
        Self {
            det: FvgDetector::new(),
            window: w,
            hits: vec![0.0; w],
            idx: 0,
            filled: false,
            current_value: 0.0,
            buffer: VecDeque::with_capacity(3),
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.det.reset();
        self.hits.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.current_value = 0.0;
        self.buffer.clear();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.buffer.len() >= 3
    }

    /// Update with OHLCV bar
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        // Maintain 3-bar ring buffer
        if self.buffer.len() >= 3 {
            self.buffer.pop_front();
        }
        self.buffer.push_back((open, high, low, close));

        // Need minimum 3 bars
        if self.buffer.len() < 3 {
            return 0.0;
        }

        // Extract triplet
        let (o0, h0, l0, c0) = self.buffer[0];
        let (o1, h1, l1, c1) = self.buffer[1];
        let (o2, h2, l2, c2) = self.buffer[2];

        // Call update_triplet_and_score
        self.update_triplet_and_score(
            o0, h0, l0, c0,
            o1, h1, l1, c1,
            o2, h2, l2, c2,
        )
    }

    /// Get current indicator value
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_value)
    }
    pub fn update_triplet_and_score(
        &mut self,
        o0: f64,
        h0: f64,
        l0: f64,
        c0: f64,
        o1: f64,
        h1: f64,
        l1: f64,
        c1: f64,
        o2: f64,
        h2: f64,
        l2: f64,
        c2: f64,
    ) -> f64 {
        let (is_bull, is_bear) = self
            .det
            .update_triplet(o0, h0, l0, c0, o1, h1, l1, c1, o2, h2, l2, c2);
        let hit = if is_bull || is_bear { 1.0 } else { 0.0 };
        self.hits[self.idx] = hit;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut s = 0.0;
            for &x in &self.hits {
                s += x;
            }
            self.current_value = s / (self.window as f64);
        }
        self.current_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fvg_duration_intensity_creation() {
        let fdi = FvgDurationIntensityScore::new(10, 30);
        assert!(!fdi.is_ready());
        assert_eq!(fdi.current_value, 0.0);
    }

    #[test]
    fn test_fvg_duration_intensity_warmup() {
        let mut fdi = FvgDurationIntensityScore::new(10, 30);
        for i in 0..40 {
            let price = 100.0 + i as f64;
            fdi.update_bar(price, price + 1.0, price - 1.0, price, 0.0);
        }
        assert!(fdi.is_ready());
    }

    #[test]
    fn test_fvg_duration_intensity_range() {
        let mut fdi = FvgDurationIntensityScore::new(10, 30);
        for i in 0..50 {
            let price = 100.0 + i as f64;
            let value = fdi.update_triplet_and_score(
                price, price + 1.0, price - 1.0, price,
                price + 1.0, price + 2.0, price, price + 1.5,
                price + 2.0, price + 3.0, price + 1.0, price + 2.5,
            );
            assert!(value >= 0.0 && value <= 1.0, "Score should be in [0, 1]");
        }
    }

    #[test]
    fn test_fvg_duration_intensity_reset() {
        let mut fdi = FvgDurationIntensityScore::new(10, 30);
        for i in 0..40 {
            let price = 100.0 + i as f64;
            fdi.update_triplet_and_score(
                price, price + 1.0, price - 1.0, price,
                price + 1.0, price + 2.0, price, price + 1.5,
                price + 2.0, price + 3.0, price + 1.0, price + 2.5,
            );
        }
        fdi.reset();
        assert!(!fdi.is_ready());
        assert_eq!(fdi.current_value, 0.0);
    }
}
