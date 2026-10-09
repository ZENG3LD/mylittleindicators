//! Level Replenishment Rate — rolling rate of positive-size orderbook updates per second.
//!
//! Counts updated (non-zero) levels from delta updates in a rolling event window
//! and computes events per second over the time span of that window.
//!
//! High rate = active market makers aggressively quoting/refreshing.
//! Low rate = passive or thinning book.
//!
//! Output: `Single(events_per_second)`.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::orderbook_delta_consumer::OrderbookDeltaConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::OrderbookDelta;

/// Rolling rate of orderbook level replenishments (positive-size updates).
#[derive(Clone, Debug)]
pub struct LevelReplenishmentRate {
    /// Max number of events retained in rolling window.
    rolling_window: usize,
    /// Ring of (timestamp_ms, price) for each replenishment event.
    events: VecDeque<(i64, f64)>,
    /// Last computed rate (events per second).
    last_rate: f64,
}

impl LevelReplenishmentRate {
    /// Create with given rolling window size (event count).
    pub fn new(rolling_window: usize) -> Self {
        Self {
            rolling_window: rolling_window.max(2),
            events: VecDeque::new(),
            last_rate: 0.0,
        }
    }

    #[inline]
    fn compute_rate(&self) -> f64 {
        let count = self.events.len();
        if count < 2 {
            return count as f64;
        }
        let oldest_ts = self.events.front().map(|e| e.0).unwrap_or(0);
        let newest_ts = self.events.back().map(|e| e.0).unwrap_or(0);
        let span_ms = (newest_ts - oldest_ts).max(1);
        let span_sec = span_ms as f64 / 1000.0;
        count as f64 / span_sec
    }
}

impl Default for LevelReplenishmentRate {
    fn default() -> Self {
        Self::new(100)
    }
}

impl OrderbookDeltaConsumer for LevelReplenishmentRate {
    fn update_delta(&mut self, delta: &OrderbookDelta) {
        // Count all positive-size updates as replenishment events
        for level in delta.updated_bids().chain(delta.updated_asks()) {
            self.events.push_back((delta.timestamp, level.price));
        }
        // Trim to window size
        while self.events.len() > self.rolling_window {
            self.events.pop_front();
        }
        self.last_rate = self.compute_rate();
    }


    fn reset(&mut self) {
        self.events.clear();
        self.last_rate = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.events.len() >= 2
    }
}

/// Typed configuration for [`LevelReplenishmentRate`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LevelReplenishmentRateConfig {
    /// Rolling event window size (number of events to retain).
    pub rolling_window: crate::contract::Param<usize>,
}

impl Indicator for LevelReplenishmentRate {
    const ID: IndicatorId = IndicatorId::LevelReplenishRate;
    /// Orderbook delta level-replenishment rate indicator.
    const FAMILY: &'static [Family] = &[Family::OrderBook];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderbookDelta];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::count(IndicatorOutputId::LevelReplenishRate)];
    /// O(updates_per_delta) per update — scans all updated levels.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = LevelReplenishmentRateConfig;
    type Runtime = LevelReplenishmentRate;

    fn create(cfg: LevelReplenishmentRateConfig) -> LevelReplenishmentRate {
        LevelReplenishmentRate::new(cfg.rolling_window.resolved())
    }
}

impl crate::contract::Config for LevelReplenishmentRateConfig {
    fn defaults() -> Self {
        LevelReplenishmentRateConfig { rolling_window: crate::contract::Param::Solo(100) }
    }
    fn machine_defaults() -> Self {
        // rolling_window: Class A (event window) — auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for LevelReplenishmentRate {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::LevelReplenishRate, "Level Replenish Rate", Color::hex(0x4CAF50))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::OrderBookLevel;
    use crate::contract::market_sample::MarketSample;
    use crate::engine::contract_engine::IndicatorOrder;

    fn make_delta(updates: &[(f64, f64)], ts: i64) -> OrderbookDelta {
        OrderbookDelta {
            bids: updates.iter().map(|&(p, s)| OrderBookLevel::new(p, s)).collect(),
            asks: vec![],
            timestamp: ts,
            first_update_id: None,
            last_update_id: None,
            prev_update_id: None,
            ..Default::default()
        }
    }

    #[test]
    fn not_ready_with_single_event() {
        let mut rate = LevelReplenishmentRate::new(50);
        rate.update_delta(&make_delta(&[(100.0, 5.0)], 1000));
        assert!(!rate.is_ready());
    }

    #[test]
    fn ready_with_two_events() {
        let mut rate = LevelReplenishmentRate::new(50);
        rate.update_delta(&make_delta(&[(100.0, 5.0)], 1000));
        rate.update_delta(&make_delta(&[(101.0, 5.0)], 2000));
        assert!(rate.is_ready());
    }

    #[test]
    fn rate_is_positive_with_data() {
        let mut rate = LevelReplenishmentRate::new(50);
        // 3 events over 2 seconds = 1.5 per second
        rate.update_delta(&make_delta(&[(100.0, 5.0)], 0));
        rate.update_delta(&make_delta(&[(101.0, 5.0)], 1000));
        rate.update_delta(&make_delta(&[(102.0, 5.0)], 2000));
        assert!(rate.is_ready());
        let v = rate.value();
        assert!(v > 0.0, "rate should be positive");
        assert!(v.is_finite(), "rate should be finite");
    }

    #[test]
    fn window_trims_old_events() {
        let mut rate = LevelReplenishmentRate::new(3);
        for i in 0..10 {
            rate.update_delta(&make_delta(&[(100.0 + i as f64, 5.0)], i * 1000));
        }
        assert!(rate.events.len() <= 3);
    }

    #[test]
    fn reset_clears_state() {
        let mut rate = LevelReplenishmentRate::new(50);
        rate.update_delta(&make_delta(&[(100.0, 5.0)], 1000));
        rate.update_delta(&make_delta(&[(101.0, 5.0)], 2000));
        rate.reset();
        assert!(!rate.is_ready());
        assert_eq!(rate.value(), 0.0);
    }

    #[test]
    fn removals_not_counted() {
        let mut rate = LevelReplenishmentRate::new(50);
        // Only removals (size=0) — should not increase event count
        rate.update_delta(&make_delta(&[(100.0, 0.0), (101.0, 0.0)], 1000));
        assert!(!rate.is_ready());
    }

    #[test]
    fn factory_feeds_resolved_level_replenishment_rate() {
        let mut f = IndicatorOrder::LevelReplenishRate(
            <<LevelReplenishmentRate as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let d1 = make_delta(&[(100.0, 5.0)], 0);
        let d2 = make_delta(&[(101.0, 5.0)], 1000);
        f.feed(0, MarketSample::OrderbookDelta(&d1));
        f.feed(0, MarketSample::OrderbookDelta(&d2));
        assert!(f.is_ready());
        assert!(f.primary() > 0.0);
    }
}

impl LevelReplenishmentRate {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_rate
    }
}
