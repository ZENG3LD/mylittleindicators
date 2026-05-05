//! Volume-Weighted Average Price (VWAP) indicator.

use std::collections::VecDeque;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Volume-Weighted Average Price (VWAP) - institutional benchmark price.
///
/// VWAP = Σ(TypicalPrice × Volume) / Σ(Volume)
///
/// where TypicalPrice = (High + Low + Close) / 3
///
/// Used by institutions to measure execution quality.
/// Price above VWAP = bullish, below = bearish.
///
/// # Implementation
///
/// Uses rolling window with O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Vwap {
    period: usize,
    price_volume_buffer: VecDeque<f64>,
    volume_buffer: VecDeque<f64>,
    cumulative_pv: f64,
    cumulative_vol: f64,
    value: f64,
}

impl Vwap {
    /// Returns `true` if the VWAP has received at least one bar with volume.
    pub fn is_ready(&self) -> bool {
        self.cumulative_vol > 0.0
    }

    /// Resets the VWAP to its initial state.
    pub fn reset(&mut self) {
        self.price_volume_buffer.clear();
        self.volume_buffer.clear();
        self.cumulative_pv = 0.0;
        self.cumulative_vol = 0.0;
        self.value = 0.0;
    }

    /// Returns the period of this VWAP.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Creates a new VWAP with the specified period.
    ///
    /// # Arguments
    /// * `period` - Number of bars in the rolling window
    pub fn new(period: usize) -> Self {
        let period = period.max(1);
        Self {
            period,
            price_volume_buffer: VecDeque::with_capacity(period),
            volume_buffer: VecDeque::with_capacity(period),
            cumulative_pv: 0.0,
            cumulative_vol: 0.0,
            value: 0.0,
        }
    }

    /// Updates the VWAP with a new bar and returns the current value.
    ///
    /// Uses typical price (H+L+C)/3 weighted by volume.
    pub fn update_bar(&mut self, _open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let typical = (high + low + close) / 3.0;
        let pv = typical * volume;

        self.price_volume_buffer.push_back(pv);
        self.volume_buffer.push_back(volume);
        self.cumulative_pv += pv;
        self.cumulative_vol += volume;

        if self.price_volume_buffer.len() > self.period {
            if let Some(old_pv) = self.price_volume_buffer.pop_front() {
                self.cumulative_pv -= old_pv;
            }
            if let Some(old_vol) = self.volume_buffer.pop_front() {
                self.cumulative_vol -= old_vol;
            }
        }

        if self.cumulative_vol > 0.0 {
            self.value = self.cumulative_pv / self.cumulative_vol;
        }
        self.value
    }

    /// Returns the current VWAP value as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vwap_basic_calculation() {
        let mut vwap = Vwap::new(3);

        // Bar 1: H=105, L=95, C=100, V=1000 -> TP=100
        vwap.update_bar(0.0, 105.0, 95.0, 100.0, 1000.0);
        // Bar 2: H=115, L=105, C=110, V=2000 -> TP=110
        vwap.update_bar(0.0, 115.0, 105.0, 110.0, 2000.0);
        // Bar 3: H=110, L=100, C=105, V=1000 -> TP=105
        let v3 = vwap.update_bar(0.0, 110.0, 100.0, 105.0, 1000.0);

        assert!(vwap.is_ready());
        // VWAP = (100*1000 + 110*2000 + 105*1000) / 4000 = 106.25
        assert!((v3 - 106.25).abs() < 0.01);
    }

    #[test]
    fn test_vwap_reset() {
        let mut vwap = Vwap::new(3);
        vwap.update_bar(0.0, 105.0, 95.0, 100.0, 1000.0);
        vwap.update_bar(0.0, 115.0, 105.0, 110.0, 1000.0);
        assert!(vwap.is_ready());

        vwap.reset();
        assert!(!vwap.is_ready());
    }
}
