//! FundingSettlementImpact — measures mark price change around funding settlements.
//!
//! Dual-stream: consumes both FundingSettlement and MarkPrice events.
//! Uses inherent methods (no dual-trait, both streams arrive via separate methods).

use std::collections::VecDeque;

use crate::engine::streams::funding_settlement_consumer::FundingSettlementConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
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
#[derive(Debug, Clone)]
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
    /// Current indicator value (inherent — avoids UFCS ambiguity with dual-trait impls).
    pub fn indicator_value(&self) -> f64 {
        self.last_impact
    }

    /// Whether the indicator has enough data (inherent).
    pub fn indicator_is_ready(&self) -> bool {
        self.buffer.len() >= 2
    }

    /// Reset all internal state (inherent).
    pub fn indicator_reset(&mut self) {
        self.buffer.clear();
        self.last_impact = 0.0;
        self.pending_settlement_time = None;
    }
}

impl MarkPriceConsumer for FundingSettlementImpact {
    fn update_mark(&mut self, mp: &MarkPrice) {
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
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl FundingSettlementConsumer for FundingSettlementImpact {
    fn update_funding_settlement(&mut self, fs: &FundingSettlement) {
        let impact = self.compute_impact(fs.settlement_time);
        if impact.abs() > 0.0 {
            self.last_impact = impact;
            self.pending_settlement_time = None;
        } else {
            // Not enough data yet — remember settlement time for later
            self.pending_settlement_time = Some(fs.settlement_time);
        }
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

/// Typed configuration for [`FundingSettlementImpact`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FundingSettlementImpactConfig {
    /// Number of mark-price snapshots to retain in the ring buffer.
    pub buffer_size: Param<usize>,
}

impl Indicator for FundingSettlementImpact {
    const ID: IndicatorId = IndicatorId::FundingSettlementImpact;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::FundingSettlement, StreamKind::MarkPrice];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::FundingSettlementImpact)];
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::fixed(StoreKind::Deque, 64)],
    );
    type Config = FundingSettlementImpactConfig;
    type Runtime = FundingSettlementImpact;

    fn create(cfg: FundingSettlementImpactConfig) -> FundingSettlementImpact {
        FundingSettlementImpact::new(cfg.buffer_size.resolved())
    }
}

impl crate::contract::Config for FundingSettlementImpactConfig {
    fn defaults() -> Self {
        FundingSettlementImpactConfig { buffer_size: Param::Solo(64) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // buffer_size: Class B (event ring-buffer count, not a lookback period).
        // Auto would widen to range(2,4048,1) which is wrong for an event buffer.
        // Correct range: 1..=50 step 1 — covers practical mark-price snapshot counts.
        s.buffer_size = Param::range(1, 50, 1);
        s
    }
}


impl Render for FundingSettlementImpact {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::FundingSettlementImpact, "Settlement Impact", Color::hex(0x9C27B0))
            .precision(6)
            .build()
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
            ..Default::default()
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
        let v = ind.indicator_value();
        // impact = (102 - 100) / 100 = 0.02
        let expected = 0.02;
        assert!((v - expected).abs() < 1e-9, "impact = {v}, expected {expected}");
    }

    #[test]
    fn returns_zero_without_enough_data() {
        let mut ind = FundingSettlementImpact::new(20);
        ind.update_funding_settlement(&settlement(1000));
        assert_eq!(ind.indicator_value(), 0.0, "no data → 0.0");
    }

    #[test]
    fn pending_settlement_resolved_after_mark_arrives() {
        let mut ind = FundingSettlementImpact::new(20);
        ind.update_mark(&mark(500, 50_000.0));
        // Settlement at t=1000, but no after-price yet
        ind.update_funding_settlement(&settlement(1000));
        // Now the after-price arrives
        ind.update_mark(&mark(1200, 51_000.0));
        let v = ind.indicator_value();
        let expected = (51_000.0 - 50_000.0) / 50_000.0;
        assert!((v - expected).abs() < 1e-9, "impact = {v}, expected {expected}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = FundingSettlementImpact::new(20);
        ind.update_mark(&mark(500, 100.0));
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        let v = ind.indicator_value();
        assert_eq!(v, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_funding_settlement_impact() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        use crate::core::types::FundingSettlement;
        let mut f = IndicatorOrder::FundingSettlementImpact(FundingSettlementImpactConfig { buffer_size: Param::Solo(20) })
            .build_solo()
            .unwrap();
        // Feed mark prices before settlement
        let m1 = mark(500, 100.0);
        let m2 = mark(900, 100.0);
        f.feed(0, MarketSample::MarkPrice(&m1));
        f.feed(0, MarketSample::MarkPrice(&m2));
        // Feed settlement event
        let fs = FundingSettlement { settled_rate: 0.0001, settlement_time: 1000, timestamp: 1010 };
        f.feed(0, MarketSample::FundingSettlement(&fs));
        // Feed after-price
        let m3 = mark(1100, 102.0);
        f.feed(0, MarketSample::MarkPrice(&m3));
        let v = f.primary();
        let expected = 0.02_f64;
        assert!((v - expected).abs() < 1e-9, "impact={v}");
    }
}
