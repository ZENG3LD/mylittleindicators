//! Month/Quarter effect — rolling mean log-return per month (12) and quarter (4).

use crate::core::types::CalendarService;

/// Accumulates log-returns per calendar month (12 buckets) and quarter
/// (4 buckets) and emits the average of all populated month-mean-returns.
///
/// Month and quarter are derived from the bar's unix-seconds timestamp via
/// `CalendarService::ymd_from_timestamp`.
#[derive(Debug, Clone)]
pub struct MonthQuarterEffect {
    month_counts: [usize; 12],
    month_sums: [f64; 12],
    quarter_counts: [usize; 4],
    quarter_sums: [f64; 4],
    last_close: Option<f64>,
    pub month_means: [f64; 12],
    pub quarter_means: [f64; 4],
}

impl Default for MonthQuarterEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl MonthQuarterEffect {
    pub fn new() -> Self {
        Self {
            month_counts: [0; 12],
            month_sums: [0.0; 12],
            quarter_counts: [0; 4],
            quarter_sums: [0.0; 4],
            last_close: None,
            month_means: [0.0; 12],
            quarter_means: [0.0; 4],
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.month_counts = [0; 12];
        self.month_sums = [0.0; 12];
        self.quarter_counts = [0; 4];
        self.quarter_sums = [0.0; 4];
        self.last_close = None;
        self.month_means = [0.0; 12];
        self.quarter_means = [0.0; 4];
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.month_counts.iter().any(|&c| c > 0)
            || self.quarter_counts.iter().any(|&c| c > 0)
    }

    pub fn value(&self) -> f64 {
        let month_avg = self.month_means.iter().filter(|&&x| x != 0.0).sum::<f64>()
            / self.month_means.iter().filter(|&&x| x != 0.0).count().max(1) as f64;
        month_avg
    }

    #[inline]
    pub fn month(&self) -> f64 {
        self.month_means.iter().filter(|&&x| x != 0.0).sum::<f64>()
            / self.month_means.iter().filter(|&&x| x != 0.0).count().max(1) as f64
    }

    /// Feed the bar's wall-clock (canonical MILLISECONDS) and close. Month and quarter are
    /// derived from `ts_ms` via `CalendarService::ymd_from_timestamp`.
    pub fn feed(&mut self, ts_ms: i64, close: f64) {
        let ts_secs = ts_ms.div_euclid(1000);
        let (_y, m, _d) = CalendarService::ymd_from_timestamp(ts_secs);
        let month = m.clamp(1, 12);
        let quarter = ((month - 1) / 3 + 1).clamp(1, 4);
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            let mi = (month as usize) - 1;
            let qi = (quarter as usize) - 1;
            self.month_counts[mi] += 1;
            self.month_sums[mi] += r;
            self.month_means[mi] = self.month_sums[mi] / self.month_counts[mi] as f64;
            self.quarter_counts[qi] += 1;
            self.quarter_sums[qi] += r;
            self.quarter_means[qi] = self.quarter_sums[qi] / self.quarter_counts[qi] as f64;
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

/// Typed config for [`MonthQuarterEffect`] — no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct MonthQuarterEffectConfig;

impl Indicator for MonthQuarterEffect {
    const ID: IndicatorId = IndicatorId::MonthQtr;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field {
        default: crate::engine::ohlcv_field::OhlcvField::Close,
    });
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::MonthQtrMonth)];

    type Config = MonthQuarterEffectConfig;
    type Runtime = MonthQuarterEffect;

    fn create(_cfg: MonthQuarterEffectConfig) -> MonthQuarterEffect {
        MonthQuarterEffect::new()
    }
}

impl crate::contract::Config for MonthQuarterEffectConfig {
    fn defaults() -> Self {
        MonthQuarterEffectConfig
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


impl Render for MonthQuarterEffect {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::MonthQtrMonth, "Month/Qtr", Color::hex(0x9C27B0))
            .bounds(1.0, 3.0)
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_month_quarter_effect_creation() {
        let mqe = MonthQuarterEffect::new();
        assert!(!mqe.is_ready());
    }

    #[test]
    fn test_month_quarter_effect_update() {
        let mut mqe = MonthQuarterEffect::new();
        // Two bars one month apart (canonical ms ×1000)
        mqe.feed(1_704_067_200_000, 100.0); // Jan 1 2024
        mqe.feed(1_706_745_600_000, 101.0); // Feb 1 2024
        assert!(mqe.is_ready());
    }

    #[test]
    fn test_month_quarter_effect_means_finite() {
        let mut mqe = MonthQuarterEffect::new();
        let base = 1_704_067_200_000_i64; // Jan 1 2024 (canonical ms)
        for i in 0..12_i64 {
            let price = 100.0 + i as f64;
            let ts = base + i * 30 * 86_400_000; // approx monthly steps
            mqe.feed(ts, price);
        }
        for mean in &mqe.month_means {
            assert!(mean.is_finite());
        }
        for mean in &mqe.quarter_means {
            assert!(mean.is_finite());
        }
    }

    #[test]
    fn test_month_quarter_effect_reset() {
        let mut mqe = MonthQuarterEffect::new();
        mqe.feed(1_704_067_200_000, 100.0);
        mqe.feed(1_706_745_600_000, 101.0);
        mqe.reset();
        assert!(!mqe.is_ready());
    }

    #[test]
    fn factory_feeds_resolved_timed_bar() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::MonthQtr(MonthQuarterEffectConfig).build_solo().unwrap();
        f.feed(1_704_067_200_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 100.0, volume: 0.0,
        });
        f.feed(1_706_745_600_000, MarketSample::Bar {
            open: 9999.0, high: 9999.0, low: 9999.0, close: 102.0, volume: 0.0,
        });
        let v = f.primary();
        assert!(v.is_finite(), "month/qtr mean should be finite, got {v}");
    }
}
