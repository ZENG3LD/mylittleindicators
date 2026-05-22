//! FundingSettlementImpact — measures mark price change around funding settlements.
//!
//! Dual-stream: consumes both FundingSettlement and MarkPrice events.
//! Uses inherent methods (no dual-trait, both streams arrive via separate methods).

use std::collections::VecDeque;

use crate::bar_indicators::funding_settlement_consumer::FundingSettlementConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::mark_price_consumer::MarkPriceConsumer;
use crate::core::types::{FundingSettlement, MarkPrice};

/// Measures mark price impact of funding settlement events.
///
/// Maintains a circular buffer of (timestamp, mark_price) pairs.
/// When a FundingSettlement arrives, finds mark prices just before and just after
/// the settlement_time and computes:
///
/// `impact = (mark_after - mark_before) / mark_before`
///
/// Returns 0.0 when there is insufficient data.
///
/// Output: `Single(impact_pct)`.
#[derive(Clone)]
pub struct FundingSettlementImpact {
    /// Circular buffer of (timestamp_ms, mark_price).
    buffer: VecDeque<(i64, f64)>,
    buffer_size: usize,
    last_impact: f64,
    pending_settlement_time: Option<i64>,
}

impl FundingSettlementImpact {
    /// Create with a given buffer size (number of mark price snapshots to retain).
    ///
    /// - `buffer_size`: clamped to at least 4.
    pub fn new(buffer_size: usize) -> Self {
        let buffer_size = buffer_size.max(4);
        Self {
            buffer: VecDeque::with_capacity(buffer_size),
            buffer_size,
            last_impact: 0.0,
            pending_settlement_time: None,
        }
    }

    fn compute_impact(&self, settlement_time: i64) -> f64 {
        if self.buffer.len() < 2 {
            return 0.0;
        }
        // Find last mark price snapshot BEFORE settlement_time
        let before = self.buffer.iter()
            .rev()
            .find(|(ts, _)| *ts <= settlement_time)
            .map(|(_, price)| *price);
        // Find first mark price snapshot AFTER settlement_time
        let after = self.buffer.iter()
            .find(|(ts, _)| *ts > settlement_time)
            .map(|(_, price)| *price);

        match (before, after) {
            (Some(b), Some(a)) if b.abs() > 1e-15 => (a - b) / b,
            _ => 0.0,
        }
    }
}

impl Default for FundingSettlementImpact {
    fn default() -> Self {
        Self::new(64)
    }
}

impl FundingSettlementImpact {
    /// Current indicator value (inherent — avoids trait ambiguity).
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_impact)
    }

    /// Whether the indicator has enough data.
    pub fn is_ready(&self) -> bool {
        self.buffer.len() >= 2
    }

    /// Reset all internal state.
    pub fn do_reset(&mut self) {
        self.buffer.clear();
        self.last_impact = 0.0;
        self.pending_settlement_time = None;
    }

    /// Called by `update_bar` passthrough — returns current impact.
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, _close: f64, _volume: f64) -> IndicatorValue {
        self.value()
    }
}

impl MarkPriceConsumer for FundingSettlementImpact {
    fn update_mark(&mut self, mp: &MarkPrice) -> IndicatorValue {
        self.buffer.push_back((mp.timestamp, mp.mark_price));
        while self.buffer.len() > self.buffer_size {
            self.buffer.pop_front();
        }
        // If there's a pending settlement, try to compute now
        if let Some(settlement_time) = self.pending_settlement_time {
            let impact = self.compute_impact(settlement_time);
            if impact.abs() > 0.0 {
                self.last_impact = impact;
                self.pending_settlement_time = None;
            }
        }
        self.value()
    }

    fn value(&self) -> IndicatorValue {
        FundingSettlementImpact::value(self)
    }

    fn reset(&mut self) {
        self.do_reset();
    }

    fn is_ready(&self) -> bool {
        FundingSettlementImpact::is_ready(self)
    }
}

impl FundingSettlementConsumer for FundingSettlementImpact {
    fn update_funding_settlement(&mut self, fs: &FundingSettlement) -> IndicatorValue {
        let impact = self.compute_impact(fs.settlement_time);
        if impact.abs() > 0.0 {
            self.last_impact = impact;
            self.pending_settlement_time = None;
        } else {
            // Not enough data yet — remember settlement time for later
            self.pending_settlement_time = Some(fs.settlement_time);
        }
        self.value()
    }

    fn value(&self) -> IndicatorValue {
        FundingSettlementImpact::value(self)
    }

    fn reset(&mut self) {
        self.do_reset();
    }

    fn is_ready(&self) -> bool {
        FundingSettlementImpact::is_ready(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mark(ts: i64, price: f64) -> MarkPrice {
        MarkPrice {
            mark_price: price,
            index_price: None,
            funding_rate: None,
            timestamp: ts,
        }
    }

    fn settlement(settlement_time: i64) -> FundingSettlement {
        FundingSettlement {
            settled_rate: 0.0001,
            settlement_time,
            timestamp: settlement_time + 10,
        }
    }

    #[test]
    fn impact_computed_from_before_after_prices() {
        let mut ind = FundingSettlementImpact::new(20);
        // Before settlement at t=1000
        ind.update_mark(&mark(500, 100.0));
        ind.update_mark(&mark(900, 100.0));
        // Settlement happens at t=1000
        ind.update_funding_settlement(&settlement(1000));
        // After settlement
        ind.update_mark(&mark(1100, 102.0));
        if let IndicatorValue::Single(v) = ind.value() {
            // impact = (102 - 100) / 100 = 0.02
            let expected = 0.02;
            assert!((v - expected).abs() < 1e-9, "impact = {v}, expected {expected}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn returns_zero_without_enough_data() {
        let mut ind = FundingSettlementImpact::new(20);
        let val = ind.update_funding_settlement(&settlement(1000));
        if let IndicatorValue::Single(v) = val {
            assert_eq!(v, 0.0, "no data → 0.0");
        }
    }

    #[test]
    fn pending_settlement_resolved_after_mark_arrives() {
        let mut ind = FundingSettlementImpact::new(20);
        ind.update_mark(&mark(500, 50_000.0));
        // Settlement at t=1000, but no after-price yet
        ind.update_funding_settlement(&settlement(1000));
        // Now the after-price arrives
        ind.update_mark(&mark(1200, 51_000.0));
        if let IndicatorValue::Single(v) = ind.value() {
            let expected = (51_000.0 - 50_000.0) / 50_000.0;
            assert!((v - expected).abs() < 1e-9, "impact = {v}, expected {expected}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = FundingSettlementImpact::new(20);
        ind.update_mark(&mark(500, 100.0));
        ind.do_reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(v) = ind.value() {
            assert_eq!(v, 0.0);
        }
    }
}
