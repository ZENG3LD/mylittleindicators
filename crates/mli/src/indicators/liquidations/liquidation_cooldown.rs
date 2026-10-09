//! Liquidation Cooldown — time elapsed since the last liquidation event.
//!
//! Measures "market cooling" between forced liquidations. A long cooldown
//! suggests low liquidation pressure; a short cooldown indicates sustained
//! cascade activity.
//!
//! # Algorithm
//!
//! - Records `last_ts` (timestamp of the most recent liquidation, ms).
//! - On each `update_liquidation`:
//!   - If `last_ts > 0`, computes `delta = (liq.timestamp - last_ts) / 1000.0`.
//!   - Updates `last_ts` to `liq.timestamp`.
//!   - Returns `Single(delta_seconds)`.
//! - Before the first pair of events the output is `Single(0.0)`.
//!
//! No external clock or `update_bar` is needed — cooldown is the inter-event
//! gap, not "time since last event in wall-clock time". This is consistent
//! with the rest of the liquidation indicator family and avoids any dependency
//! on bar timestamps.
//!
//! # Output
//! `Single(seconds_since_last_liquidation)`.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::liquidation_consumer::LiquidationConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::Liquidation;

/// Inter-event cooldown between consecutive liquidations (seconds).
#[derive(Clone, Debug)]
pub struct LiquidationCooldown {
    /// Timestamp of the most recent liquidation (ms). `None` until first event.
    last_ts: Option<i64>,
    /// Cached delta between the two most recent events (seconds).
    last_delta_sec: f64,
}

impl LiquidationCooldown {
    /// Create a new cooldown tracker.
    pub fn new() -> Self {
        Self { last_ts: None, last_delta_sec: 0.0 }
    }
}

impl Default for LiquidationCooldown {
    fn default() -> Self {
        Self::new()
    }
}

impl LiquidationConsumer for LiquidationCooldown {
    fn update_liquidation(&mut self, liq: &Liquidation) {
        if let Some(prev_ts) = self.last_ts {
            let delta_ms = liq.timestamp.saturating_sub(prev_ts);
            self.last_delta_sec = delta_ms as f64 / 1_000.0;
        }
        self.last_ts = Some(liq.timestamp);
    }


    fn reset(&mut self) {
        self.last_ts = None;
        self.last_delta_sec = 0.0;
    }

    fn is_ready(&self) -> bool {
        // Ready only after at least two events (so we have a real delta).
        self.last_delta_sec > 0.0
    }
}

/// Typed configuration for [`LiquidationCooldown`]. No parameters — purely event-driven.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct LiquidationCooldownConfig;

impl Indicator for LiquidationCooldown {
    const ID: IndicatorId = IndicatorId::LiquidationCooldown;
    const FAMILY: &'static [Family] = &[Family::Liquidations];
    const INPUT: &'static [StreamKind] = &[StreamKind::Liquidation];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::count(IndicatorOutputId::LiquidationCooldown)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = LiquidationCooldownConfig;
    type Runtime = LiquidationCooldown;

    fn create(_cfg: LiquidationCooldownConfig) -> LiquidationCooldown {
        LiquidationCooldown::new()
    }
}

impl crate::contract::Config for LiquidationCooldownConfig {
    fn defaults() -> Self {
        LiquidationCooldownConfig
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // Unit config — no Param fields. Auto is the full implementation.
        Self::machine_defaults_auto()
    }
}


impl Render for LiquidationCooldown {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::LiquidationCooldown, "Cooldown (s)", Color::hex(0x00BCD4))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::TradeSide;

    fn liq(ts: i64) -> Liquidation {
        Liquidation { symbol: String::new(), side: TradeSide::Buy, price: 30_000.0, quantity: 0.1, timestamp: ts, value: None, ..Default::default()}
    }

    #[test]
    fn zero_before_any_event() {
        let lc = LiquidationCooldown::new();
        assert_eq!(lc.value(), 0.0);
        assert!(!lc.is_ready());
    }

    #[test]
    fn zero_after_first_event() {
        let mut lc = LiquidationCooldown::new();
        lc.update_liquidation(&liq(1_000));
        // No previous timestamp — delta stays 0.
        assert_eq!(lc.value(), 0.0);
        assert!(!lc.is_ready());
    }

    #[test]
    fn cooldown_after_second_event() {
        let mut lc = LiquidationCooldown::new();
        lc.update_liquidation(&liq(0));
        // 5 seconds later.
        lc.update_liquidation(&liq(5_000));
        assert_eq!(lc.value(), 5.0);
        assert!(lc.is_ready());
    }

    #[test]
    fn successive_cooldowns() {
        let mut lc = LiquidationCooldown::new();
        lc.update_liquidation(&liq(0));
        lc.update_liquidation(&liq(2_000)); // 2 s
        lc.update_liquidation(&liq(7_000)); // 5 s
        assert_eq!(lc.value(), 5.0);
    }

    #[test]
    fn reset_clears_state() {
        let mut lc = LiquidationCooldown::new();
        lc.update_liquidation(&liq(0));
        lc.update_liquidation(&liq(3_000));
        lc.reset();
        assert_eq!(lc.value(), 0.0);
        assert!(!lc.is_ready());
    }

    #[test]
    fn no_underflow_on_equal_timestamps() {
        let mut lc = LiquidationCooldown::new();
        lc.update_liquidation(&liq(1_000));
        lc.update_liquidation(&liq(1_000));
        // Same timestamp → 0 seconds cooldown.
        assert_eq!(lc.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_liquidation_cooldown() {
        use crate::engine::contract_engine::IndicatorOrder;
        
        let mut f =
            IndicatorOrder::LiquidationCooldown(<<LiquidationCooldown as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
                .build_solo()
                .unwrap();
        f.feed(0, crate::contract::MarketSample::Liquidation(&liq(0)));
        f.feed(0, crate::contract::MarketSample::Liquidation(&liq(3_000)));
        // 3 000 ms gap → 3.0 s cooldown
        let delta = f.primary();
        assert!((delta - 3.0).abs() < 1e-9, "delta={delta}");
    }
}

impl LiquidationCooldown {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_delta_sec
    }
}
