//! PriceVsIndexSpread — spread between last bar close and current index price.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::IndexPriceConsumer;
use crate::core::types::IndexPrice;

/// Tracks the spread between the last bar close price and the current index price.
///
/// Output: `Triple(price, index, spread)` where `spread = price - index`.
/// Returns `NAN` components until both price and index have been set.
#[derive(Clone)]
pub struct PriceVsIndexSpread {
    last_price: f64,
    last_index: f64,
}

impl PriceVsIndexSpread {
    /// Create a new indicator with no data.
    pub fn new() -> Self {
        Self {
            last_price: f64::NAN,
            last_index: f64::NAN,
        }
    }

    /// Update with a new OHLCV bar. Records close price.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> IndicatorValue {
        self.last_price = c;
        let spread = if self.last_price.is_finite() && self.last_index.is_finite() {
            self.last_price - self.last_index
        } else {
            f64::NAN
        };
        IndicatorValue::Triple(self.last_price, self.last_index, spread)
    }
}

impl Default for PriceVsIndexSpread {
    fn default() -> Self {
        Self::new()
    }
}

impl IndexPriceConsumer for PriceVsIndexSpread {
    fn update_index_price(&mut self, ip: &IndexPrice) -> IndicatorValue {
        self.last_index = ip.price;
        let spread = if self.last_price.is_finite() && self.last_index.is_finite() {
            self.last_price - self.last_index
        } else {
            f64::NAN
        };
        IndicatorValue::Triple(self.last_price, self.last_index, spread)
    }

    fn value(&self) -> IndicatorValue {
        let spread = if self.last_price.is_finite() && self.last_index.is_finite() {
            self.last_price - self.last_index
        } else {
            f64::NAN
        };
        IndicatorValue::Triple(self.last_price, self.last_index, spread)
    }

    fn reset(&mut self) {
        self.last_price = f64::NAN;
        self.last_index = f64::NAN;
    }

    fn is_ready(&self) -> bool {
        self.last_price.is_finite() && self.last_index.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ip(price: f64) -> IndexPrice {
        IndexPrice { price, timestamp: 0 }
    }

    #[test]
    fn spread_computed_after_both_updates() {
        let mut ind = PriceVsIndexSpread::new();
        ind.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        let v = ind.update_index_price(&make_ip(98.0));
        if let IndicatorValue::Triple(p, i, s) = v {
            assert!((p - 100.0).abs() < 1e-9);
            assert!((i - 98.0).abs() < 1e-9);
            assert!((s - 2.0).abs() < 1e-9);
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn not_ready_until_both_set() {
        let mut ind = PriceVsIndexSpread::new();
        assert!(!ind.is_ready());
        ind.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        assert!(!ind.is_ready());
        ind.update_index_price(&make_ip(98.0));
        assert!(ind.is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = PriceVsIndexSpread::new();
        ind.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        ind.update_index_price(&make_ip(98.0));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Triple(p, i, s) = ind.value() {
            assert!(p.is_nan());
            assert!(i.is_nan());
            assert!(s.is_nan());
        } else {
            panic!("expected Triple");
        }
    }
}
