//! Start/End-of-week binary flags within an N-day window (Mon..Sun) — price-free.

use crate::core::types::CalendarService;

/// Emits two flags: `start_flag` (proximity to week start / Monday) and
/// `end_flag` (proximity to week end / Sunday), each 1.0 within
/// `window_days` of the respective boundary, 0.0 otherwise.
///
/// Price-free: the wall-clock coordinate only, no kline source.
#[derive(Debug, Clone)]
pub struct StartEndOfWeekFlags {
    window_days: u32,
    pub start_flag: f64,
    pub end_flag: f64,
}

impl StartEndOfWeekFlags {
    pub fn new(window_days: u32) -> Self {
        Self {
            window_days: window_days.clamp(1, 3),
            start_flag: 0.0,
            end_flag: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.start_flag = 0.0;
        self.end_flag = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }


    /// Named getter for the `start` brace output (1.0 within window of week start / Monday).
    pub fn start(&self) -> f64 {
        self.start_flag
    }

    /// Named getter for the `end` brace output (1.0 within window of week end / Sunday).
    pub fn end(&self) -> f64 {
        self.end_flag
    }

    /// Feed the bar's wall-clock (canonical MILLISECONDS); price-free (pure-time core).
    /// Weekday: 0=Mon..6=Sun (CalendarService convention).
    pub fn feed(&mut self, ts_ms: i64) -> (f64, f64) {
        let ts_secs = ts_ms.div_euclid(1000);
        let weekday = CalendarService::weekday_from_timestamp(ts_secs); // 0=Mon..6=Sun
        let dist_start = weekday.min(6);
        let dist_end = 6 - weekday.min(6);
        self.start_flag = if dist_start <= self.window_days { 1.0 } else { 0.0 };
        self.end_flag = if dist_end <= self.window_days { 1.0 } else { 0.0 };
        (self.start_flag, self.end_flag)
    }
}

impl Default for StartEndOfWeekFlags {
    /// Factory default: window_days = 3.
    fn default() -> Self {
        Self::new(3)
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

/// Typed config for [`StartEndOfWeekFlags`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct StartEndOfWeekFlagsConfig {
    pub window_days: Param<u32>,
}

impl Indicator for StartEndOfWeekFlags {
    const ID: IndicatorId = IndicatorId::SowEow;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Pure-time core (the `Time` flavor): no kline source — only the wall-clock coordinate.
    const SOURCE: Option<SourceAxis> = None;
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::SowEowStart),
        Output::discrete(IndicatorOutputId::SowEowEnd),
    ];

    type Config = StartEndOfWeekFlagsConfig;
    type Runtime = StartEndOfWeekFlags;

    fn create(cfg: StartEndOfWeekFlagsConfig) -> StartEndOfWeekFlags {
        StartEndOfWeekFlags::new(cfg.window_days.resolved())
    }
}

impl crate::contract::Config for StartEndOfWeekFlagsConfig {
    fn defaults() -> Self {
        StartEndOfWeekFlagsConfig { window_days: Param::Solo(3) }
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


impl Render for StartEndOfWeekFlags {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::SowEowStart, "SOW Flag", Color::hex(0x009688))
            .line_output(IndicatorOutputId::SowEowEnd, "EOW Flag", Color::hex(0xE91E63))
            .bounds(0.0, 1.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_end_of_week_flags_creation() {
        let flags = StartEndOfWeekFlags::new(1);
        assert!(flags.is_ready());
    }

    #[test]
    fn test_start_end_of_week_reset() {
        let mut flags = StartEndOfWeekFlags::new(1);
        // Mon 2023-11-13 ~ ts 1699833600 (canonical ms → ×1000)
        flags.feed(1_699_833_600_000);
        flags.reset();
        assert_eq!(flags.start_flag, 0.0);
        assert_eq!(flags.end_flag, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_timed_bar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<StartEndOfWeekFlags as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::SowEow(cfg).build_solo().unwrap();
        // Feed a Monday (ts 1699833600 = 2023-11-13 Mon) — canonical ms ×1000
        f.feed(1_699_833_600_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 9999.0, volume: 0.0,
        });
        let v = f.primary();
        assert!(v == 0.0 || v == 1.0, "start flag should be binary, got {v}");
    }
}
