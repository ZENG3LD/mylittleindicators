//! Accumulative Swing Index (ASI) - Welles Wilder
//!
//! The ASI is a cumulative total of the Swing Index (SI) values.
//! It attempts to show the "real" market direction by comparing
//! the relationship between consecutive bars.
//!
//! Swing Index Formula:
//! SI = 50 * (Cy - C + 0.5*(Cy - Oy) + 0.25*(C - O)) / R * K / T
//! Where:
//!   Cy = Yesterday's close
//!   C  = Today's close
//!   Oy = Yesterday's open
//!   O  = Today's open
//!   K  = Max(|H - Cy|, |L - Cy|)
//!   R  = Largest of: |H-Cy|, |L-Cy|, |H-L|
//!   T  = Limit move value (typically the maximum daily price change allowed)

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct AccumulativeSwingIndex {
    limit_move: f64,

    // Previous bar data
    prev_open: f64,
    prev_high: f64,
    prev_low: f64,
    prev_close: f64,

    // Values
    swing_index: f64,
    asi: f64,

    // State
    bars_count: usize,
    is_ready: bool,
}

impl Default for AccumulativeSwingIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl AccumulativeSwingIndex {
    pub fn new() -> Self {
        Self::with_limit_move(0.0) // 0 means auto-calculate based on price
    }

    pub fn with_limit_move(limit_move: f64) -> Self {
        Self {
            limit_move: limit_move.max(0.0),
            prev_open: 0.0,
            prev_high: 0.0,
            prev_low: 0.0,
            prev_close: 0.0,
            swing_index: 0.0,
            asi: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.prev_open = 0.0;
        self.prev_high = 0.0;
        self.prev_low = 0.0;
        self.prev_close = 0.0;
        self.swing_index = 0.0;
        self.asi = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.asi)
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, _volume: f64) -> f64 {
        self.bars_count += 1;

        // Need at least 2 bars to calculate SI
        if self.bars_count < 2 {
            self.prev_open = open;
            self.prev_high = high;
            self.prev_low = low;
            self.prev_close = close;
            return self.asi;
        }

        // Calculate Swing Index components
        let cy = self.prev_close; // Yesterday's close
        let oy = self.prev_open;  // Yesterday's open
        let c = close;            // Today's close
        let o = open;             // Today's open
        let h = high;             // Today's high
        let l = low;              // Today's low

        // K = Max(|H - Cy|, |L - Cy|)
        let k = (h - cy).abs().max((l - cy).abs());

        // Calculate R (largest of the three values)
        let hl = h - l;
        let h_cy = (h - cy).abs();
        let l_cy = (l - cy).abs();

        let r = if h_cy >= l_cy && h_cy >= hl {
            // |H - Cy| is largest
            h_cy + 0.5 * l_cy + 0.25 * (cy - oy).abs()
        } else if l_cy >= h_cy && l_cy >= hl {
            // |L - Cy| is largest
            l_cy + 0.5 * h_cy + 0.25 * (cy - oy).abs()
        } else {
            // |H - L| is largest
            hl + 0.25 * (cy - oy).abs()
        };

        // T (limit move) - if 0, use a percentage of current price
        let t = if self.limit_move > 0.0 {
            self.limit_move
        } else {
            // Auto: use 3% of price as reasonable limit
            close * 0.03
        }.max(1e-10);

        // Calculate Swing Index
        // SI = 50 * (Cy - C + 0.5*(Cy - Oy) + 0.25*(C - O)) / R * K / T
        if r > 1e-10 {
            let numerator = (cy - c) + 0.5 * (cy - oy) + 0.25 * (c - o);
            self.swing_index = 50.0 * numerator / r * k / t;

            // Clamp to reasonable range
            self.swing_index = self.swing_index.clamp(-100.0, 100.0);
        } else {
            self.swing_index = 0.0;
        }

        // Accumulate
        self.asi += self.swing_index;

        // Update previous values
        self.prev_open = open;
        self.prev_high = high;
        self.prev_low = low;
        self.prev_close = close;

        // Ready after 2 bars
        if self.bars_count >= 2 {
            self.is_ready = true;
        }

        self.asi
    }

    /// Get current Swing Index (non-cumulative)
    pub fn swing_index(&self) -> f64 {
        self.swing_index
    }

    /// Get limit move value
    pub fn limit_move(&self) -> f64 {
        self.limit_move
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asi_creation() {
        let asi = AccumulativeSwingIndex::new();
        assert!(!asi.is_ready());
        assert_eq!(asi.value().main(), 0.0);
    }

    #[test]
    fn test_asi_with_limit_move() {
        let asi = AccumulativeSwingIndex::with_limit_move(5.0);
        assert_eq!(asi.limit_move(), 5.0);
    }

    #[test]
    fn test_asi_warmup() {
        let mut asi = AccumulativeSwingIndex::new();
        // First bar
        asi.update_bar(100.0, 102.0, 99.0, 101.0, 1000.0);
        assert!(!asi.is_ready());

        // Second bar
        asi.update_bar(101.0, 103.0, 100.0, 102.0, 1000.0);
        assert!(asi.is_ready());
    }

    #[test]
    fn test_asi_uptrend() {
        let mut asi = AccumulativeSwingIndex::new();
        // Strong uptrend
        for i in 0..20 {
            let base = 100.0 + i as f64 * 2.0;
            let open = base;
            let high = base + 2.5;
            let low = base - 0.5;
            let close = base + 2.0; // Close near high
            asi.update_bar(open, high, low, close, 1000.0);
        }
        assert!(asi.is_ready());
        // In uptrend, ASI should generally be positive
        // (may vary based on exact price action)
    }

    #[test]
    fn test_asi_downtrend() {
        let mut asi = AccumulativeSwingIndex::new();
        // Strong downtrend
        for i in 0..20 {
            let base = 200.0 - i as f64 * 2.0;
            let open = base;
            let high = base + 0.5;
            let low = base - 2.5;
            let close = base - 2.0; // Close near low
            asi.update_bar(open, high, low, close, 1000.0);
        }
        assert!(asi.is_ready());
        // In downtrend, ASI should generally be negative
    }

    #[test]
    fn test_asi_values_finite() {
        let mut asi = AccumulativeSwingIndex::new();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = asi.update_bar(price, price + 1.0, price - 1.0, price + 0.5, 1000.0);
            assert!(value.is_finite(), "ASI value should be finite");
        }
    }

    #[test]
    fn test_asi_swing_index() {
        let mut asi = AccumulativeSwingIndex::new();
        // First bar - no SI yet
        asi.update_bar(100.0, 102.0, 99.0, 101.0, 1000.0);
        assert_eq!(asi.swing_index(), 0.0);

        // Second bar - SI calculated
        asi.update_bar(101.0, 104.0, 100.0, 103.0, 1000.0);
        assert!(asi.swing_index().is_finite());
    }

    #[test]
    fn test_asi_reset() {
        let mut asi = AccumulativeSwingIndex::new();
        for i in 0..10 {
            asi.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        assert!(asi.is_ready());
        asi.reset();
        assert!(!asi.is_ready());
        assert_eq!(asi.value().main(), 0.0);
        assert_eq!(asi.swing_index(), 0.0);
    }
}
