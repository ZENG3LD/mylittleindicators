//! PinRiskDetector — detects pin risk near a strike at expiration.
//!
//! A position is at pin risk when:
//! - `|delta| - delta_target| <= delta_tolerance` (delta near ±0.5, i.e. near strike)
//! - `|theta| >= theta_threshold` (significant time decay, near expiration)
//!
//! Output: `Signal(i8)`. `+1` = pin risk high, `0` = no pin risk.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::OptionGreeksConsumer;
use crate::core::types::OptionGreeks;

/// Detects options pin risk: position near strike price close to expiration.
///
/// Pin risk occurs when delta is close to ±0.5 (near-the-money) AND theta is
/// large in absolute value (close to expiry).
#[derive(Clone)]
pub struct PinRiskDetector {
    delta_target: f64,
    delta_tolerance: f64,
    theta_threshold: f64,
    last_signal: i8,
}

impl PinRiskDetector {
    /// Create a new indicator.
    ///
    /// - `delta_target`: target |delta| for pin risk (default 0.5).
    /// - `delta_tolerance`: allowed deviation from target (default 0.05).
    /// - `theta_threshold`: minimum `|theta|` to confirm near-expiry (default 0.5).
    pub fn new(delta_target: f64, delta_tolerance: f64, theta_threshold: f64) -> Self {
        Self {
            delta_target,
            delta_tolerance,
            theta_threshold,
            last_signal: 0,
        }
    }
}

impl Default for PinRiskDetector {
    fn default() -> Self {
        Self::new(0.5, 0.05, 0.5)
    }
}

impl OptionGreeksConsumer for PinRiskDetector {
    fn update_option_greeks(&mut self, g: &OptionGreeks) -> IndicatorValue {
        let delta_close = (g.delta.abs() - self.delta_target).abs() <= self.delta_tolerance;
        let theta_big = g.theta.abs() >= self.theta_threshold;
        self.last_signal = if delta_close && theta_big { 1 } else { 0 };
        IndicatorValue::Signal(self.last_signal)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    fn reset(&mut self) {
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_greeks(delta: f64, theta: f64) -> OptionGreeks {
        OptionGreeks {
            delta,
            gamma: 0.0,
            vega: 0.0,
            theta,
            rho: 0.0,
            mark_iv: 0.0,
            bid_iv: None,
            ask_iv: None,
            timestamp: 0,
        }
    }

    #[test]
    fn fires_when_near_strike_and_high_theta() {
        let mut ind = PinRiskDetector::default();
        // delta ≈ 0.5, |theta| > 0.5
        let v = ind.update_option_greeks(&make_greeks(0.48, -0.8));
        if let IndicatorValue::Signal(s) = v {
            assert_eq!(s, 1, "should detect pin risk");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn no_signal_when_delta_far_from_target() {
        let mut ind = PinRiskDetector::default();
        // delta = 0.1 — far from 0.5
        let v = ind.update_option_greeks(&make_greeks(0.1, -1.0));
        if let IndicatorValue::Signal(s) = v {
            assert_eq!(s, 0, "delta far from 0.5 should not trigger");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn no_signal_when_theta_low() {
        let mut ind = PinRiskDetector::default();
        // delta close to 0.5 but theta near zero
        let v = ind.update_option_greeks(&make_greeks(0.5, -0.1));
        if let IndicatorValue::Signal(s) = v {
            assert_eq!(s, 0, "low theta should not trigger");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn works_with_negative_delta() {
        let mut ind = PinRiskDetector::default();
        // delta = -0.5 (put near-the-money)
        let v = ind.update_option_greeks(&make_greeks(-0.5, -0.9));
        if let IndicatorValue::Signal(s) = v {
            assert_eq!(s, 1, "negative delta near -0.5 should also trigger");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn reset_clears_signal() {
        let mut ind = PinRiskDetector::default();
        ind.update_option_greeks(&make_greeks(0.5, -1.0));
        ind.reset();
        if let IndicatorValue::Signal(s) = ind.value() {
            assert_eq!(s, 0);
        }
    }
}
