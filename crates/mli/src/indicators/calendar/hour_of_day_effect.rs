//! Hour-of-day effect (0..23) — pure civil-time indicator, price-free.


/// Emits the UTC hour of the bar (0..=23) as a scalar feature.
///
/// Price-free time-only core: consumes only the wall-clock coordinate (canonical ms).
/// The indicator is always ready; it returns 0.0 until the first `feed` call.
#[derive(Debug, Clone)]
pub struct HourOfDayEffect {
    value: f64,
}

impl Default for HourOfDayEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl HourOfDayEffect {
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
    pub fn hour(&self) -> f64 {
        self.value
    }

    /// Feed the bar's wall-clock (canonical MILLISECONDS); price-free (time-only core).
    pub fn feed(&mut self, ts_ms: i64) -> f64 {
        let ts_secs = ts_ms.div_euclid(1000);
        let h = ((ts_secs.rem_euclid(86_400)) / 3600) as f64;
        self.value = h;
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

/// Typed config for [`HourOfDayEffect`] — no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct HourOfDayEffectConfig;

impl Indicator for HourOfDayEffect {
    const ID: IndicatorId = IndicatorId::HourDay;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Pure-time core (the `Time` flavor): no kline source — only the wall-clock coordinate.
    const SOURCE: Option<SourceAxis> = None;
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::HourDayHour)];

    type Config = HourOfDayEffectConfig;
    type Runtime = HourOfDayEffect;

    fn create(_cfg: HourOfDayEffectConfig) -> HourOfDayEffect {
        HourOfDayEffect::new()
    }
}

impl crate::contract::Config for HourOfDayEffectConfig {
    fn defaults() -> Self {
        HourOfDayEffectConfig
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


impl Render for HourOfDayEffect {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::HourDayHour, "Hour of Day", Color::hex(0x2196F3))
            .bounds(0.0, 24.0)
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hour_of_day_effect_creation() {
        let hod = HourOfDayEffect::new();
        assert!(hod.is_ready());
        assert_eq!(hod.value(), 0.0);
    }

    #[test]
    fn test_hour_of_day_effect_update() {
        let mut hod = HourOfDayEffect::new();
        // 1700000000 s = ~2023-11-14 22:13:20 UTC (canonical ms → ×1000)
        let value = hod.feed(1_700_000_000_000);
        assert!(value >= 0.0 && value < 24.0, "Hour should be in [0, 24)");
    }

    #[test]
    fn test_hour_of_day_effect_range() {
        let mut hod = HourOfDayEffect::new();
        for hour in 0..24_i64 {
            let ts = 1_700_000_000_000 + hour * 3_600_000;
            let value = hod.feed(ts);
            assert!(value >= 0.0 && value < 24.0, "Hour should be in [0, 24)");
        }
    }

    #[test]
    fn test_hour_of_day_effect_reset() {
        let mut hod = HourOfDayEffect::new();
        hod.feed(1_700_000_000_000);
        hod.reset();
        assert_eq!(hod.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_timed_bar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        // ts=1_700_000_000_000 ms → 22:13:20 UTC → hour 22. Time is the orthogonal feed arg.
        let mut f = IndicatorOrder::HourDay(HourOfDayEffectConfig).build_solo().unwrap();
        f.feed(1_700_000_000_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 9999.0, volume: 0.0,
        });
        let h = f.primary();
        assert!(h >= 0.0 && h < 24.0, "expected a valid hour, got {h}");
    }
}
