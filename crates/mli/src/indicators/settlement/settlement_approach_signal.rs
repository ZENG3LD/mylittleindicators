//! SettlementApproachSignal — pressure score approaching contract settlement.
//!
//! Contract-driven via the `Settlement` stream: each `update_settlement` event carries
//! `settlement_time` + `timestamp`, from which the approach score is computed. INPUT =
//! Settlement; time travels INSIDE the event (no separate ts axis).
//!
//! Output: `Single(approach_score)` ∈ [0, 1].
//! - `0.0` = far from settlement (or no settlement known)
//! - `1.0` = at or past settlement time

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::settlement_event_consumer::SettlementEventConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Render, SourceAxis, RenderSpec, Color, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::SettlementEvent;

/// Score that grows from 0 → 1 as the next settlement time approaches, computed from the
/// `SettlementEvent`'s `settlement_time` vs its `timestamp`.
#[derive(Debug, Clone)]
pub struct SettlementApproachSignal {
    /// Lookahead window (ms): score is 0 when `countdown >= max_window_ms`, 1 at countdown 0.
    max_window_ms: i64,
    last_score: f64,
}

impl SettlementApproachSignal {
    /// Create a new indicator. `max_window_ms`: score is 0 when `countdown >= max_window_ms`.
    pub fn new(max_window_ms: i64) -> Self {
        Self { max_window_ms: max_window_ms.max(1), last_score: 0.0 }
    }
}

impl Default for SettlementApproachSignal {
    fn default() -> Self {
        // 8 hours default window
        Self::new(8 * 3600 * 1000)
    }
}

impl SettlementEventConsumer for SettlementApproachSignal {
    fn update_settlement(&mut self, s: &SettlementEvent) {
        // Time travels inside the event: countdown = settlement_time − event timestamp.
        if s.timestamp > 0 {
            let countdown = (s.settlement_time - s.timestamp).max(0);
            let normalized = countdown as f64 / self.max_window_ms as f64;
            self.last_score = 1.0 - normalized.clamp(0.0, 1.0);
        }
    }


    fn reset(&mut self) {
        self.last_score = 0.0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

use crate::contract::Param;

/// Typed configuration for [`SettlementApproachSignal`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SettlementApproachSignalConfig {
    /// Countdown window in milliseconds. Score is 0 when `countdown >= max_window_ms`.
    pub max_window_ms: Param<i64>,
}

impl Indicator for SettlementApproachSignal {
    const ID: IndicatorId = IndicatorId::SettlementApproachSignal;
    /// Settlement-event approach score — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Settlement];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::SettlementApproachSignal)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = SettlementApproachSignalConfig;
    type Runtime = Self;

    fn create(cfg: SettlementApproachSignalConfig) -> Self {
        Self::new(cfg.max_window_ms.resolved())
    }
}

impl crate::contract::Config for SettlementApproachSignalConfig {
    fn defaults() -> Self {
        SettlementApproachSignalConfig { max_window_ms: Param::Solo(8 * 3600 * 1000) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // max_window_ms: Class M (settlement window horizon, i64 ms).
        // Explicit 5-value set: 1h / 2h / 4h / 8h (Binance default) / 24h.
        s.max_window_ms = Param::many(vec![
            3_600_000i64,   // 1h
            7_200_000,      // 2h
            14_400_000,     // 4h
            28_800_000,     // 8h
            86_400_000,     // 24h
        ]);
        s
    }
}


impl Render for SettlementApproachSignal {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::SettlementApproachSignal, "Settlement Approach", Color::hex(0xFF9800))
            .precision(3)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_settlement(settlement_time: i64, ts: i64) -> SettlementEvent {
        SettlementEvent { settlement_price: 50000.0, settlement_time, timestamp: ts, ..Default::default()}
    }

    #[test]
    fn far_from_settlement_low_score() {
        let mut ind = SettlementApproachSignal::new(8 * 3600 * 1000);
        // event ts a full window before settlement → countdown ≈ window → score ≈ 0
        ind.update_settlement(&make_settlement(8 * 3600 * 1000, 1));
        let score = ind.value();
        assert!(score < 0.1, "score should be low far from settlement, got {score}");
    }

    #[test]
    fn near_settlement_high_score() {
        let mut ind = SettlementApproachSignal::new(8 * 3600 * 1000);
        // event ts just before settlement → countdown tiny → score ≈ 1
        ind.update_settlement(&make_settlement(1_000, 990));
        let score = ind.value();
        assert!(score > 0.9, "score should be high near settlement, got {score}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = SettlementApproachSignal::new(8 * 3600 * 1000);
        ind.update_settlement(&make_settlement(1_000_000, 1));
        ind.reset();
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_settlement() {
        use crate::engine::contract_engine::IndicatorOrder;
        let mut f = IndicatorOrder::SettlementApproachSignal(
            <<SettlementApproachSignal as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let ev = make_settlement(1_000_000, 0);
        f.feed(0, crate::contract::MarketSample::Settlement(&ev));
        let _ = f.primary();
    }
}

impl SettlementApproachSignal {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_score
    }
}
