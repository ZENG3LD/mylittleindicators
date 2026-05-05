// Alternative FVG intensity score: exponential weighting of recent FVG gaps magnitude

use crate::bar_indicators::levels::fvg_detector::FvgDetector;
use crate::bar_indicators::indicator_value::IndicatorValue;
use std::collections::VecDeque;

#[derive(Clone)]
pub struct FvgIntensityAltScore {
    det: FvgDetector,
    alpha: f64,
    pub current_value: f64,
    buffer: VecDeque<(f64, f64, f64, f64)>,  // (open, high, low, close)
}

impl FvgIntensityAltScore {
    pub fn new(alpha: f64) -> Self {
        Self {
            det: FvgDetector::new(),
            alpha: alpha.clamp(0.0, 1.0),
            current_value: 0.0,
            buffer: VecDeque::with_capacity(3),
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.det.reset();
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

        // Need minimum 3 bars for triplet
        if self.buffer.len() < 3 {
            return 0.0;
        }

        // Extract triplet (bar0 = oldest, bar2 = newest)
        let (o0, h0, l0, c0) = self.buffer[0];
        let (o1, h1, l1, c1) = self.buffer[1];
        let (o2, h2, l2, c2) = self.buffer[2];

        // Call update_triplet
        self.update_triplet(
            o0, h0, l0, c0,
            o1, h1, l1, c1,
            o2, h2, l2, c2,
        )
    }

    /// Get current indicator value
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_value)
    }
    pub fn update_triplet(
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
        let (bull, bear) = self
            .det
            .update_triplet(o0, h0, l0, c0, o1, h1, l1, c1, o2, h2, l2, c2);
        let gap = if bull {
            (l1 - h0).max(l1 - h2).max(0.0)
        } else if bear {
            (l0 - h1).min(l2 - h1).abs().max(0.0)
        } else {
            0.0
        };
        self.current_value = self.alpha * gap + (1.0 - self.alpha) * self.current_value;
        self.current_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fvg_intensity_alt_creation() {
        let fia = FvgIntensityAltScore::new(0.1);
        assert!(!fia.is_ready()); // Not ready until 3 bars
        assert_eq!(fia.current_value, 0.0);
    }

    #[test]
    fn test_fvg_intensity_alt_update() {
        let mut fia = FvgIntensityAltScore::new(0.1);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            fia.update_triplet(
                price, price + 1.0, price - 1.0, price,
                price + 1.0, price + 2.0, price, price + 1.5,
                price + 2.0, price + 3.0, price + 1.0, price + 2.5,
            );
        }
        assert!(fia.current_value >= 0.0);
    }

    #[test]
    fn test_fvg_intensity_alt_non_negative() {
        let mut fia = FvgIntensityAltScore::new(0.2);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = fia.update_triplet(
                price, price + 1.0, price - 1.0, price,
                price + 1.0, price + 2.0, price, price + 1.5,
                price + 2.0, price + 3.0, price + 1.0, price + 2.5,
            );
            assert!(value >= 0.0, "Value should be non-negative");
        }
    }

    #[test]
    fn test_fvg_intensity_alt_reset() {
        let mut fia = FvgIntensityAltScore::new(0.1);
        fia.update_triplet(
            100.0, 101.0, 99.0, 100.0,
            101.0, 102.0, 100.0, 101.5,
            102.0, 103.0, 101.0, 102.5,
        );
        fia.reset();
        assert_eq!(fia.current_value, 0.0);
    }
}
