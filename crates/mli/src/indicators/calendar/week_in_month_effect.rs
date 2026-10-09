//! Week-in-month effect — which week of the month (1..=5) the bar falls in.

use crate::core::types::CalendarService;

/// Emits the week-of-month index (1..=5) for the bar's date, derived from
/// the day-of-month: `week = ((day - 1) / 7) + 1`.
///
/// Price-free: the wall-clock coordinate only, no kline source.
#[derive(Debug, Clone)]
pub struct WeekInMonthEffect {
    pub value: f64,
}

impl Default for WeekInMonthEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl WeekInMonthEffect {
    pub fn new() -> Self {
        Self { value: 0.0 }
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
    pub fn week(&self) -> f64 {
        self.value
    }

    /// Feed the bar's wall-clock (canonical MILLISECONDS); price-free (pure-time core).
    pub fn feed(&mut self, ts_ms: i64) -> f64 {
        let ts_secs = ts_ms.div_euclid(1000);
        let (_y, _m, d) = CalendarService::ymd_from_timestamp(ts_secs);
        let w = ((d - 1) / 7) + 1;
        self.value = w as f64;
        self.value
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

/// Typed config for [`WeekInMonthEffect`] — no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct WeekInMonthEffectConfig;

impl Indicator for WeekInMonthEffect {
    const ID: IndicatorId = IndicatorId::WeekMonth;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Pure-time core (the `Time` flavor): no kline source — only the wall-clock coordinate.
    const SOURCE: Option<SourceAxis> = None;
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::ordinal(IndicatorOutputId::WeekMonthWeek)];

    type Config = WeekInMonthEffectConfig;
    type Runtime = WeekInMonthEffect;

    fn create(_cfg: WeekInMonthEffectConfig) -> WeekInMonthEffect {
        WeekInMonthEffect::new()
    }
}

impl crate::contract::Config for WeekInMonthEffectConfig {
    fn defaults() -> Self {
        WeekInMonthEffectConfig
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


impl Render for WeekInMonthEffect {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::WeekMonthWeek, "Week/Month", Color::hex(0xE91E63))
            .bounds(1.0, 5.0)
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_week_in_month_effect_creation() {
        let wim = WeekInMonthEffect::new();
        assert!(wim.is_ready());
        assert_eq!(wim.value, 0.0);
    }

    #[test]
    fn test_week_in_month_effect_range() {
        let mut wim = WeekInMonthEffect::new();
        // Jan 1, 2024 = day 1 = week 1 — canonical ms ×1000
        let value = wim.feed(1_704_067_200_000);
        assert!(value >= 1.0 && value <= 5.0, "Week should be in [1, 5], got {value}");
    }

    #[test]
    fn test_week_in_month_effect_values() {
        let mut wim = WeekInMonthEffect::new();
        let ts_base = 1_704_067_200_000_i64; // Jan 1, 2024 (canonical ms)
        for day in 0..28_i64 {
            let value = wim.feed(ts_base + day * 86_400_000);
            let expected_week = (day as u32 / 7) + 1;
            assert_eq!(
                value as u32, expected_week,
                "Day {} should be week {}", day + 1, expected_week
            );
        }
    }

    #[test]
    fn test_week_in_month_effect_reset() {
        let mut wim = WeekInMonthEffect::new();
        wim.feed(1_704_067_200_000);
        wim.reset();
        assert_eq!(wim.value, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_timed_bar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::WeekMonth(WeekInMonthEffectConfig).build_solo().unwrap();
        f.feed(1_704_067_200_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 9999.0, volume: 0.0,
        });
        let v = f.primary();
        assert!(v >= 1.0 && v <= 5.0, "week should be in [1, 5], got {v}");
    }
}
