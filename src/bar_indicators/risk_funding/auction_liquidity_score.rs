//! AuctionLiquidityScore — rolling ratio of current indicative qty to rolling mean.
//!
//! score = current_qty / rolling_mean_qty
//!
//! A score above 1 means more liquidity than usual at the auction.
//! A score below 1 means thinner liquidity.
//!
//! Output: `Single(score)`.

use std::collections::VecDeque;

use crate::bar_indicators::auction_event_consumer::AuctionEventConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::types::AuctionEvent;

/// Rolling liquidity score for auction events.
///
/// `score = current_indicative_qty / rolling_mean_qty`
///
/// Returns 1.0 (neutral) with a single event. Returns 0.0 before any event.
#[derive(Clone)]
pub struct AuctionLiquidityScore {
    window: usize,
    qty_history: VecDeque<f64>,
    last_score: f64,
}

impl AuctionLiquidityScore {
    /// Create a new indicator. `window` is clamped to at least 1.
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(1),
            qty_history: VecDeque::with_capacity(window),
            last_score: 0.0,
        }
    }

    fn rolling_mean(&self) -> f64 {
        if self.qty_history.is_empty() {
            return 0.0;
        }
        self.qty_history.iter().sum::<f64>() / self.qty_history.len() as f64
    }
}

impl Default for AuctionLiquidityScore {
    fn default() -> Self {
        Self::new(20)
    }
}

impl AuctionEventConsumer for AuctionLiquidityScore {
    fn update_auction(&mut self, a: &AuctionEvent) -> IndicatorValue {
        self.qty_history.push_back(a.indicative_qty);
        while self.qty_history.len() > self.window {
            self.qty_history.pop_front();
        }
        let mean = self.rolling_mean();
        self.last_score = if mean > 0.0 {
            a.indicative_qty / mean
        } else {
            1.0
        };
        IndicatorValue::Single(self.last_score)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_score)
    }

    fn reset(&mut self) {
        self.qty_history.clear();
        self.last_score = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.qty_history.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_auction(qty: f64) -> AuctionEvent {
        AuctionEvent {
            auction_id: "1".to_string(),
            indicative_price: 100.0,
            indicative_qty: qty,
            state: "indicative".to_string(),
            timestamp: 0,
        }
    }

    #[test]
    fn neutral_on_first_event() {
        let mut ind = AuctionLiquidityScore::new(5);
        let val = ind.update_auction(&make_auction(100.0));
        if let IndicatorValue::Single(v) = val {
            assert!((v - 1.0).abs() < 1e-9, "first event should be neutral (1.0), got {v}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn spike_gives_score_above_one() {
        let mut ind = AuctionLiquidityScore::new(5);
        for _ in 0..5 {
            ind.update_auction(&make_auction(100.0));
        }
        // Now spike to 300 — rolling mean still mostly 100
        let val = ind.update_auction(&make_auction(300.0));
        if let IndicatorValue::Single(v) = val {
            assert!(v > 1.0, "spike should give score > 1.0, got {v}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn low_qty_gives_score_below_one() {
        let mut ind = AuctionLiquidityScore::new(5);
        for _ in 0..5 {
            ind.update_auction(&make_auction(100.0));
        }
        let val = ind.update_auction(&make_auction(10.0));
        if let IndicatorValue::Single(v) = val {
            assert!(v < 1.0, "low qty should give score < 1.0, got {v}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = AuctionLiquidityScore::new(5);
        ind.update_auction(&make_auction(100.0));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::Single(0.0));
    }
}
