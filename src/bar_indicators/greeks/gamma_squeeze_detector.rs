//! GammaSqueezeDetector — detects potential gamma squeeze conditions.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::OptionGreeksConsumer;
use crate::core::types::OptionGreeks;

/// Detects potential gamma squeeze: high gamma + significant price movement.
///
/// Fires +1 when:
/// - `gamma > gamma_threshold` AND
/// - `|last_price - prev_price| > price_move_threshold`
///
/// Also accepts OHLCV bar updates to track price movement.
///
/// Output: `Signal(i8)`.
#[derive(Clone)]
pub struct GammaSqueezeDetector {
    gamma_threshold: f64,
    price_move_threshold: f64,
    last_gamma: f64,
    prev_price: f64,
    last_price: f64,
    last_signal: i8,
}

impl GammaSqueezeDetector {
    /// Create a new indicator.
    /// - `gamma_threshold`: minimum gamma value to consider (default 0.01)
    /// - `price_move_threshold`: minimum absolute price move to trigger (default 1.0)
    pub fn new(gamma_threshold: f64, price_move_threshold: f64) -> Self {
        Self {
            gamma_threshold,
            price_move_threshold,
            last_gamma: 0.0,
            prev_price: f64::NAN,
            last_price: f64::NAN,
            last_signal: 0,
        }
    }

    /// Update with a new OHLCV bar. Records close price and checks squeeze condition.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> IndicatorValue {
        self.prev_price = self.last_price;
        self.last_price = c;
        self.last_signal = self.evaluate_signal();
        IndicatorValue::Signal(self.last_signal)
    }

    fn evaluate_signal(&self) -> i8 {
        let price_moved = if self.prev_price.is_finite() && self.last_price.is_finite() {
            (self.last_price - self.prev_price).abs() > self.price_move_threshold
        } else {
            false
        };
        if self.last_gamma > self.gamma_threshold && price_moved {
            1
        } else {
            0
        }
    }
}

impl Default for GammaSqueezeDetector {
    fn default() -> Self {
        Self::new(0.01, 1.0)
    }
}

impl OptionGreeksConsumer for GammaSqueezeDetector {
    fn update_option_greeks(&mut self, g: &OptionGreeks) -> IndicatorValue {
        self.last_gamma = g.gamma;
        self.last_signal = self.evaluate_signal();
        IndicatorValue::Signal(self.last_signal)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    fn reset(&mut self) {
        self.last_gamma = 0.0;
        self.prev_price = f64::NAN;
        self.last_price = f64::NAN;
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        self.last_gamma > 0.0 && self.last_price.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_greeks(gamma: f64) -> OptionGreeks {
        OptionGreeks {
            delta: 0.0,
            gamma,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
            mark_iv: 0.0,
            bid_iv: None,
            ask_iv: None,
            timestamp: 0,
        }
    }

    #[test]
    fn squeeze_detected_with_high_gamma_and_price_move() {
        let mut ind = GammaSqueezeDetector::new(0.01, 1.0);
        // Set prev price and last price with significant move
        ind.update_bar(0.0, 0.0, 0.0, 100.0, 0.0); // last_price = 100, prev = NAN
        ind.update_bar(0.0, 0.0, 0.0, 103.0, 0.0); // price_move = 3 > 1
        // Set high gamma
        let v = ind.update_option_greeks(&make_greeks(0.05));
        if let IndicatorValue::Signal(s) = v {
            assert_eq!(s, 1, "should detect squeeze");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn no_squeeze_with_low_gamma() {
        let mut ind = GammaSqueezeDetector::new(0.01, 1.0);
        ind.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        ind.update_bar(0.0, 0.0, 0.0, 103.0, 0.0);
        let v = ind.update_option_greeks(&make_greeks(0.001)); // below threshold
        if let IndicatorValue::Signal(s) = v {
            assert_eq!(s, 0, "should not detect squeeze with low gamma");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn no_squeeze_with_small_price_move() {
        let mut ind = GammaSqueezeDetector::new(0.01, 1.0);
        ind.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        ind.update_bar(0.0, 0.0, 0.0, 100.5, 0.0); // move = 0.5 < 1.0
        let v = ind.update_option_greeks(&make_greeks(0.05));
        if let IndicatorValue::Signal(s) = v {
            assert_eq!(s, 0, "should not detect squeeze with small price move");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn reset_clears() {
        let mut ind = GammaSqueezeDetector::new(0.01, 1.0);
        ind.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        ind.update_bar(0.0, 0.0, 0.0, 105.0, 0.0);
        ind.update_option_greeks(&make_greeks(0.05));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Signal(s) = ind.value() {
            assert_eq!(s, 0);
        }
    }
}
