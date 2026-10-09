//! WarningRate — rolling rate of market warning events per minute.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::market_warning_consumer::MarketWarningConsumer;
use crate::engine::time_window::TimeWindow;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::MarketWarning;

/// Rolling rate of market warning events per minute within a sliding time window.
///
/// `warnings_per_min = event_count_in_window / window_minutes`
///
/// Output: `Single(warnings_per_min)`.
///
/// Default window: 5 minutes (300_000 ms).
#[derive(Debug, Clone)]
pub struct WarningRate {
    window_ms: i64,
    events: VecDeque<i64>,
    last_rate: f64,
}

impl WarningRate {
    /// Create a new indicator with explicit window in milliseconds.
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            events: VecDeque::new(),
            last_rate: 0.0,
        }
    }

    fn evict_old(&mut self, current_ts: i64) {
        let cutoff = current_ts - self.window_ms;
        while self.events.front().map_or(false, |&ts| ts < cutoff) {
            self.events.pop_front();
        }
    }

    fn compute_rate(&self) -> f64 {
        let window_minutes = self.window_ms as f64 / 60_000.0;
        self.events.len() as f64 / window_minutes
    }
}

impl Default for WarningRate {
    fn default() -> Self {
        Self::new(300_000)
    }
}

impl MarketWarningConsumer for WarningRate {
    fn update_market_warning(&mut self, w: &MarketWarning) {
        self.evict_old(w.timestamp);
        self.events.push_back(w.timestamp);
        self.last_rate = self.compute_rate();
    }


    fn reset(&mut self) {
        self.events.clear();
        self.last_rate = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

/// Typed configuration for [`WarningRate`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct WarningRateConfig {
    pub window: Param<TimeWindow>,
}

impl Indicator for WarningRate {
    const ID: IndicatorId = IndicatorId::WarningRate;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::MarketWarning];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::count(IndicatorOutputId::WarningRate)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    type Config = WarningRateConfig;
    type Runtime = WarningRate;

    fn create(cfg: WarningRateConfig) -> WarningRate {
        WarningRate::new(cfg.window.resolved().as_millis())
    }
}

impl crate::contract::Config for WarningRateConfig {
    fn defaults() -> Self {
        WarningRateConfig {
            window: Param::Solo(TimeWindow::Minutes(5)),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // Class N — TimeWindow: 11-value curated discrete set spanning 1s..24h
        s.window = Param::many(vec![
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


impl Render for WarningRate {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::WarningRate, "Warning Rate", Color::hex(0xFF5722))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    fn make_warning(timestamp: i64) -> MarketWarning {
        MarketWarning {
            symbol: "BTCUSDT".to_string(),
            warning_kind: "high_volatility".to_string(),
            message: "test".to_string(),
            timestamp,
        }
    }

    #[test]
    fn rate_increases_with_more_events() {
        let window_ms = 60_000i64; // 1 minute
        let mut ind = WarningRate::new(window_ms);
        // 3 events in 1-minute window = 3/min
        ind.update_market_warning(&make_warning(1000));
        ind.update_market_warning(&make_warning(2000));
        ind.update_market_warning(&make_warning(3000));
        let r = ind.value();
        assert!((r - 3.0).abs() < 1e-9, "rate = {r}");
    }

    #[test]
    fn old_events_evicted() {
        let window_ms = 60_000i64;
        let mut ind = WarningRate::new(window_ms);
        // Add event at t=0
        ind.update_market_warning(&make_warning(0));
        // Add event well outside window (t = 2 minutes later)
        ind.update_market_warning(&make_warning(120_001));
        // Only 1 event should remain in window
        // 1 event / 1 min = 1.0 rate
        let r = ind.value();
        assert!((r - 1.0).abs() < 1e-9, "rate = {r}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = WarningRate::new(60_000);
        ind.update_market_warning(&make_warning(1000));
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_warning_rate() {
        let mut f = IndicatorOrder::WarningRate(WarningRateConfig {
            window: Param::Solo(TimeWindow::Minutes(1)),
        })
        .build_solo()
        .unwrap();
        let w1 = MarketWarning {
            symbol: "BTCUSDT".to_string(),
            warning_kind: "volatility".to_string(),
            message: "test".to_string(),
            timestamp: 1_000,
        };
        let w2 = MarketWarning {
            symbol: "BTCUSDT".to_string(),
            warning_kind: "margin_call".to_string(),
            message: "test".to_string(),
            timestamp: 2_000,
        };
        f.feed(0, MarketSample::MarketWarning(&w1));
        f.feed(0, MarketSample::MarketWarning(&w2));
        assert!(f.primary() > 0.0, "rate should be positive after events");
    }
}

impl WarningRate {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_rate
    }
}
