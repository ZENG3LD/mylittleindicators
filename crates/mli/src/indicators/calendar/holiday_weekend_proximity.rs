//! Holiday/Weekend proximity effect — proximity score to nearest weekend day.

use crate::core::types::CalendarService;

/// Emits a [0, 1] proximity score to the nearest weekend day (Sat/Sun).
///
/// Distance mapping (Mon=0..Sun=6 per CalendarService): Mon→1, Tue→2, Wed→3,
/// Thu→2, Fri→1, Sat→0, Sun→0.  Score = `max(0, 1 − dist/window)`.
/// Holiday handling is limited to weekend proximity (no calendar of public
/// holidays). Price-free: the wall-clock coordinate only, no kline source.
#[derive(Debug, Clone)]
pub struct HolidayWeekendProximityEffect {
    window: u32,
    value: f64,
}

impl HolidayWeekendProximityEffect {
    pub fn new(window_days: u32) -> Self {
        Self {
            window: window_days.clamp(1, 5),
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
    pub fn prox(&self) -> f64 {
        self.value
    }

    /// Feed the bar's wall-clock (canonical MILLISECONDS); price-free (pure-time core).
    pub fn feed(&mut self, ts_ms: i64) -> f64 {
        let ts_secs = ts_ms.div_euclid(1000);
        // CalendarService: 0=Mon..6=Sun
        let weekday = CalendarService::weekday_from_timestamp(ts_secs);
        let dist_to_weekend = match weekday {
            0 => 1u32, // Mon
            1 => 2,    // Tue
            2 => 3,    // Wed
            3 => 2,    // Thu
            4 => 1,    // Fri
            5 => 0,    // Sat
            6 => 0,    // Sun
            _ => 3,
        };
        self.value = if dist_to_weekend <= self.window {
            1.0 - (dist_to_weekend as f64 / self.window as f64)
        } else {
            0.0
        };
        self.value
    }
}

impl Default for HolidayWeekendProximityEffect {
    /// Factory default: window_days = 2.
    fn default() -> Self {
        Self::new(2)
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

/// Typed config for [`HolidayWeekendProximityEffect`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct HolidayWeekendProximityEffectConfig {
    pub window_days: Param<u32>,
}

impl Indicator for HolidayWeekendProximityEffect {
    const ID: IndicatorId = IndicatorId::HolidayProx;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Pure-time core (the `Time` flavor): no kline source — only the wall-clock coordinate.
    const SOURCE: Option<SourceAxis> = None;
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::HolidayProxProx)];

    type Config = HolidayWeekendProximityEffectConfig;
    type Runtime = HolidayWeekendProximityEffect;

    fn create(cfg: HolidayWeekendProximityEffectConfig) -> HolidayWeekendProximityEffect {
        HolidayWeekendProximityEffect::new(cfg.window_days.resolved())
    }
}

impl crate::contract::Config for HolidayWeekendProximityEffectConfig {
    fn defaults() -> Self {
        HolidayWeekendProximityEffectConfig { window_days: Param::Solo(2) }
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


impl Render for HolidayWeekendProximityEffect {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::HolidayProxProx, "Holiday Prox", Color::hex(0xE91E63))
            .bounds(0.0, 1.0)
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_holiday_weekend_proximity_creation() {
        let hwp = HolidayWeekendProximityEffect::new(3);
        assert!(hwp.is_ready());
    }

    #[test]
    fn test_holiday_weekend_proximity_range() {
        let mut hwp = HolidayWeekendProximityEffect::new(3);
        // Walk a week of timestamps (Mon 2023-11-13 00:00 UTC + N days) — canonical ms ×1000
        for day in 0..7_i64 {
            let ts = 1_699_833_600_000 + day * 86_400_000;
            let value = hwp.feed(ts);
            assert!(value >= 0.0 && value <= 1.0, "Value should be in [0, 1], got {value}");
        }
    }

    #[test]
    fn test_holiday_weekend_proximity_reset() {
        let mut hwp = HolidayWeekendProximityEffect::new(3);
        hwp.feed(1_700_000_000_000);
        hwp.reset();
        assert_eq!(hwp.value, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_timed_bar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<HolidayWeekendProximityEffect as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::HolidayProx(cfg).build_solo().unwrap();
        f.feed(1_700_000_000_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 9999.0, volume: 0.0,
        });
        let v = f.primary();
        assert!(v >= 0.0 && v <= 1.0, "proximity should be in [0, 1], got {v}");
    }
}
