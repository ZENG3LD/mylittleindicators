//! AuctionPriceDeviation — percentage deviation of indicative auction price from last close.

use crate::bar_indicators::auction_event_consumer::AuctionEventConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::types::AuctionEvent;

/// Percentage deviation of the auction indicative price from the last bar close.
///
/// `deviation_pct = (indicative_price - last_close) / last_close * 100`
///
/// Returns 0.0 until both a bar and an auction event have been received.
///
/// Output: `Single(deviation_pct)`.
#[derive(Clone)]
pub struct AuctionPriceDeviation {
    last_close: f64,
    last_deviation: f64,
    has_close: bool,
}

impl AuctionPriceDeviation {
    /// Create a new indicator with no prior state.
    pub fn new() -> Self {
        Self {
            last_close: 0.0,
            last_deviation: 0.0,
            has_close: false,
        }
    }

    /// Update with OHLCV bar data — stores close price.
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> IndicatorValue {
        self.last_close = close;
        self.has_close = true;
        IndicatorValue::Single(self.last_deviation)
    }
}

impl Default for AuctionPriceDeviation {
    fn default() -> Self {
        Self::new()
    }
}

impl AuctionEventConsumer for AuctionPriceDeviation {
    fn update_auction(&mut self, a: &AuctionEvent) -> IndicatorValue {
        if self.has_close && self.last_close != 0.0 {
            self.last_deviation = (a.indicative_price - self.last_close) / self.last_close * 100.0;
        }
        IndicatorValue::Single(self.last_deviation)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_deviation)
    }

    fn reset(&mut self) {
        self.last_close = 0.0;
        self.last_deviation = 0.0;
        self.has_close = false;
    }

    fn is_ready(&self) -> bool {
        self.has_close
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_auction(indicative_price: f64) -> AuctionEvent {
        AuctionEvent {
            auction_id: "1".to_string(),
            indicative_price,
            indicative_qty: 100.0,
            state: "indicative".to_string(),
            timestamp: 0,
        }
    }

    #[test]
    fn deviation_computed_correctly() {
        let mut ind = AuctionPriceDeviation::new();
        ind.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        let val = ind.update_auction(&make_auction(105.0));
        if let IndicatorValue::Single(d) = val {
            assert!((d - 5.0).abs() < 1e-9, "deviation = {d}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn negative_deviation_when_below_close() {
        let mut ind = AuctionPriceDeviation::new();
        ind.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        let val = ind.update_auction(&make_auction(95.0));
        if let IndicatorValue::Single(d) = val {
            assert!(d < 0.0, "deviation should be negative, got {d}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn no_close_no_deviation() {
        let mut ind = AuctionPriceDeviation::new();
        let val = ind.update_auction(&make_auction(105.0));
        assert_eq!(val, IndicatorValue::Single(0.0));
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = AuctionPriceDeviation::new();
        ind.update_bar(0.0, 0.0, 0.0, 100.0, 0.0);
        ind.update_auction(&make_auction(110.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::Single(0.0));
    }
}
