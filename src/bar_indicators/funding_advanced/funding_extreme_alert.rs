//! FundingExtremeAlert — flags extreme funding rate deviations.
//!
//! Computes rolling mean + std over `window` snapshots.
//! When `|current - mean| > sigma_threshold × std`:
//! - `signal = +1` if current > mean, `-1` if current < mean
//! - `magnitude = |current - mean| / std` (in sigmas)
//!
//! Output: `Double(signal_as_f64, magnitude_sigma)`.

use std::collections::VecDeque;

use crate::bar_indicators::funding_rate_consumer::FundingRateConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::types::FundingRate;

/// Detects statistically extreme funding rate events.
///
/// Fires a signal when `|rate - rolling_mean| > sigma_threshold × rolling_std`.
///
/// Output: `Double(signal_as_f64, magnitude_sigma)`.
/// `signal_as_f64 = +1.0 / -1.0 / 0.0`, `magnitude_sigma = deviation in sigmas`.
#[derive(Clone)]
pub struct FundingExtremeAlert {
    window: usize,
    sigma_threshold: f64,
    history: VecDeque<f64>,
    last_signal: f64,
    last_magnitude: f64,
}

impl FundingExtremeAlert {
    /// Create a new indicator.
    ///
    /// - `window`: rolling lookback length (min 2).
    /// - `sigma_threshold`: deviation threshold in standard deviations (default 2.0).
    pub fn new(window: usize, sigma_threshold: f64) -> Self {
        Self {
            window: window.max(2),
            sigma_threshold,
            history: VecDeque::new(),
            last_signal: 0.0,
            last_magnitude: 0.0,
        }
    }

    fn compute(&self, current: f64) -> (f64, f64) {
        let n = self.history.len();
        if n < 2 {
            return (0.0, 0.0);
        }
        let mean = self.history.iter().sum::<f64>() / n as f64;
        let variance = self.history.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        let std = variance.sqrt();
        if std < 1e-15 {
            return (0.0, 0.0);
        }
        let deviation = current - mean;
        let magnitude = deviation.abs() / std;
        if magnitude > self.sigma_threshold {
            let signal = if deviation > 0.0 { 1.0 } else { -1.0 };
            (signal, magnitude)
        } else {
            (0.0, magnitude)
        }
    }
}

impl Default for FundingExtremeAlert {
    fn default() -> Self {
        Self::new(20, 2.0)
    }
}

impl FundingRateConsumer for FundingExtremeAlert {
    fn update_funding(&mut self, fr: &FundingRate) -> IndicatorValue {
        self.history.push_back(fr.rate);
        if self.history.len() > self.window {
            self.history.pop_front();
        }
        let (signal, magnitude) = self.compute(fr.rate);
        self.last_signal = signal;
        self.last_magnitude = magnitude;
        IndicatorValue::Double(self.last_signal, self.last_magnitude)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.last_signal, self.last_magnitude)
    }

    fn reset(&mut self) {
        self.history.clear();
        self.last_signal = 0.0;
        self.last_magnitude = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.history.len() >= self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate {
            rate,
            next_funding_time: None,
            timestamp: 1000,
        }
    }

    #[test]
    fn no_extreme_for_normal_rate() {
        let mut ind = FundingExtremeAlert::new(10, 2.0);
        for _ in 0..10 {
            ind.update_funding(&make_fr(0.0001));
        }
        // Constant series — std≈0, no extreme
        if let IndicatorValue::Double(sig, _) = ind.value() {
            assert_eq!(sig, 0.0);
        } else {
            panic!("expected Double");
        }
    }

    #[test]
    fn extreme_positive_fires_plus_one() {
        let mut ind = FundingExtremeAlert::new(10, 2.0);
        // Fill with small rates
        for _ in 0..9 {
            ind.update_funding(&make_fr(0.0001));
        }
        // Push a very large rate to trigger alert
        let v = ind.update_funding(&make_fr(0.1));
        if let IndicatorValue::Double(sig, mag) = v {
            assert_eq!(sig, 1.0, "extreme positive should give +1");
            assert!(mag > 2.0, "magnitude should exceed threshold");
        } else {
            panic!("expected Double");
        }
    }

    #[test]
    fn extreme_negative_fires_minus_one() {
        let mut ind = FundingExtremeAlert::new(10, 2.0);
        for _ in 0..9 {
            ind.update_funding(&make_fr(0.0001));
        }
        let v = ind.update_funding(&make_fr(-0.1));
        if let IndicatorValue::Double(sig, _) = v {
            assert_eq!(sig, -1.0, "extreme negative should give -1");
        } else {
            panic!("expected Double");
        }
    }

    #[test]
    fn not_ready_until_window_full() {
        let mut ind = FundingExtremeAlert::new(5, 2.0);
        for i in 0..4 {
            ind.update_funding(&make_fr(i as f64 * 0.0001));
        }
        assert!(!ind.is_ready());
        ind.update_funding(&make_fr(0.0005));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = FundingExtremeAlert::new(5, 2.0);
        for _ in 0..5 {
            ind.update_funding(&make_fr(0.0001));
        }
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Double(sig, mag) = ind.value() {
            assert_eq!(sig, 0.0);
            assert_eq!(mag, 0.0);
        }
    }
}
