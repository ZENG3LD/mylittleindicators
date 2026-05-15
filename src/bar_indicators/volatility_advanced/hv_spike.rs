//! HvSpike — detects when HV exceeds N times its rolling mean.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::HistoricalVolatilityConsumer;
use crate::core::types::HistoricalVolatility;

/// Detects spikes in historical volatility by comparing current value to a rolling mean.
///
/// If `current_hv > multiplier * rolling_mean` → signal +1, otherwise 0.
///
/// Output: `Signal(i8)`.
#[derive(Clone)]
pub struct HvSpike {
    period: usize,
    multiplier: f64,
    history: VecDeque<f64>,
    last_signal: i8,
}

impl HvSpike {
    /// Create a new indicator.
    /// - `period`: rolling window for mean computation (clamped to at least 2)
    /// - `multiplier`: spike threshold factor (default 2.0)
    pub fn new(period: usize, multiplier: f64) -> Self {
        let period = period.max(2);
        Self {
            period,
            multiplier,
            history: VecDeque::with_capacity(period),
            last_signal: 0,
        }
    }

    fn compute_signal(&self, current: f64) -> i8 {
        let n = self.history.len();
        if n < 2 {
            return 0;
        }
        let mean = self.history.iter().sum::<f64>() / n as f64;
        if mean > 1e-12 && current > self.multiplier * mean {
            1
        } else {
            0
        }
    }
}

impl Default for HvSpike {
    fn default() -> Self {
        Self::new(20, 2.0)
    }
}

impl HistoricalVolatilityConsumer for HvSpike {
    fn update_historical_volatility(&mut self, hv: &HistoricalVolatility) -> IndicatorValue {
        let current = hv.volatility;
        self.history.push_back(current);
        while self.history.len() > self.period {
            self.history.pop_front();
        }
        self.last_signal = self.compute_signal(current);
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

    fn make_hv(v: f64) -> HistoricalVolatility {
        HistoricalVolatility { volatility: v, timestamp: 0 }
    }

    #[test]
    fn spike_detected_above_threshold() {
        let mut ind = HvSpike::new(5, 2.0);
        // Fill with baseline around 0.1
        for _ in 0..5 {
            ind.update_historical_volatility(&make_hv(0.1));
        }
        // Push spike: 0.1 * 2.0 = 0.2 threshold; current = 0.5 > 0.2
        let v = ind.update_historical_volatility(&make_hv(0.5));
        if let IndicatorValue::Signal(s) = v {
            assert_eq!(s, 1, "should detect spike");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn no_spike_below_threshold() {
        let mut ind = HvSpike::new(5, 2.0);
        for _ in 0..5 {
            ind.update_historical_volatility(&make_hv(0.1));
        }
        let v = ind.update_historical_volatility(&make_hv(0.15));
        if let IndicatorValue::Signal(s) = v {
            assert_eq!(s, 0, "should not detect spike");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn reset_clears() {
        let mut ind = HvSpike::new(5, 2.0);
        for _ in 0..5 {
            ind.update_historical_volatility(&make_hv(0.1));
        }
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Signal(s) = ind.value() {
            assert_eq!(s, 0);
        }
    }
}
