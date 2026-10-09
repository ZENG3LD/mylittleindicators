//! Start/End-of-quarter binary flags within an N-day window.

use crate::core::types::CalendarService;

/// Emits two binary flags: `start_flag` (bar is within `window_days` of the
/// quarter start) and `end_flag` (bar is within `window_days` of the quarter
/// end). Uses `CalendarService` for all date arithmetic.
///
/// Price-free: the wall-clock coordinate only, no kline source.
#[derive(Debug, Clone)]
pub struct StartEndOfQuarterFlags {
    window_days: u32,
    pub start_flag: f64,
    pub end_flag: f64,
}

impl StartEndOfQuarterFlags {
    pub fn new(window_days: u32) -> Self {
        Self {
            window_days: window_days.clamp(1, 7),
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


    /// Named getter for the `start` brace output (1.0 within window of quarter start).
    pub fn start(&self) -> f64 {
        self.start_flag
    }

    /// Named getter for the `end` brace output (1.0 within window of quarter end).
    pub fn end(&self) -> f64 {
        self.end_flag
    }

    /// Feed the bar's wall-clock (canonical MILLISECONDS); price-free (pure-time core).
    pub fn feed(&mut self, ts_ms: i64) -> (f64, f64) {
        let ts_secs = ts_ms.div_euclid(1000);
        let (y, m, d) = CalendarService::ymd_from_timestamp(ts_secs);
        let q_start_month = ((m - 1) / 3) * 3 + 1;
        let q_end_month = q_start_month + 2;
        let start = m == q_start_month && d <= self.window_days;
        let end =
            m == q_end_month && (CalendarService::days_in_month(y, m) - d + 1) <= self.window_days;
        self.start_flag = if start { 1.0 } else { 0.0 };
        self.end_flag = if end { 1.0 } else { 0.0 };
        (self.start_flag, self.end_flag)
    }
}

impl Default for StartEndOfQuarterFlags {
    /// Factory default: window_days = 7.
    fn default() -> Self {
        Self::new(7)
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

/// Typed config for [`StartEndOfQuarterFlags`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct StartEndOfQuarterFlagsConfig {
    pub window_days: Param<u32>,
}

impl Indicator for StartEndOfQuarterFlags {
    const ID: IndicatorId = IndicatorId::SoqEoq;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Pure-time core (the `Time` flavor): no kline source — only the wall-clock coordinate.
    const SOURCE: Option<SourceAxis> = None;
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::SoqEoqStart),
        Output::discrete(IndicatorOutputId::SoqEoqEnd),
    ];

    type Config = StartEndOfQuarterFlagsConfig;
    type Runtime = StartEndOfQuarterFlags;

    fn create(cfg: StartEndOfQuarterFlagsConfig) -> StartEndOfQuarterFlags {
        StartEndOfQuarterFlags::new(cfg.window_days.resolved())
    }
}

impl crate::contract::Config for StartEndOfQuarterFlagsConfig {
    fn defaults() -> Self {
        StartEndOfQuarterFlagsConfig { window_days: Param::Solo(7) }
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


impl Render for StartEndOfQuarterFlags {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::SoqEoqStart, "SOQ Flag", Color::hex(0xFF9800))
            .line_output(IndicatorOutputId::SoqEoqEnd, "EOQ Flag", Color::hex(0x9C27B0))
            .bounds(0.0, 1.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_start_end_of_quarter_flags_creation() {
        let flags = StartEndOfQuarterFlags::new(5);
        assert!(flags.is_ready());
    }

    #[test]
    fn test_start_of_quarter() {
        let mut flags = StartEndOfQuarterFlags::new(5);
        // 1704067200 is 2024-01-01 00:00:00 UTC — start of Q1 (canonical ms ×1000)
        let (start, end) = flags.feed(1_704_067_200_000);
        assert_eq!(start, 1.0, "Jan 1 should be start of quarter");
        assert_eq!(end, 0.0);
    }

    #[test]
    fn test_quarter_flags_range() {
        let mut flags = StartEndOfQuarterFlags::new(5);
        for day in 0..90_i64 {
            let ts = 1_704_067_200_000 + day * 86_400_000;
            let (start, end) = flags.feed(ts);
            assert!(start == 0.0 || start == 1.0);
            assert!(end == 0.0 || end == 1.0);
        }
    }

    #[test]
    fn test_quarter_flags_reset() {
        let mut flags = StartEndOfQuarterFlags::new(5);
        flags.feed(1_704_067_200_000);
        flags.reset();
        assert_eq!(flags.start_flag, 0.0);
        assert_eq!(flags.end_flag, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_timed_bar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<StartEndOfQuarterFlags as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::SoqEoq(cfg).build_solo().unwrap();
        f.feed(1_704_067_200_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 9999.0, volume: 0.0,
        });
        let v = f.primary();
        assert_eq!(v, 1.0, "start flag on Jan 1 should be 1.0, got {v}");
    }
}
