//! Liquidation Rate — rolling count of liquidation events per second.
//!
//! Tracks how many liquidation events occurred within the last `window_ms`
//! milliseconds and expresses the density as events per second.
//!
//! Output: `Single(rate)` — events per second over the rolling window.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::liquidation_consumer::LiquidationConsumer;
use crate::engine::time_window::TimeWindow;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Family, Indicator, Output, SourceAxis};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::Liquidation;

/// Rolling liquidation event rate (events / second).
#[derive(Clone, Debug)]
pub struct LiquidationRate {
    /// Rolling window length in milliseconds.
    window_ms: i64,
    /// Timestamps of events still inside the window.
    events: VecDeque<i64>,
    /// Cached last computed rate.
    last_rate: f64,
}

impl LiquidationRate {
    /// Create with the given rolling window.
    ///
    /// `window_ms` — window size in milliseconds (e.g. `60_000` for 1 minute).
    pub fn new(window_ms: i64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            events: VecDeque::new(),
            last_rate: 0.0,
        }
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

use crate::contract::Param;

/// Typed dual-mode configuration for [`LiquidationRate`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LiquidationRateConfig {
    pub window: Param<TimeWindow>,
}

impl Indicator for LiquidationRate {
    const ID: IndicatorId = IndicatorId::LiquidationRate;
    // Liquidation-stream indicator (consumes `Liquidation` via update_liquidation).
    const FAMILY: &'static [Family] = &[Family::Liquidations];
    const INPUT: &'static [StreamKind] = &[StreamKind::Liquidation];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::count(IndicatorOutputId::LiquidationRate)];
    type Config = LiquidationRateConfig;
    type Runtime = LiquidationRate;

    fn create(cfg: LiquidationRateConfig) -> LiquidationRate {
        LiquidationRate::new(cfg.window.resolved().as_millis())
    }
}

impl LiquidationConsumer for LiquidationRate {
    fn update_liquidation(&mut self, liq: &Liquidation) {
        self.events.push_back(liq.timestamp);
        self.evict(liq.timestamp);
        let span_seconds = self.window_ms as f64 / 1_000.0;
        self.last_rate = self.events.len() as f64 / span_seconds;
    }


    fn reset(&mut self) {
        self.events.clear();
        self.last_rate = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

impl crate::contract::Config for LiquidationRateConfig {
    fn defaults() -> Self {
        LiquidationRateConfig {
            window: Param::Solo(TimeWindow::Seconds(60)),
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
        s
    }
}


impl Render for LiquidationRate {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::LiquidationRate, "Liquidation Rate", Color::hex(0xF44336))
            .precision(2)
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
    fn rate_zero_initially() {
        let lr = LiquidationRate::new(60_000);
        assert!(!lr.is_ready());
        assert_eq!(lr.value(), 0.0);
    }

    #[test]
    fn rate_after_single_event() {
        let mut lr = LiquidationRate::new(60_000);
        lr.update_liquidation(&liq(0));
        assert!(lr.is_ready());
        // 1 event in 60 s = 1/60 ≈ 0.01667 events/s
        let r = lr.value();
        assert!((r - 1.0 / 60.0).abs() < 1e-9);
    }

    #[test]
    fn old_events_evicted() {
        let mut lr = LiquidationRate::new(10_000); // 10 s window
        lr.update_liquidation(&liq(0));
        // event at t=0; update at t=15000 — event is outside 10 s window
        lr.update_liquidation(&liq(15_000));
        // only the t=15_000 event should remain
        let r = lr.value();
        let expected = 1.0 / 10.0; // 1 event in 10 s
        assert!((r - expected).abs() < 1e-9, "rate={r}");
    }

    #[test]
    fn reset_clears_state() {
        let mut lr = LiquidationRate::new(60_000);
        lr.update_liquidation(&liq(0));
        lr.reset();
        assert!(!lr.is_ready());
        assert_eq!(lr.value(), 0.0);
    }
}

impl LiquidationRate {
    /// Primary scalar output (relocated from the stream-consumer impl).
    pub fn value(&self) -> f64 {
        self.last_rate
    }
}
