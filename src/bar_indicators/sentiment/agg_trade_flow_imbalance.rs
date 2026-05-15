//! AggTradeFlowImbalance — rolling buy/sell volume imbalance over a time window.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::AggTradeConsumer;
use crate::core::types::AggTrade;

/// Computes `(buy_vol - sell_vol) / (buy_vol + sell_vol)` over a sliding time window.
///
/// Events older than `window_ms` milliseconds are dropped on each update.
///
/// Output: `IndicatorValue::Single(imbalance)` ∈ [-1, 1].
#[derive(Clone)]
pub struct AggTradeFlowImbalance {
    /// Window length in milliseconds.
    window_ms: i64,
    /// Buffered events: (timestamp_ms, quantity, is_buy).
    events: VecDeque<(i64, f64, bool)>,
    last_imbalance: f64,
}

impl AggTradeFlowImbalance {
    /// Create a new indicator.
    ///
    /// `window_ms` — rolling window in milliseconds. Default: 60 000 (1 minute).
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            events: VecDeque::new(),
            last_imbalance: 0.0,
        }
    }

    /// Passthrough for bar events — returns last imbalance unchanged.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        IndicatorValue::Single(self.last_imbalance)
    }
}

impl AggTradeConsumer for AggTradeFlowImbalance {
    fn update_agg_trade(&mut self, t: &AggTrade) -> IndicatorValue {
        self.events.push_back((t.timestamp, t.quantity, t.is_buy));

        // Evict stale events.
        while let Some(&(ts, _, _)) = self.events.front() {
            if t.timestamp - ts > self.window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }

        let (buy, sell) = self
            .events
            .iter()
            .fold((0.0f64, 0.0f64), |(b, s), &(_, q, is_buy)| {
                if is_buy {
                    (b + q, s)
                } else {
                    (b, s + q)
                }
            });

        let total = buy + sell;
        self.last_imbalance = if total > 0.0 {
            (buy - sell) / total
        } else {
            0.0
        };

        IndicatorValue::Single(self.last_imbalance)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_imbalance)
    }

    fn reset(&mut self) {
        self.events.clear();
        self.last_imbalance = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_trade(timestamp: i64, quantity: f64, is_buy: bool) -> AggTrade {
        AggTrade {
            aggregate_id: 0,
            price: 100.0,
            quantity,
            first_trade_id: 0,
            last_trade_id: 0,
            is_buy,
            timestamp,
        }
    }

    #[test]
    fn all_buy_gives_one() {
        let mut ind = AggTradeFlowImbalance::new(60_000);
        ind.update_agg_trade(&make_trade(1000, 5.0, true));
        ind.update_agg_trade(&make_trade(2000, 3.0, true));
        if let IndicatorValue::Single(v) = ind.value() {
            assert!((v - 1.0).abs() < 1e-9, "expected 1.0, got {v}");
        }
    }

    #[test]
    fn all_sell_gives_minus_one() {
        let mut ind = AggTradeFlowImbalance::new(60_000);
        ind.update_agg_trade(&make_trade(1000, 5.0, false));
        ind.update_agg_trade(&make_trade(2000, 3.0, false));
        if let IndicatorValue::Single(v) = ind.value() {
            assert!((v + 1.0).abs() < 1e-9, "expected -1.0, got {v}");
        }
    }

    #[test]
    fn equal_buy_sell_gives_zero() {
        let mut ind = AggTradeFlowImbalance::new(60_000);
        ind.update_agg_trade(&make_trade(1000, 4.0, true));
        ind.update_agg_trade(&make_trade(2000, 4.0, false));
        if let IndicatorValue::Single(v) = ind.value() {
            assert!(v.abs() < 1e-9, "expected 0.0, got {v}");
        }
    }

    #[test]
    fn old_events_evicted() {
        let window = 60_000i64;
        let mut ind = AggTradeFlowImbalance::new(window);
        // old sell trade
        ind.update_agg_trade(&make_trade(0, 100.0, false));
        // new buy trade far in the future — old one should be dropped
        ind.update_agg_trade(&make_trade(window + 1, 1.0, true));
        if let IndicatorValue::Single(v) = ind.value() {
            assert!((v - 1.0).abs() < 1e-9, "old sell should be evicted, got {v}");
        }
    }
}
