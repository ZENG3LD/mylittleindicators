//! Day-of-week-in-month effect — which occurrence (1..=5) of the weekday within the month.

use crate::core::types::CalendarService;

/// Emits the occurrence index (1..=5) of the bar's weekday within the current
/// month. For example, the third Monday of the month → 3.0.
///
/// The local `weekday_of_date` (Zeller congruence) duplicate that was in the
/// original file has been removed; weekday is now derived from the timestamp
/// via `CalendarService::weekday_from_timestamp` (0=Mon..6=Sun).
/// Price-free: the wall-clock coordinate only, no kline source.
#[derive(Debug, Clone)]
pub struct DayOfWeekInMonthEffect {
    current_value: f64,
}

impl Default for DayOfWeekInMonthEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl DayOfWeekInMonthEffect {
    pub fn new() -> Self {
        Self { current_value: 0.0 }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.current_value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }

    pub fn value(&self) -> f64 {
        self.current_value
    }

    #[inline]
    pub fn pos(&self) -> f64 {
        self.current_value
    }

    /// Feed the bar's wall-clock (canonical MILLISECONDS); price-free (pure-time core).
    pub fn feed(&mut self, ts_ms: i64) -> f64 {
        let ts_secs = ts_ms.div_euclid(1000);
        let (year, month, day) = CalendarService::ymd_from_timestamp(ts_secs);
        let weekday = CalendarService::weekday_from_timestamp(ts_secs); // 0=Mon..6=Sun
        let occurrence =
            Self::weekday_occurrence_in_month(year, month, day, weekday) as f64;
        self.current_value = occurrence;
        self.current_value
    }

    fn weekday_occurrence_in_month(year: i32, month: u32, day: u32, weekday: u32) -> u32 {
        // Find the first day of the month with the same weekday, then count occurrences.
        // CalendarService::weekday_from_timestamp(ts_of_first_of_month) gives the first day's weekday.
        // Use the unix timestamp of the 1st of this month.
        let first_ts = Self::first_of_month_ts(year, month);
        let first_weekday = CalendarService::weekday_from_timestamp(first_ts); // 0=Mon..6=Sun
        // Days from 1st until the first occurrence of `weekday` in this month
        let days_until_first: u32 = ((7 + weekday as i32 - first_weekday as i32) % 7) as u32;
        let first_target_day = 1 + days_until_first;
        if day < first_target_day {
            return 0;
        }
        1 + ((day - first_target_day) / 7)
    }

    /// Unix timestamp (seconds) for the 1st of `month` in `year` at 00:00 UTC.
    /// Uses a simple day-count approach consistent with CalendarService.
    fn first_of_month_ts(year: i32, month: u32) -> i64 {
        // Count days from Unix epoch (1970-01-01) to (year, month, 1).
        let mut days: i64 = 0;
        // Years 1970..year
        for y in 1970..year {
            days += if CalendarService::is_leap(y) { 366 } else { 365 };
        }
        const MD: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        for m in 1..month {
            let d = if m == 2 && CalendarService::is_leap(year) { 29 } else { MD[(m - 1) as usize] };
            days += d as i64;
        }
        days * 86_400
    }
}

// ── contract ──────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, RenderSpec, SourceAxis, UpdateComplexity,
};
use crate::contract::Render;
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`DayOfWeekInMonthEffect`] — no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct DayOfWeekInMonthEffectConfig;

impl Indicator for DayOfWeekInMonthEffect {
    const ID: IndicatorId = IndicatorId::DayWeekMonth;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Pure-time core (the `Time` flavor): no kline source — only the wall-clock coordinate.
    const SOURCE: Option<SourceAxis> = None;
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::ordinal(IndicatorOutputId::DayWeekMonthPos)];

    type Config = DayOfWeekInMonthEffectConfig;
    type Runtime = DayOfWeekInMonthEffect;

    fn create(_cfg: DayOfWeekInMonthEffectConfig) -> DayOfWeekInMonthEffect {
        DayOfWeekInMonthEffect::new()
    }
}

impl crate::contract::Config for DayOfWeekInMonthEffectConfig {
    fn defaults() -> Self {
        DayOfWeekInMonthEffectConfig
    }
    fn machine_defaults() -> Self {
        // Unit struct — no Param fields; machine_defaults_auto() = defaults().
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for DayOfWeekInMonthEffect {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(
                IndicatorOutputId::DayWeekMonthPos,
                "Day/Week/Month",
                Color::hex(0x9C27B0),
            )
            .bounds(0.0, 5.0)
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_of_week_in_month_creation() {
        let eff = DayOfWeekInMonthEffect::new();
        assert!(eff.is_ready());
    }

    #[test]
    fn test_day_of_week_in_month_range() {
        let mut eff = DayOfWeekInMonthEffect::new();
        // Jan 2024 — 31 days (canonical ms ×1000)
        for day in 0..31_i64 {
            let ts = 1_704_067_200_000 + day * 86_400_000;
            let value = eff.feed(ts);
            assert!(value >= 0.0 && value <= 5.0, "Occurrence should be in [0, 5], got {value}");
        }
    }

    #[test]
    fn test_day_of_week_in_month_reset() {
        let mut eff = DayOfWeekInMonthEffect::new();
        eff.feed(1_704_067_200_000);
        eff.reset();
        assert_eq!(eff.current_value, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_timed_bar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f =
            IndicatorOrder::DayWeekMonth(DayOfWeekInMonthEffectConfig).build_solo().unwrap();
        f.feed(1_700_000_000_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 9999.0, volume: 0.0,
        });
        let v = f.primary();
        assert!(v >= 0.0 && v <= 5.0, "occurrence should be in [0, 5], got {v}");
    }
}
