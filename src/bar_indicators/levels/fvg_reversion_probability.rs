// FVG reversion probability within horizon H bars after detection

use crate::bar_indicators::levels::fvg_detector::FvgDetector;
use crate::bar_indicators::indicator_value::IndicatorValue;
use std::collections::VecDeque;

#[derive(Clone)]
struct PendingGap {
    upper: f64,
    lower: f64,
    remain: usize,
}

#[derive(Clone)]
pub struct FvgReversionProbability {
    det: FvgDetector,
    horizon: usize,
    active: Vec<PendingGap>,
    total: usize,
    hits: usize,
    pub current_value: f64,
    buffer: VecDeque<(f64, f64, f64, f64)>,  // (open, high, low, close)
}

impl FvgReversionProbability {
    pub fn new(horizon: usize) -> Self {
        Self {
            det: FvgDetector::new(),
            horizon: horizon.clamp(1, 50),
            active: Vec::new(),
            total: 0,
            hits: 0,
            current_value: 0.0,
            buffer: VecDeque::with_capacity(4),
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.det.reset();
        self.active.clear();
        self.total = 0;
        self.hits = 0;
        self.current_value = 0.0;
        self.buffer.clear();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.buffer.len() >= 4
    }

    /// Update with OHLCV bar
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        // Maintain 4-bar ring buffer (3 for triplet + 1 for next_close)
        if self.buffer.len() >= 4 {
            self.buffer.pop_front();
        }
        self.buffer.push_back((open, high, low, close));

        // Need minimum 4 bars
        if self.buffer.len() < 4 {
            return 0.0;
        }

        // Extract triplet (bars 0-2) + next_close (bar 3)
        let (o0, h0, l0, c0) = self.buffer[0];
        let (o1, h1, l1, c1) = self.buffer[1];
        let (o2, h2, l2, c2) = self.buffer[2];
        let (_o3, _h3, _l3, c3) = self.buffer[3];  // next_close

        // Call update_triplet_and_progress
        self.update_triplet_and_progress(
            o0, h0, l0, c0,
            o1, h1, l1, c1,
            o2, h2, l2, c2,
            c3,  // next_close (from bar 3)
        )
    }

    /// Get current indicator value
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.current_value)
    }
    pub fn update_triplet_and_progress(
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
        next_close: f64,
    ) -> f64 {
        // detect
        let (bull, bear) = self
            .det
            .update_triplet(o0, h0, l0, c0, o1, h1, l1, c1, o2, h2, l2, c2);
        if bull {
            // gap between h0 and l1
            let upper = l1;
            let lower = h0.max(h2);
            self.active.push(PendingGap {
                upper,
                lower,
                remain: self.horizon,
            });
            self.total += 1;
        } else if bear {
            // gap between h1 and l0 (approx)
            let lower = h1;
            let upper = l0.min(h2);
            self.active.push(PendingGap {
                upper,
                lower,
                remain: self.horizon,
            });
            self.total += 1;
        }
        // progress one bar with next_close to check fill
        let mut still: Vec<PendingGap> = Vec::with_capacity(self.active.len());
        for mut g in self.active.drain(..) {
            if next_close <= g.upper && next_close >= g.lower {
                self.hits += 1;
            } else if g.remain > 1 {
                g.remain -= 1;
                still.push(g);
            }
        }
        self.active = still;
        self.current_value = if self.total > 0 {
            (self.hits as f64) / (self.total as f64)
        } else {
            0.0
        };
        self.current_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fvg_reversion_probability_creation() {
        let frp = FvgReversionProbability::new(10);
        assert!(!frp.is_ready()); // needs 10 FVGs
        assert_eq!(frp.current_value, 0.0);
    }

    #[test]
    fn test_fvg_reversion_probability_update() {
        let mut frp = FvgReversionProbability::new(5);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            // Create a bullish FVG scenario
            frp.update_triplet_and_progress(
                price, price + 1.0, price - 1.0, price,
                price + 5.0, price + 6.0, price + 4.0, price + 5.5,
                price + 2.0, price + 3.0, price + 1.0, price + 2.5,
                price + 4.5, // next close
            );
        }
        assert!(frp.current_value >= 0.0 && frp.current_value <= 1.0, "Probability should be in [0, 1]");
    }

    #[test]
    fn test_fvg_reversion_probability_reset() {
        let mut frp = FvgReversionProbability::new(5);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            frp.update_triplet_and_progress(
                price, price + 1.0, price - 1.0, price,
                price + 5.0, price + 6.0, price + 4.0, price + 5.5,
                price + 2.0, price + 3.0, price + 1.0, price + 2.5,
                price + 4.5,
            );
        }
        frp.reset();
        assert!(!frp.is_ready());
        assert_eq!(frp.current_value, 0.0);
    }
}
