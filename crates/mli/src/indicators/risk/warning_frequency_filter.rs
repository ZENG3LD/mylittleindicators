//! WarningFrequencyFilter — debounce filter for market warning events.
//!
//! Emits a signal only when:
//! - warning_kind differs from the previous emitted warning, OR
//! - enough time has passed since last emission (> min_interval_ms)

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::market_warning_consumer::MarketWarningConsumer;
use crate::engine::time_window::TimeWindow;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::MarketWarning;

/// Debounce filter that suppresses repeated identical market warnings.
///
/// Returns `Signal(+1)` when a warning passes the filter,
/// `Signal(0)` when it is suppressed.
///
/// A warning passes if:
/// - Its `warning_kind` differs from the previous emitted warning, OR
/// - The elapsed time since last emission exceeds `min_interval_ms`.
///
/// Output: `Signal(i8)`: `+1` = emitted, `0` = filtered.
#[derive(Debug, Clone)]
pub struct WarningFrequencyFilter {
    min_interval_ms: i64,
    last_kind: Option<String>,
    last_emission_ts: Option<i64>,
    last_signal: i8,
}

impl WarningFrequencyFilter {
    /// Create a new filter.
    ///
    /// - `min_interval_ms`: minimum milliseconds between emissions of the same warning kind.
    pub fn new(min_interval_ms: i64) -> Self {
        Self {
            min_interval_ms: min_interval_ms.max(0),
            last_kind: None,
            last_emission_ts: None,
            last_signal: 0,
        }
    }

    fn should_emit(&self, warning: &MarketWarning) -> bool {
        match (&self.last_kind, self.last_emission_ts) {
            (None, _) => true,
            (Some(prev_kind), _) if prev_kind != &warning.warning_kind => true,
            (Some(_), Some(last_ts)) => {
                warning.timestamp - last_ts > self.min_interval_ms
            }
            (Some(_), None) => true,
        }
    }
}

impl Default for WarningFrequencyFilter {
    fn default() -> Self {
        Self::new(60_000)
    }
}

impl MarketWarningConsumer for WarningFrequencyFilter {
    fn update_market_warning(&mut self, w: &MarketWarning) {
        if self.should_emit(w) {
            self.last_kind = Some(w.warning_kind.clone());
            self.last_emission_ts = Some(w.timestamp);
            self.last_signal = 1;
        } else {
            self.last_signal = 0;
        }
    }


    fn reset(&mut self) {
        self.last_kind = None;
        self.last_emission_ts = None;
        self.last_signal = 0;
    }

    fn is_ready(&self) -> bool {
        true
    }
}

/// Typed configuration for [`WarningFrequencyFilter`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct WarningFrequencyFilterConfig {
    pub min_interval: Param<TimeWindow>,
}

impl Indicator for WarningFrequencyFilter {
    const ID: IndicatorId = IndicatorId::WarningFrequencyFilter;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::MarketWarning];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::WarningFrequencyFilter)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = WarningFrequencyFilterConfig;
    type Runtime = WarningFrequencyFilter;

    fn create(cfg: WarningFrequencyFilterConfig) -> WarningFrequencyFilter {
        WarningFrequencyFilter::new(cfg.min_interval.resolved().as_millis())
    }
}

impl crate::contract::Config for WarningFrequencyFilterConfig {
    fn defaults() -> Self {
        WarningFrequencyFilterConfig {
            min_interval: Param::Solo(TimeWindow::Minutes(1)),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // Class N — TimeWindow: 11-value curated discrete set spanning 1s..24h
        s.min_interval = Param::many(vec![
            TimeWindow::Seconds(1),
            TimeWindow::Seconds(5),
            TimeWindow::Seconds(15),
            TimeWindow::Seconds(30),
            TimeWindow::Minutes(1),
            TimeWindow::Minutes(5),
            TimeWindow::Minutes(15),
            TimeWindow::Minutes(30),
            TimeWindow::Hours(1),
            TimeWindow::Hours(4),
            TimeWindow::Hours(24),
        ]);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for WarningFrequencyFilter {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::WarningFrequencyFilter, "Warning Filter", Color::hex(0x9C27B0))
            .zero_baseline()
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    fn make_warning(kind: &str, timestamp: i64) -> MarketWarning {
        MarketWarning {
            symbol: "BTCUSDT".to_string(),
            warning_kind: kind.to_string(),
            message: "test".to_string(),
            timestamp,
        }
    }

    #[test]
    fn first_warning_always_emits() {
        let mut f = WarningFrequencyFilter::new(60_000);
        f.update_market_warning(&make_warning("volatility", 0));
        assert_eq!(f.value() as i8, 1);
    }

    #[test]
    fn same_kind_within_interval_suppressed() {
        let mut f = WarningFrequencyFilter::new(60_000);
        f.update_market_warning(&make_warning("volatility", 0));
        f.update_market_warning(&make_warning("volatility", 1_000));
        assert_eq!(f.value() as i8, 0, "same kind too soon should be filtered");
    }

    #[test]
    fn different_kind_passes_immediately() {
        let mut f = WarningFrequencyFilter::new(60_000);
        f.update_market_warning(&make_warning("volatility", 0));
        f.update_market_warning(&make_warning("margin_call", 1_000));
        assert_eq!(f.value() as i8, 1, "different kind should pass");
    }

    #[test]
    fn same_kind_after_interval_passes() {
        let mut f = WarningFrequencyFilter::new(60_000);
        f.update_market_warning(&make_warning("volatility", 0));
        f.update_market_warning(&make_warning("volatility", 60_001));
        assert_eq!(f.value() as i8, 1, "same kind after interval should pass");
    }

    #[test]
    fn reset_clears_state() {
        let mut f = WarningFrequencyFilter::new(60_000);
        f.update_market_warning(&make_warning("volatility", 0));
        f.reset();
        f.update_market_warning(&make_warning("volatility", 1_000));
        assert_eq!(f.value() as i8, 1, "after reset, first update should emit");
    }

    #[test]
    fn factory_feeds_resolved_warning_frequency_filter() {
        let mut f = IndicatorOrder::WarningFrequencyFilter(WarningFrequencyFilterConfig {
            min_interval: Param::Solo(TimeWindow::Minutes(1)),
        })
        .build_solo()
        .unwrap();
        let w = MarketWarning {
            symbol: "BTCUSDT".to_string(),
            warning_kind: "volatility".to_string(),
            message: "test".to_string(),
            timestamp: 1_000,
        };
        f.feed(0, MarketSample::MarketWarning(&w));
        assert_eq!(f.primary() as i8, 1, "first warning should emit");
    }
}

impl WarningFrequencyFilter {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        (self.last_signal) as f64
    }
}
