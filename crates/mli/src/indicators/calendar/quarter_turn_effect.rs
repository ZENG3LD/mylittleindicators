//! Quarter-turn effect — proximity score to start or end of the current quarter.

use crate::core::types::CalendarService;

/// Emits a [0, 1] score: 1.0 at the very start or end of the quarter, decaying
/// linearly to 0.0 at `window_days` distance from either boundary.
///
/// **NOTE**: the `day_distance` helper is an approximate computation that compares
/// only day-of-month numbers (ignoring month span). This approximation is kept
/// byte-identical to the original — it is NOT corrected in this pass.
///
/// Price-free: the wall-clock coordinate only, no kline source.
#[derive(Debug, Clone)]
pub struct QuarterTurnEffect {
    window: u32,
    value: f64,
}

impl QuarterTurnEffect {
    pub fn new(window_days: u32) -> Self {
        Self {
            window: window_days.clamp(1, 15),
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
        let (q_start_m, q_start_d) = Self::quarter_start(month);
        let (q_end_m, q_end_d) = Self::quarter_end(year, month);
        let dist_start =
            Self::day_distance(year, month, day, year, q_start_m, q_start_d) as u32;
        let dist_end =
            Self::day_distance(year, month, day, year, q_end_m, q_end_d) as u32;
        let near = dist_start.min(dist_end);
        self.value = if near <= self.window {
            1.0 - (near as f64 / self.window as f64)
        } else {
            0.0
        };
        self.value
    }

    fn quarter_start(month: u32) -> (u32, u32) {
        match ((month - 1) / 3) + 1 {
            1 => (1, 1),
            2 => (4, 1),
            3 => (7, 1),
            _ => (10, 1),
        }
    }

    fn quarter_end(_year: i32, month: u32) -> (u32, u32) {
        match ((month - 1) / 3) + 1 {
            1 => (3, 31),
            2 => (6, 30),
            3 => (9, 30),
            _ => (12, 31),
        }
    }

    /// Approximate day distance — compares day-of-month values only (ignores month span).
    /// Math kept byte-identical to original; NOT fixed in this pass.
    fn day_distance(_y1: i32, _m1: u32, d1: u32, _y2: i32, _m2: u32, d2: u32) -> i32 {
        (d1 as i32 - d2 as i32).abs()
    }
}

impl Default for QuarterTurnEffect {
    /// Factory default: window_days = 10.
    fn default() -> Self {
        Self::new(10)
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

/// Typed config for [`QuarterTurnEffect`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct QuarterTurnEffectConfig {
    pub window_days: Param<u32>,
}

impl Indicator for QuarterTurnEffect {
    const ID: IndicatorId = IndicatorId::QtrTurn;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Pure-time core (the `Time` flavor): no kline source — only the wall-clock coordinate.
    const SOURCE: Option<SourceAxis> = None;
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::QtrTurnTurn)];

    type Config = QuarterTurnEffectConfig;
    type Runtime = QuarterTurnEffect;

    fn create(cfg: QuarterTurnEffectConfig) -> QuarterTurnEffect {
        QuarterTurnEffect::new(cfg.window_days.resolved())
    }
}

impl crate::contract::Config for QuarterTurnEffectConfig {
    fn defaults() -> Self {
        QuarterTurnEffectConfig { window_days: Param::Solo(10) }
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


impl Render for QuarterTurnEffect {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::QtrTurnTurn, "Qtr Turn", Color::hex(0x009688))
            .bounds(0.0, 1.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quarter_turn_effect_creation() {
        let qte = QuarterTurnEffect::new(5);
        assert!(qte.is_ready());
    }

    #[test]
    fn test_start_of_quarter() {
        let mut qte = QuarterTurnEffect::new(5);
        // 2024-01-01 00:00:00 UTC = start of Q1 — canonical ms ×1000
        let value = qte.feed(1_704_067_200_000);
        assert_eq!(value, 1.0, "Jan 1 should be at start of quarter");
    }

    #[test]
    fn test_quarter_turn_range() {
        let mut qte = QuarterTurnEffect::new(5);
        for day in 0..31_i64 {
            let ts = 1_704_067_200_000 + day * 86_400_000;
            let value = qte.feed(ts);
            assert!(value >= 0.0 && value <= 1.0, "Value should be in [0, 1], got {value}");
        }
    }

    #[test]
    fn test_quarter_turn_reset() {
        let mut qte = QuarterTurnEffect::new(5);
        qte.feed(1_704_067_200_000);
        qte.reset();
        assert_eq!(qte.value, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_timed_bar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<QuarterTurnEffect as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::QtrTurn(cfg).build_solo().unwrap();
        f.feed(1_704_067_200_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 9999.0, volume: 0.0,
        });
        let v = f.primary();
        assert_eq!(v, 1.0, "Jan 1 should be qtr turn 1.0, got {v}");
    }
}
