//! Liquidation Cascade — detect rapid bursts of forced liquidations.
//!
//! Triggers when the number of liquidation events within a rolling window
//! reaches or exceeds `threshold_count`.
//!
//! # Output
//! `Double(in_cascade, count)`
//!
//! - `in_cascade` — `1.0` when cascade is active, `0.0` otherwise.
//! - `count`      — number of events in the current window (as `f64`).

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::liquidation_consumer::LiquidationConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::time_window::TimeWindow;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::Liquidation;

/// Burst cascade detector for public liquidation streams.
#[derive(Clone, Debug)]
pub struct LiquidationCascade {
    /// Rolling window length in milliseconds.
    window_ms: i64,
    /// Number of events in window required to declare a cascade.
    threshold_count: usize,
    /// Timestamps of events still inside the window.
    events: VecDeque<i64>,
    /// Cached cascade flag.
    last_in_cascade: bool,
    /// Cached event count.
    last_count: usize,
}

impl Default for LiquidationCascade {
    fn default() -> Self {
        Self::new(10_000, 5)
    }
}

impl LiquidationCascade {
    /// Create with the given window and threshold.
    ///
    /// - `window_ms`       — rolling window in milliseconds.
    /// - `threshold_count` — minimum events in window to signal a cascade.
    pub fn new(window_ms: i64, threshold_count: usize) -> Self {
        Self {
            window_ms: window_ms.max(1),
            threshold_count: threshold_count.max(1),
            events: VecDeque::new(),
            last_in_cascade: false,
            last_count: 0,
        }
    }

    /// Cascade flag: `1.0` when a cascade is active, `0.0` otherwise.
    pub fn flag(&self) -> f64 {
        if self.last_in_cascade { 1.0 } else { 0.0 }
    }

    /// Number of liquidation events in the current rolling window.
    pub fn count(&self) -> f64 {
        self.last_count as f64
    }

    fn evict(&mut self, now: i64) {
        while let Some(&front) = self.events.front() {
            if now - front > self.window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }
    }
}

impl LiquidationConsumer for LiquidationCascade {
    fn update_liquidation(&mut self, liq: &Liquidation) {
        self.events.push_back(liq.timestamp);
        self.evict(liq.timestamp);
        self.last_count = self.events.len();
        self.last_in_cascade = self.last_count >= self.threshold_count;
    }


    fn reset(&mut self) {
        self.events.clear();
        self.last_in_cascade = false;
        self.last_count = 0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

use crate::contract::Param;

/// Typed dual-mode configuration for [`LiquidationCascade`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LiquidationCascadeConfig {
    /// Rolling window as a typed [`TimeWindow`].
    pub window: Param<TimeWindow>,
    /// Number of events within the window required to declare a cascade.
    pub threshold_count: Param<usize>,
}

impl Indicator for LiquidationCascade {
    const ID: IndicatorId = IndicatorId::LiquidationCascade;
    const FAMILY: &'static [Family] = &[Family::Liquidations];
    const INPUT: &'static [StreamKind] = &[StreamKind::Liquidation];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::LiquidationCascadeFlag),
        Output::count(IndicatorOutputId::LiquidationCascadeCount),
    ];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::window(StoreKind::Deque)]);
    type Config = LiquidationCascadeConfig;
    type Runtime = LiquidationCascade;

    fn create(cfg: LiquidationCascadeConfig) -> LiquidationCascade {
        LiquidationCascade::new(cfg.window.resolved().as_millis(), cfg.threshold_count.resolved())
    }
}

impl crate::contract::Config for LiquidationCascadeConfig {
    fn defaults() -> Self {
        LiquidationCascadeConfig {
            window: Param::Solo(TimeWindow::Seconds(10)),
            threshold_count: Param::Solo(5),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // window: Class N (TimeWindow enum) — 11-point curated discrete set.
        s.window = Param::many(vec![
            TimeWindow::Seconds(1), TimeWindow::Seconds(5), TimeWindow::Seconds(15),
            TimeWindow::Seconds(30), TimeWindow::Minutes(1), TimeWindow::Minutes(5),
            TimeWindow::Minutes(15), TimeWindow::Minutes(30),
            TimeWindow::Hours(1), TimeWindow::Hours(4), TimeWindow::Hours(24),
        ]);
        // threshold_count: Class B structural count — event burst threshold, 1..=20 step 1.
        // Auto would widen usize to 2..=4048; override to semantically bounded range.
        s.threshold_count = Param::range(1, 20, 1);
        s
    }
}


impl Render for LiquidationCascade {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::LiquidationCascadeFlag, "Cascade", Color::hex(0xFF5722))
            .line_output(IndicatorOutputId::LiquidationCascadeCount, "Count", Color::hex(0xFFA726))
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::TradeSide;

    fn liq(ts: i64) -> Liquidation {
        Liquidation { symbol: String::new(), side: TradeSide::Buy, price: 30_000.0, quantity: 0.1, timestamp: ts, value: None, ..Default::default()}
    }

    #[test]
    fn not_in_cascade_initially() {
        let lc = LiquidationCascade::new(10_000, 3);
        assert_eq!(lc.flag(), 0.0);
        assert_eq!(lc.count(), 0.0);
        assert!(!lc.is_ready());
    }

    #[test]
    fn below_threshold_no_cascade() {
        let mut lc = LiquidationCascade::new(10_000, 3);
        lc.update_liquidation(&liq(0));
        lc.update_liquidation(&liq(1_000));
        assert_eq!(lc.flag(), 0.0);
        assert_eq!(lc.count(), 2.0);
    }

    #[test]
    fn at_threshold_triggers_cascade() {
        let mut lc = LiquidationCascade::new(10_000, 3);
        lc.update_liquidation(&liq(0));
        lc.update_liquidation(&liq(1_000));
        lc.update_liquidation(&liq(2_000));
        assert_eq!(lc.flag(), 1.0, "should be in cascade");
        assert_eq!(lc.count(), 3.0);
    }

    #[test]
    fn cascade_drops_after_events_expire() {
        let mut lc = LiquidationCascade::new(5_000, 3);
        lc.update_liquidation(&liq(0));
        lc.update_liquidation(&liq(1_000));
        lc.update_liquidation(&liq(2_000));
        // cascade active
        assert_eq!(lc.flag(), 1.0);
        // new event 6 s later — all old events are outside 5 s window
        lc.update_liquidation(&liq(8_000));
        assert_eq!(lc.flag(), 0.0, "cascade should end");
        assert_eq!(lc.count(), 1.0);
    }

    #[test]
    fn reset_clears_state() {
        let mut lc = LiquidationCascade::new(10_000, 3);
        lc.update_liquidation(&liq(0));
        lc.update_liquidation(&liq(1_000));
        lc.update_liquidation(&liq(2_000));
        lc.reset();
        assert!(!lc.is_ready());
        assert_eq!(lc.flag(), 0.0);
        assert_eq!(lc.count(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_liquidation_cascade() {
        use crate::engine::contract_engine::IndicatorOrder;
        
        let mut f = IndicatorOrder::LiquidationCascade(<<LiquidationCascade as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        let data = liq(1_000);
        f.feed(0, crate::contract::MarketSample::Liquidation(&data));
        f.feed(0, crate::contract::MarketSample::Liquidation(&liq(2_000)));
        f.feed(0, crate::contract::MarketSample::Liquidation(&liq(3_000)));
        f.feed(0, crate::contract::MarketSample::Liquidation(&liq(4_000)));
        f.feed(0, crate::contract::MarketSample::Liquidation(&liq(5_000)));
        // 5 events within 10s window with threshold_count=5 → cascade active; f.value() = first output (flag)
        assert_eq!(f.primary(), 1.0, "cascade should be active");
    }
}
