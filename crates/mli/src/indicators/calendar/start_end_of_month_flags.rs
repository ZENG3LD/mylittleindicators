//! Start/End-of-month binary flags within an N-day window.

use crate::core::types::CalendarService;

/// Emits two binary flags: `start_flag` (bar is within `window_days` of month
/// start, day 1) and `end_flag` (bar is within `window_days` of month end).
///
/// Uses `CalendarService::days_in_month` — the local `days_in_month` and
/// `is_leap` duplicates that were in the original file have been removed.
/// Price-free: the wall-clock coordinate only, no kline source.
#[derive(Debug, Clone)]
pub struct StartEndOfMonthFlags {
    window_days: u32,
    pub start_flag: f64,
    pub end_flag: f64,
}

impl StartEndOfMonthFlags {
    pub fn new(window_days: u32) -> Self {
        Self {
            window_days: window_days.clamp(1, 5),
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


    /// Named getter for the `start` brace output (1.0 within window of month start).
    pub fn start(&self) -> f64 {
        self.start_flag
    }

    /// Named getter for the `end` brace output (1.0 within window of month end).
    pub fn end(&self) -> f64 {
        self.end_flag
    }

    /// Feed the bar's wall-clock (canonical MILLISECONDS); price-free (pure-time core).
    pub fn feed(&mut self, ts_ms: i64) -> (f64, f64) {
        let ts_secs = ts_ms.div_euclid(1000);
        let (year, month, day) = CalendarService::ymd_from_timestamp(ts_secs);
        let dim = CalendarService::days_in_month(year, month);
        let dist_start = (day as i32 - 1).unsigned_abs();
        let dist_end = (dim as i32 - day as i32).unsigned_abs();
        self.start_flag = if dist_start <= self.window_days { 1.0 } else { 0.0 };
        self.end_flag = if dist_end <= self.window_days { 1.0 } else { 0.0 };
        (self.start_flag, self.end_flag)
    }
}

impl Default for StartEndOfMonthFlags {
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

/// Typed config for [`StartEndOfMonthFlags`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct StartEndOfMonthFlagsConfig {
    pub window_days: Param<u32>,
}

impl Indicator for StartEndOfMonthFlags {
    const ID: IndicatorId = IndicatorId::SomEom;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Pure-time core (the `Time` flavor): no kline source — only the wall-clock coordinate.
    const SOURCE: Option<SourceAxis> = None;
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::SomEomStart),
        Output::discrete(IndicatorOutputId::SomEomEnd),
    ];

    type Config = StartEndOfMonthFlagsConfig;
    type Runtime = StartEndOfMonthFlags;

    fn create(cfg: StartEndOfMonthFlagsConfig) -> StartEndOfMonthFlags {
        StartEndOfMonthFlags::new(cfg.window_days.resolved())
    }
}

impl crate::contract::Config for StartEndOfMonthFlagsConfig {
    fn defaults() -> Self {
        StartEndOfMonthFlagsConfig { window_days: Param::Solo(5) }
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


impl Render for StartEndOfMonthFlags {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::SomEomStart, "SOM Flag", Color::hex(0x9C27B0))
            .line_output(IndicatorOutputId::SomEomEnd, "EOM Flag", Color::hex(0xFF9800))
            .bounds(0.0, 1.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_end_of_month_flags_creation() {
        let flags = StartEndOfMonthFlags::new(3);
        assert!(flags.is_ready());
    }

    #[test]
    fn test_start_flag_day1() {
        let mut flags = StartEndOfMonthFlags::new(3);
        // 2024-01-01 00:00:00 UTC — canonical ms ×1000
        let (start, end) = flags.feed(1_704_067_200_000);
        assert_eq!(start, 1.0, "Day 1 should be start of month");
        assert_eq!(end, 0.0);
    }

    #[test]
    fn test_end_flag_day31() {
        let mut flags = StartEndOfMonthFlags::new(3);
        // 2024-01-31 00:00:00 UTC — canonical ms
        let (start, end) = flags.feed(1_704_067_200_000 + 30 * 86_400_000);
        assert_eq!(start, 0.0);
        assert_eq!(end, 1.0, "Day 31 should be end of month");
    }

    #[test]
    fn test_start_end_of_month_reset() {
        let mut flags = StartEndOfMonthFlags::new(3);
        flags.feed(1_704_067_200_000);
        flags.reset();
        assert_eq!(flags.start_flag, 0.0);
        assert_eq!(flags.end_flag, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_timed_bar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<StartEndOfMonthFlags as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::SomEom(cfg).build_solo().unwrap();
        f.feed(1_704_067_200_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 9999.0, volume: 0.0,
        });
        let v = f.primary();
        assert_eq!(v, 1.0, "start flag on Jan 1 should be 1.0, got {v}");
    }
}
