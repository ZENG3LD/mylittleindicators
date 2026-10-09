//! Month-turn effect — proximity score to start or end of month.

use crate::core::types::CalendarService;

/// Emits a [0, 1] score: 1.0 at the very start or end of the month, decaying
/// linearly to 0.0 at `window_days` distance from either boundary.
///
/// Uses `CalendarService::days_in_month` — the local `days_in_month` duplicate
/// that was in the original file has been removed.
/// Price-free: the wall-clock coordinate only, no kline source.
#[derive(Debug, Clone)]
pub struct MonthTurnEffect {
    window: u32,
    value: f64,
}

impl MonthTurnEffect {
    pub fn new(window_days: u32) -> Self {
        Self {
            window: window_days.clamp(1, 10),
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }

    pub fn value(&self) -> f64 {
        self.value
    }

    #[inline]
    pub fn turn(&self) -> f64 {
        self.value
    }

    /// Feed the bar's wall-clock (canonical MILLISECONDS); price-free (pure-time core).
    pub fn feed(&mut self, ts_ms: i64) -> f64 {
        let ts_secs = ts_ms.div_euclid(1000);
        let (year, month, day) = CalendarService::ymd_from_timestamp(ts_secs);
        let days_in_month = CalendarService::days_in_month(year, month);
        let dist_start = (day as i32 - 1).unsigned_abs();
        let dist_end = (days_in_month as i32 - day as i32).unsigned_abs();
        let near = dist_start.min(dist_end);
        self.value = if near <= self.window {
            1.0 - (near as f64 / self.window as f64)
        } else {
            0.0
        };
        self.value
    }
}

impl Default for MonthTurnEffect {
    /// Factory default: window_days = 5.
    fn default() -> Self {
        Self::new(5)
    }
}

// ── contract ──────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, RenderSpec, SourceAxis, UpdateComplexity,
};
use crate::contract::Render;
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`MonthTurnEffect`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct MonthTurnEffectConfig {
    pub window_days: Param<u32>,
}

impl Indicator for MonthTurnEffect {
    const ID: IndicatorId = IndicatorId::MonthTurn;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Pure-time core (the `Time` flavor): no kline source — only the wall-clock coordinate.
    const SOURCE: Option<SourceAxis> = None;
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::MonthTurnTurn)];

    type Config = MonthTurnEffectConfig;
    type Runtime = MonthTurnEffect;

    fn create(cfg: MonthTurnEffectConfig) -> MonthTurnEffect {
        MonthTurnEffect::new(cfg.window_days.resolved())
    }
}

impl crate::contract::Config for MonthTurnEffectConfig {
    fn defaults() -> Self {
        MonthTurnEffectConfig { window_days: Param::Solo(5) }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // window_days: Class M (u32, calendar window) — auto leaves u32 Solo; must patch.
        s.window_days = Param::many((1u32..=30).collect());
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for MonthTurnEffect {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::MonthTurnTurn, "Month Turn", Color::hex(0xFF9800))
            .bounds(0.0, 1.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_month_turn_effect_creation() {
        let mte = MonthTurnEffect::new(3);
        assert!(mte.is_ready());
    }

    #[test]
    fn test_month_turn_effect_start_of_month() {
        let mut mte = MonthTurnEffect::new(3);
        // 2024-01-01 00:00:00 UTC — canonical ms ×1000
        let value = mte.feed(1_704_067_200_000);
        assert_eq!(value, 1.0, "Day 1 should be at start of month");
    }

    #[test]
    fn test_month_turn_effect_mid_month() {
        let mut mte = MonthTurnEffect::new(3);
        // 2024-01-15 — canonical ms
        let value = mte.feed(1_704_067_200_000 + 14 * 86_400_000);
        assert_eq!(value, 0.0, "Day 15 should be far from turn");
    }

    #[test]
    fn test_month_turn_effect_reset() {
        let mut mte = MonthTurnEffect::new(3);
        mte.feed(1_704_067_200_000);
        mte.reset();
        assert_eq!(mte.value, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_timed_bar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<MonthTurnEffect as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::MonthTurn(cfg).build_solo().unwrap();
        f.feed(1_704_067_200_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 9999.0, volume: 0.0,
        });
        let v = f.primary();
        assert_eq!(v, 1.0, "Jan 1 should be turn score 1.0, got {v}");
    }
}
