//! SettlementVsMarkSpread — spread between settlement price and mark price.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::mark_price_consumer::MarkPriceConsumer;
use crate::bar_indicators::SettlementEventConsumer;
use crate::core::types::MarkPrice;
use crate::core::types::SettlementEvent;

/// Spread between the latest settlement price and the latest mark price.
///
/// spread = settlement_price − last_mark_price
///
/// Output: `Triple(settlement, last_mark, spread)`.
/// Returns `Triple(0, 0, 0)` until both streams have delivered at least one event.
#[derive(Clone)]
pub struct SettlementVsMarkSpread {
    last_settlement: f64,
    last_mark: f64,
    has_settlement: bool,
    has_mark: bool,
}

impl SettlementVsMarkSpread {
    /// Create a new indicator with no prior state.
    pub fn new() -> Self {
        Self {
            last_settlement: 0.0,
            last_mark: 0.0,
            has_settlement: false,
            has_mark: false,
        }
    }

    /// Compute the current indicator value.
    pub fn indicator_value(&self) -> IndicatorValue {
        let spread = self.last_settlement - self.last_mark;
        IndicatorValue::Triple(self.last_settlement, self.last_mark, spread)
    }

    /// Check readiness: both streams must have provided at least one event.
    pub fn indicator_is_ready(&self) -> bool {
        self.has_settlement && self.has_mark
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.last_settlement = 0.0;
        self.last_mark = 0.0;
        self.has_settlement = false;
        self.has_mark = false;
    }
}

impl Default for SettlementVsMarkSpread {
    fn default() -> Self {
        Self::new()
    }
}

impl SettlementEventConsumer for SettlementVsMarkSpread {
    fn update_settlement(&mut self, s: &SettlementEvent) -> IndicatorValue {
        self.last_settlement = s.settlement_price;
        self.has_settlement = true;
        self.indicator_value()
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
    }

    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl MarkPriceConsumer for SettlementVsMarkSpread {
    fn update_mark(&mut self, mp: &MarkPrice) -> IndicatorValue {
        self.last_mark = mp.mark_price;
        self.has_mark = true;
        self.indicator_value()
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
    }

    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_settlement(price: f64) -> SettlementEvent {
        SettlementEvent {
            settlement_price: price,
            settlement_time: 0,
            timestamp: 0,
        }
    }

    fn make_mark(price: f64) -> MarkPrice {
        MarkPrice {
            symbol: "BTC-USDT".to_string(),
            mark_price: price,
            index_price: None,
            funding_rate: None,
            timestamp: 0,
        }
    }

    #[test]
    fn spread_correct() {
        let mut ind = SettlementVsMarkSpread::new();
        ind.update_mark(&make_mark(29_000.0));
        ind.update_settlement(&make_settlement(30_000.0));
        if let IndicatorValue::Triple(s, m, spread) = ind.indicator_value() {
            assert_eq!(s, 30_000.0);
            assert_eq!(m, 29_000.0);
            assert!((spread - 1000.0).abs() < 1e-9, "spread = {spread}");
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn not_ready_before_both_streams() {
        let mut ind = SettlementVsMarkSpread::new();
        assert!(!ind.indicator_is_ready());
        ind.update_mark(&make_mark(29_000.0));
        assert!(!ind.indicator_is_ready());
        ind.update_settlement(&make_settlement(30_000.0));
        assert!(ind.indicator_is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = SettlementVsMarkSpread::new();
        ind.update_mark(&make_mark(100.0));
        ind.update_settlement(&make_settlement(110.0));
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        if let IndicatorValue::Triple(s, m, sp) = ind.indicator_value() {
            assert_eq!(s, 0.0);
            assert_eq!(m, 0.0);
            assert_eq!(sp, 0.0);
        }
    }
}
