//! Weekday effect — rolling mean log-return per weekday bucket (0=Mon..6=Sun).

use crate::core::types::CalendarService;

/// Accumulates log-returns by weekday (7 buckets: 0=Mon..6=Sun) and emits
/// the running mean return for the current bar's weekday bucket.
///
/// The caller no longer supplies `weekday` externally — it is derived from the
/// bar's unix-seconds timestamp inside `feed` via `CalendarService`.
#[derive(Debug, Clone)]
pub struct WeekdayEffect {
    counts: [usize; 7],
    sums: [f64; 7],
    last_close: Option<f64>,
    pub last_bucket: usize,
    pub mean_returns: [f64; 7],
}

impl Default for WeekdayEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl WeekdayEffect {
    pub fn new() -> Self {
        Self {
            counts: [0; 7],
            sums: [0.0; 7],
            last_close: None,
            last_bucket: 0,
            mean_returns: [0.0; 7],
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.counts = [0; 7];
        self.sums = [0.0; 7];
        self.last_close = None;
        self.last_bucket = 0;
        self.mean_returns = [0.0; 7];
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.counts.iter().any(|&c| c > 0)
    }

    pub fn value(&self) -> f64 {
        self.mean_returns[self.last_bucket]
    }

    #[inline]
    pub fn day(&self) -> f64 {
        self.mean_returns[self.last_bucket]
    }

    /// Feed the bar's wall-clock (canonical MILLISECONDS) and close. Weekday (0=Mon..6=Sun)
    /// is derived from `ts_ms` via `CalendarService::weekday_from_timestamp`.
    pub fn feed(&mut self, ts_ms: i64, close: f64) {
        let ts_secs = ts_ms.div_euclid(1000);
        let weekday = CalendarService::weekday_from_timestamp(ts_secs); // 0=Mon..6=Sun
        // Legacy expected bucket: weekday as usize (0..6)
        let b = (weekday as usize).min(6);
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            self.counts[b] += 1;
            self.sums[b] += r;
            self.mean_returns[b] = self.sums[b] / self.counts[b] as f64;
            self.last_bucket = b;
        }
        self.last_close = Some(close);
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

/// Typed config for [`WeekdayEffect`] — no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct WeekdayEffectConfig;

impl Indicator for WeekdayEffect {
    const ID: IndicatorId = IndicatorId::Weekday;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field {
        default: crate::engine::ohlcv_field::OhlcvField::Close,
    });
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::WeekdayDay)];

    type Config = WeekdayEffectConfig;
    type Runtime = WeekdayEffect;

    fn create(_cfg: WeekdayEffectConfig) -> WeekdayEffect {
        WeekdayEffect::new()
    }
}

impl crate::contract::Config for WeekdayEffectConfig {
    fn defaults() -> Self {
        WeekdayEffectConfig
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


impl Render for WeekdayEffect {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::WeekdayDay, "Weekday", Color::hex(0x00BCD4))
            .bounds(1.0, 7.0)
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weekday_effect_creation() {
        let we = WeekdayEffect::new();
        assert!(!we.is_ready());
        assert_eq!(we.last_bucket, 0);
    }

    #[test]
    fn test_weekday_effect_update() {
        let mut we = WeekdayEffect::new();
        // Mon 2023-11-13 00:00 UTC (canonical ms ×1000)
        we.feed(1_699_833_600_000, 100.0);
        // Tue 2023-11-14 00:00 UTC
        we.feed(1_699_920_000_000, 101.0);
        assert!(we.is_ready());
    }

    #[test]
    fn test_weekday_effect_mean_returns_finite() {
        let mut we = WeekdayEffect::new();
        let base = 1_699_833_600_000_i64; // Mon 2023-11-13 (canonical ms)
        for i in 0..14_i64 {
            let price = 100.0 + i as f64;
            let ts = base + i * 86_400_000;
            we.feed(ts, price);
        }
        for mean in &we.mean_returns {
            assert!(mean.is_finite());
        }
    }

    #[test]
    fn test_weekday_effect_reset() {
        let mut we = WeekdayEffect::new();
        we.feed(1_699_833_600_000, 100.0);
        we.feed(1_699_920_000_000, 101.0);
        we.reset();
        assert!(!we.is_ready());
        assert_eq!(we.last_bucket, 0);
    }

    #[test]
    fn factory_feeds_resolved_timed_bar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Weekday(WeekdayEffectConfig).build_solo().unwrap();
        // Two bars one day apart — close feeds into the weekday accumulator
        f.feed(1_699_833_600_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 100.0, volume: 0.0,
        });
        f.feed(1_699_920_000_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 101.0, volume: 0.0,
        });
        let v = f.primary();
        assert!(v.is_finite(), "weekday mean return should be finite, got {v}");
    }
}
