//! IvSkew — implied volatility skew from bid/ask IV spread.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::OptionGreeksConsumer;
use crate::core::types::OptionGreeks;

/// Computes the implied volatility skew as `bid_iv - ask_iv`.
///
/// Returns 0.0 when either `bid_iv` or `ask_iv` is `None`.
///
/// Output: `Single(skew)`.
#[derive(Clone)]
pub struct IvSkew {
    last_skew: f64,
}

impl IvSkew {
    /// Create a new indicator.
    pub fn new() -> Self {
        Self { last_skew: 0.0 }
    }
}

impl Default for IvSkew {
    fn default() -> Self {
        Self::new()
    }
}

impl OptionGreeksConsumer for IvSkew {
    fn update_option_greeks(&mut self, g: &OptionGreeks) -> IndicatorValue {
        self.last_skew = match (g.bid_iv, g.ask_iv) {
            (Some(bid), Some(ask)) => bid - ask,
            _ => 0.0,
        };
        IndicatorValue::Single(self.last_skew)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_skew)
    }

    fn reset(&mut self) {
        self.last_skew = 0.0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_greeks(bid_iv: Option<f64>, ask_iv: Option<f64>) -> OptionGreeks {
        OptionGreeks {
            delta: 0.0,
            gamma: 0.0,
            vega: 0.0,
            theta: 0.0,
            rho: 0.0,
            mark_iv: 0.5,
            bid_iv,
            ask_iv,
            timestamp: 0,
        }
    }

    #[test]
    fn positive_skew_when_bid_above_ask() {
        let mut ind = IvSkew::new();
        let v = ind.update_option_greeks(&make_greeks(Some(0.8), Some(0.6)));
        if let IndicatorValue::Single(s) = v {
            assert!((s - 0.2).abs() < 1e-9, "skew should be 0.2, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn negative_skew_when_ask_above_bid() {
        let mut ind = IvSkew::new();
        let v = ind.update_option_greeks(&make_greeks(Some(0.5), Some(0.7)));
        if let IndicatorValue::Single(s) = v {
            assert!((s - (-0.2)).abs() < 1e-9, "skew should be -0.2, got {s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn zero_when_iv_missing() {
        let mut ind = IvSkew::new();
        let v = ind.update_option_greeks(&make_greeks(None, None));
        if let IndicatorValue::Single(s) = v {
            assert_eq!(s, 0.0);
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears() {
        let mut ind = IvSkew::new();
        ind.update_option_greeks(&make_greeks(Some(0.8), Some(0.6)));
        ind.reset();
        if let IndicatorValue::Single(s) = ind.value() {
            assert_eq!(s, 0.0);
        }
    }
}
