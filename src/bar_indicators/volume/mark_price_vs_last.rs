//! MarkPriceVsLast — deviation of mark price from the last traded price.
//!
//! Measures the premium or discount between the exchange mark price and
//! the last traded price. Useful for detecting funding arbitrage setups.
//!
//! Output: `Double(deviation, deviation_pct)`
//!   - deviation:      mark_price - last_traded_price (absolute)
//!   - deviation_pct:  deviation / last_traded_price * 100 (percentage)
//!
//! `is_ready` once at least one mark price and one traded price have been seen.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::mark_price_consumer::MarkPriceConsumer;
use crate::core::types::MarkPrice;

/// Tracks deviation between mark price and last traded price.
#[derive(Debug, Clone)]
pub struct MarkPriceVsLast {
    last_mark: f64,
    last_traded: f64,
    has_mark: bool,
    has_traded: bool,
}

impl MarkPriceVsLast {
    pub fn new() -> Self {
        Self {
            last_mark: 0.0,
            last_traded: 0.0,
            has_mark: false,
            has_traded: false,
        }
    }

    /// Update the last traded price (call with each new trade or bar close).
    pub fn set_last_traded(&mut self, price: f64) {
        if price > 0.0 {
            self.last_traded = price;
            self.has_traded = true;
        }
    }

    fn compute(&self) -> IndicatorValue {
        if !self.has_mark || !self.has_traded || self.last_traded < 1e-12 {
            return IndicatorValue::Double(0.0, 0.0);
        }
        let deviation = self.last_mark - self.last_traded;
        let deviation_pct = deviation / self.last_traded * 100.0;
        IndicatorValue::Double(deviation, deviation_pct)
    }
}

impl Default for MarkPriceVsLast {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkPriceConsumer for MarkPriceVsLast {
    fn update_mark(&mut self, mp: &MarkPrice) -> IndicatorValue {
        self.last_mark = mp.mark_price;
        self.has_mark = true;
        self.compute()
    }

    fn value(&self) -> IndicatorValue {
        self.compute()
    }

    fn reset(&mut self) {
        self.last_mark = 0.0;
        self.last_traded = 0.0;
        self.has_mark = false;
        self.has_traded = false;
    }

    fn is_ready(&self) -> bool {
        self.has_mark && self.has_traded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mp(mark: f64) -> MarkPrice {
        MarkPrice {
            symbol: "BTCUSDT".to_string(),
            mark_price: mark,
            index_price: None,
            funding_rate: None,
            timestamp: 0,
        }
    }

    #[test]
    fn not_ready_without_traded_price() {
        let mut ind = MarkPriceVsLast::new();
        ind.update_mark(&mp(50_000.0));
        assert!(!ind.is_ready(), "needs traded price too");
    }

    #[test]
    fn not_ready_without_mark_price() {
        let mut ind = MarkPriceVsLast::new();
        ind.set_last_traded(50_000.0);
        assert!(!ind.is_ready(), "needs mark price too");
    }

    #[test]
    fn computes_deviation() {
        let mut ind = MarkPriceVsLast::new();
        ind.set_last_traded(50_000.0);
        let v = ind.update_mark(&mp(50_100.0));
        assert!(ind.is_ready());
        match v {
            IndicatorValue::Double(dev, dev_pct) => {
                assert!((dev - 100.0).abs() < 1e-9);
                assert!((dev_pct - 0.2).abs() < 1e-6);
            }
            _ => panic!("expected Double"),
        }
    }

    #[test]
    fn negative_deviation_mark_below_last() {
        let mut ind = MarkPriceVsLast::new();
        ind.set_last_traded(50_000.0);
        let v = ind.update_mark(&mp(49_900.0));
        match v {
            IndicatorValue::Double(dev, _) => {
                assert!(dev < 0.0, "mark below last → negative deviation");
            }
            _ => panic!("expected Double"),
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = MarkPriceVsLast::new();
        ind.set_last_traded(50_000.0);
        ind.update_mark(&mp(50_100.0));
        assert!(ind.is_ready());
        ind.reset();
        assert!(!ind.is_ready());
    }
}
