//! TickerSpreadRatio — normalized bid-ask spread relative to last price.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ticker_consumer::TickerConsumer;
use crate::core::types::Ticker;

/// Normalized bid-ask spread: `(ask - bid) / last_price`.
///
/// Returns 0.0 when bid_price or ask_price is None, or last_price is zero.
///
/// Output: `Single(spread_ratio)`.
#[derive(Clone, Default)]
pub struct TickerSpreadRatio {
    last_ratio: f64,
}

impl TickerSpreadRatio {
    /// Create a new indicator.
    pub fn new() -> Self {
        Self { last_ratio: 0.0 }
    }
}

impl TickerConsumer for TickerSpreadRatio {
    fn update_ticker(&mut self, ticker: &Ticker) -> IndicatorValue {
        self.last_ratio = match (ticker.bid_price, ticker.ask_price) {
            (Some(bid), Some(ask)) if ticker.last_price.abs() > 1e-15 => {
                (ask - bid) / ticker.last_price
            }
            _ => 0.0,
        };
        IndicatorValue::Single(self.last_ratio)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_ratio)
    }

    fn reset(&mut self) {
        self.last_ratio = 0.0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticker(bid: Option<f64>, ask: Option<f64>, last: f64) -> Ticker {
        Ticker {
            last_price: last,
            bid_price: bid,
            ask_price: ask,
            high_24h: None,
            low_24h: None,
            volume_24h: None,
            quote_volume_24h: None,
            price_change_24h: None,
            price_change_percent_24h: None,
            timestamp: 0,
        }
    }

    #[test]
    fn spread_ratio_computed_correctly() {
        let mut ind = TickerSpreadRatio::new();
        let val = ind.update_ticker(&ticker(Some(99.0), Some(101.0), 100.0));
        if let IndicatorValue::Single(r) = val {
            // (101 - 99) / 100 = 0.02
            assert!((r - 0.02).abs() < 1e-12, "spread ratio = {r}");
        }
    }

    #[test]
    fn missing_bid_returns_zero() {
        let mut ind = TickerSpreadRatio::new();
        let val = ind.update_ticker(&ticker(None, Some(101.0), 100.0));
        if let IndicatorValue::Single(r) = val {
            assert_eq!(r, 0.0);
        }
    }

    #[test]
    fn missing_ask_returns_zero() {
        let mut ind = TickerSpreadRatio::new();
        let val = ind.update_ticker(&ticker(Some(99.0), None, 100.0));
        if let IndicatorValue::Single(r) = val {
            assert_eq!(r, 0.0);
        }
    }

    #[test]
    fn zero_last_price_returns_zero() {
        let mut ind = TickerSpreadRatio::new();
        let val = ind.update_ticker(&ticker(Some(0.0), Some(0.0), 0.0));
        if let IndicatorValue::Single(r) = val {
            assert_eq!(r, 0.0);
        }
    }
}
