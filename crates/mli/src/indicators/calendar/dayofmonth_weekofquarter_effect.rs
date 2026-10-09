//! Day-of-Month and Week-of-Quarter effect — mean log-returns by day (1..31) and week (1..13).

use crate::core::types::CalendarService;

/// Accumulates log-returns per day-of-month (31 buckets) and week-of-quarter
/// (13 buckets). Emits the average of all populated day-of-month means.
///
/// Both time fields are derived from the bar's unix-seconds timestamp via
/// `CalendarService`.
#[derive(Debug, Clone)]
pub struct DayOfMonthWeekOfQuarterEffect {
    dom_counts: [usize; 31],
    dom_sums: [f64; 31],
    woq_counts: [usize; 13],
    woq_sums: [f64; 13],
    last_close: Option<f64>,
    pub dom_means: [f64; 31],
    pub woq_means: [f64; 13],
}

impl Default for DayOfMonthWeekOfQuarterEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl DayOfMonthWeekOfQuarterEffect {
    pub fn new() -> Self {
        Self {
            dom_counts: [0; 31],
            dom_sums: [0.0; 31],
            woq_counts: [0; 13],
            woq_sums: [0.0; 13],
            last_close: None,
            dom_means: [0.0; 31],
            woq_means: [0.0; 13],
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.dom_counts = [0; 31];
        self.dom_sums = [0.0; 31];
        self.woq_counts = [0; 13];
        self.woq_sums = [0.0; 13];
        self.last_close = None;
        self.dom_means = [0.0; 31];
        self.woq_means = [0.0; 13];
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.dom_counts.iter().any(|&c| c > 0) || self.woq_counts.iter().any(|&c| c > 0)
    }

    pub fn value(&self) -> f64 {
        let dom_avg = self.dom_means.iter().filter(|&&x| x != 0.0).sum::<f64>()
            / self.dom_means.iter().filter(|&&x| x != 0.0).count().max(1) as f64;
        dom_avg
    }

    #[inline]
    pub fn woq(&self) -> f64 {
        self.dom_means.iter().filter(|&&x| x != 0.0).sum::<f64>()
            / self.dom_means.iter().filter(|&&x| x != 0.0).count().max(1) as f64
    }

    /// Feed the bar's wall-clock (canonical MILLISECONDS) and close. Day-of-month and
    /// week-of-quarter are derived from `ts_ms` via `CalendarService`.
    pub fn feed(&mut self, ts_ms: i64, close: f64) {
        let ts_secs = ts_ms.div_euclid(1000);
        let (_y, _m, d) = CalendarService::ymd_from_timestamp(ts_secs);
        let day_of_month = d.clamp(1, 31);
        let week_of_quarter = CalendarService::week_of_quarter(ts_secs).clamp(1, 13);
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            let di = (day_of_month as usize) - 1;
            let wi = (week_of_quarter as usize) - 1;
            self.dom_counts[di] += 1;
            self.dom_sums[di] += r;
            self.dom_means[di] = self.dom_sums[di] / self.dom_counts[di] as f64;
            self.woq_counts[wi] += 1;
            self.woq_sums[wi] += r;
            self.woq_means[wi] = self.woq_sums[wi] / self.woq_counts[wi] as f64;
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

/// Typed config for [`DayOfMonthWeekOfQuarterEffect`] — no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct DayOfMonthWeekOfQuarterEffectConfig;

impl Indicator for DayOfMonthWeekOfQuarterEffect {
    const ID: IndicatorId = IndicatorId::DomWoq;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field {
        default: crate::engine::ohlcv_field::OhlcvField::Close,
    });
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::DomWoqWoq)];

    type Config = DayOfMonthWeekOfQuarterEffectConfig;
    type Runtime = DayOfMonthWeekOfQuarterEffect;

    fn create(_cfg: DayOfMonthWeekOfQuarterEffectConfig) -> DayOfMonthWeekOfQuarterEffect {
        DayOfMonthWeekOfQuarterEffect::new()
    }
}

impl crate::contract::Config for DayOfMonthWeekOfQuarterEffectConfig {
    fn defaults() -> Self {
        DayOfMonthWeekOfQuarterEffectConfig
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


impl Render for DayOfMonthWeekOfQuarterEffect {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::DomWoqWoq, "DOM WoQ", Color::hex(0x009688))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dayofmonth_weekofquarter_creation() {
        let eff = DayOfMonthWeekOfQuarterEffect::new();
        assert!(!eff.is_ready());
    }

    #[test]
    fn test_dayofmonth_weekofquarter_update() {
        let mut eff = DayOfMonthWeekOfQuarterEffect::new();
        eff.feed(1_704_067_200_000, 100.0); // Jan 1 (canonical ms ×1000)
        eff.feed(1_704_153_600_000, 101.0); // Jan 2
        assert!(eff.is_ready());
    }

    #[test]
    fn test_dayofmonth_weekofquarter_means_finite() {
        let mut eff = DayOfMonthWeekOfQuarterEffect::new();
        for i in 0..10_i64 {
            let price = 100.0 + i as f64;
            let ts = 1_704_067_200_000 + i * 86_400_000;
            eff.feed(ts, price);
        }
        for mean in &eff.dom_means {
            assert!(mean.is_finite());
        }
        for mean in &eff.woq_means {
            assert!(mean.is_finite());
        }
    }

    #[test]
    fn test_dayofmonth_weekofquarter_reset() {
        let mut eff = DayOfMonthWeekOfQuarterEffect::new();
        eff.feed(1_704_067_200_000, 100.0);
        eff.feed(1_704_153_600_000, 101.0);
        eff.reset();
        assert!(!eff.is_ready());
    }

    #[test]
    fn factory_feeds_resolved_timed_bar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f =
            IndicatorOrder::DomWoq(DayOfMonthWeekOfQuarterEffectConfig).build_solo().unwrap();
        f.feed(1_704_067_200_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 100.0, volume: 0.0,
        });
        f.feed(1_704_153_600_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 102.0, volume: 0.0,
        });
        let v = f.primary();
        assert!(v.is_finite(), "dom/woq mean should be finite, got {v}");
    }
}
