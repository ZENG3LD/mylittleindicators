//! Volume-Weighted Moving Average (VWMA) indicator.

use std::collections::VecDeque;
use crate::bar_indicators::indicator_value::IndicatorValue;

/// Volume-Weighted Moving Average (VWMA) - weights prices by their volume.
///
/// VWMA = Σ(Price × Volume) / Σ(Volume)
///
/// Gives more weight to prices traded with higher volume.
/// Useful for identifying true support/resistance levels.
///
/// # Implementation
///
/// Uses two ring buffers for O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Vwma {
    period: usize,
    pv_buf: VecDeque<f64>,
    v_buf: VecDeque<f64>,
    sum_pv: f64,
    sum_v: f64,
    value: f64,
}

impl Vwma {
    /// Creates a new VWMA with the specified period.
    ///
    /// # Arguments
    /// * `period` - Number of bars to include in the calculation
    pub fn new(period: usize) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            pv_buf: VecDeque::with_capacity(p),
            v_buf: VecDeque::with_capacity(p),
            sum_pv: 0.0,
            sum_v: 0.0,
            value: 0.0,
        }
    }

    /// Updates the VWMA with a new bar and returns the current value.
    ///
    /// Uses `close` price weighted by `volume`.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, v: f64) -> f64 {
        let pv = c * v;
        self.pv_buf.push_back(pv);
        self.v_buf.push_back(v);
        self.sum_pv += pv;
        self.sum_v += v;
        if self.pv_buf.len() > self.period {
            if let Some(x) = self.pv_buf.pop_front() {
                self.sum_pv -= x;
            }
            if let Some(x) = self.v_buf.pop_front() {
                self.sum_v -= x;
            }
        }
        if self.sum_v > 0.0 {
            self.value = self.sum_pv / self.sum_v;
        }
        self.value
    }

    /// Returns the current VWMA value as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the VWMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.pv_buf.len() == self.period && self.sum_v > 0.0
    }

    /// Resets the VWMA to its initial state.
    pub fn reset(&mut self) {
        self.pv_buf.clear();
        self.v_buf.clear();
        self.sum_pv = 0.0;
        self.sum_v = 0.0;
        self.value = 0.0;
    }

    /// Returns the period of this VWMA.
    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vwma_basic_calculation() {
        let mut vwma = Vwma::new(3);

        // Price 100, Volume 1000
        vwma.update_bar(0.0, 0.0, 0.0, 100.0, 1000.0);
        // Price 110, Volume 2000 (more volume = more weight)
        vwma.update_bar(0.0, 0.0, 0.0, 110.0, 2000.0);
        // Price 105, Volume 1000
        let v3 = vwma.update_bar(0.0, 0.0, 0.0, 105.0, 1000.0);

        assert!(vwma.is_ready());
        // VWMA = (100*1000 + 110*2000 + 105*1000) / (1000 + 2000 + 1000)
        //      = (100000 + 220000 + 105000) / 4000 = 425000 / 4000 = 106.25
        assert!((v3 - 106.25).abs() < 0.01);
    }

    #[test]
    fn test_vwma_zero_volume() {
        let mut vwma = Vwma::new(2);

        vwma.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        vwma.update_bar(0.0, 0.0, 0.0, 110.0, 0.0);

        // With zero volume, is_ready should be false
        assert!(!vwma.is_ready());
    }

    #[test]
    fn test_vwma_reset() {
        let mut vwma = Vwma::new(3);
        vwma.update_bar(0.0, 0.0, 0.0, 100.0, 1000.0);
        vwma.update_bar(0.0, 0.0, 0.0, 110.0, 1000.0);
        vwma.update_bar(0.0, 0.0, 0.0, 120.0, 1000.0);
        assert!(vwma.is_ready());

        vwma.reset();
        assert!(!vwma.is_ready());
    }
}
