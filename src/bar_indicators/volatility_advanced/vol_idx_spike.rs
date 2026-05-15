//! VolIdxSpike — detects when volatility index exceeds 95th percentile of rolling history.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::VolatilityIndexConsumer;
use crate::core::types::VolatilityIndex;

/// Detects spikes in a volatility index by comparing to the 95th percentile of a rolling window.
///
/// If `current > p95(history)` → signal +1, otherwise 0.
///
/// Output: `Signal(i8)`.
#[derive(Clone)]
pub struct VolIdxSpike {
    period: usize,
    history: VecDeque<f64>,
    last_signal: i8,
}

impl VolIdxSpike {
    /// Create a new indicator. `period` is clamped to at least 3.
    pub fn new(period: usize) -> Self {
        let period = period.max(3);
        Self {
            period,
            history: VecDeque::with_capacity(period),
            last_signal: 0,
        }
    }

    fn compute_signal(&self, current: f64) -> i8 {
        let n = self.history.len();
        if n < 2 {
            return 0;
        }
        let mut sorted: Vec<f64> = self.history.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let p95_idx = ((n as f64 * 0.95) as usize).min(n.saturating_sub(1));
        let p95 = sorted[p95_idx];
        if current > p95 { 1 } else { 0 }
    }
}

impl Default for VolIdxSpike {
    fn default() -> Self {
        Self::new(20)
    }
}

impl VolatilityIndexConsumer for VolIdxSpike {
    fn update_volatility_index(&mut self, vi: &VolatilityIndex) -> IndicatorValue {
        let current = vi.value;
        // Compute signal against existing history before inserting new value
        self.last_signal = self.compute_signal(current);
        self.history.push_back(current);
        while self.history.len() > self.period {
            self.history.pop_front();
        }
        IndicatorValue::Signal(self.last_signal)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    fn reset(&mut self) {
        self.history.clear();
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_vi(v: f64) -> VolatilityIndex {
        VolatilityIndex { value: v, timestamp: 0 }
    }

    #[test]
    fn spike_above_p95() {
        let mut ind = VolIdxSpike::new(20);
        for i in 0..20 {
            ind.update_volatility_index(&make_vi(i as f64));
        }
        let v = ind.update_volatility_index(&make_vi(1000.0));
        if let IndicatorValue::Signal(s) = v {
            assert_eq!(s, 1, "should be +1 for spike above p95");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn no_spike_below_p95() {
        let mut ind = VolIdxSpike::new(20);
        for i in 0..20 {
            ind.update_volatility_index(&make_vi(i as f64));
        }
        // Median value should not trigger
        let v = ind.update_volatility_index(&make_vi(9.0));
        if let IndicatorValue::Signal(s) = v {
            assert_eq!(s, 0, "should be 0 for median value");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn reset_clears() {
        let mut ind = VolIdxSpike::new(5);
        for i in 0..5 {
            ind.update_volatility_index(&make_vi(i as f64));
        }
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Signal(s) = ind.value() {
            assert_eq!(s, 0);
        }
    }
}
