//! VolumeImbalanceZone — rolling buy/sell imbalance zone detector.
//!
//! Within a rolling `window_ms` window, computes `delta = buy_vol - sell_vol`
//! and `total_vol = buy_vol + sell_vol`. If `|delta| / total_vol > delta_threshold`
//! an imbalance zone is present. The zone bounds are the min/max price of the
//! dominant-side ticks inside the window.
//!
//! Outputs: `side`, `zone_low`, `zone_high`.
//! - `side`: `+1.0` buy zone, `-1.0` sell zone, `0.0` neutral.
//! - `zone_low` / `zone_high`: price range of the dominant-side ticks.

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::engine::time_window::TimeWindow;
use crate::contract::{Family, Indicator, Output, SourceAxis};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::core::types::Tick;
use crate::engine::stream_kind::StreamKind;

/// Rolling buy/sell volume imbalance zone detector.
///
/// Parameters:
/// - `window_ms`       — rolling time window in milliseconds.
/// - `delta_threshold` — minimum `|delta_vol| / total_vol` ratio to signal an
///                       imbalance zone (default 0.6).
#[derive(Debug, Clone)]
pub struct VolumeImbalanceZone {
    window_ms: i64,
    delta_threshold: f64,
    /// `(timestamp_ms, price, size, is_buy)`
    events: VecDeque<(i64, f64, f64, bool)>,
    last_side: f64,
    last_zone_low: f64,
    last_zone_high: f64,
}

impl VolumeImbalanceZone {
    /// Create a new detector.
    ///
    /// - `window_ms`       — rolling window in milliseconds (clamped ≥ 1).
    /// - `delta_threshold` — imbalance ratio threshold, clamped to (0, 1].
    pub fn new(window_ms: i64, delta_threshold: f64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            delta_threshold: delta_threshold.clamp(f64::EPSILON, 1.0),
            events: VecDeque::with_capacity(512),
            last_side: 0.0,
            last_zone_low: 0.0,
            last_zone_high: 0.0,
        }
    }

    /// Imbalance zone side: `+1.0` (buy), `-1.0` (sell), `0.0` (neutral).
    pub fn side(&self) -> f64 {
        self.last_side
    }

    /// Lower price bound of the imbalance zone.
    pub fn low(&self) -> f64 {
        self.last_zone_low
    }

    /// Upper price bound of the imbalance zone.
    pub fn high(&self) -> f64 {
        self.last_zone_high
    }

    /// Convenience constructor using the default 60 % threshold.
    pub fn with_window(window_ms: i64) -> Self {
        Self::new(window_ms, 0.6)
    }
}

impl TickConsumer for VolumeImbalanceZone {
    fn update_tick(&mut self, tick: &Tick) {
        self.events.push_back((tick.time, tick.price, tick.size, tick.is_buy));

        // Evict stale events.
        while let Some(&(ts, _, _, _)) = self.events.front() {
            if tick.time - ts > self.window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }

        if self.events.is_empty() {
            return;
        }

        let mut buy_vol = 0.0_f64;
        let mut sell_vol = 0.0_f64;
        for &(_, _, sz, is_buy) in &self.events {
            if is_buy {
                buy_vol += sz;
            } else {
                sell_vol += sz;
            }
        }
        let total = buy_vol + sell_vol;

        if total <= f64::EPSILON {
            self.last_side = 0.0;
            self.last_zone_low = 0.0;
            self.last_zone_high = 0.0;
            return;
        }

        let delta = (buy_vol - sell_vol).abs() / total;
        if delta <= self.delta_threshold {
            self.last_side = 0.0;
            self.last_zone_low = 0.0;
            self.last_zone_high = 0.0;
            return;
        }

        let dominant_buy = buy_vol >= sell_vol;
        let side = if dominant_buy { 1.0_f64 } else { -1.0_f64 };

        // Zone bounds = min/max price of dominant-side ticks in window.
        let (zone_low, zone_high) = self
            .events
            .iter()
            .filter(|&&(_, _, _, is_buy)| is_buy == dominant_buy)
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), &(_, p, _, _)| {
                (lo.min(p), hi.max(p))
            });

        let zone_low = if zone_low == f64::INFINITY { 0.0 } else { zone_low };
        let zone_high = if zone_high == f64::NEG_INFINITY { 0.0 } else { zone_high };

        self.last_side = side;
        self.last_zone_low = zone_low;
        self.last_zone_high = zone_high;

    }


    fn reset(&mut self) {
        self.events.clear();
        self.last_side = 0.0;
        self.last_zone_low = 0.0;
        self.last_zone_high = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buy(time_ms: i64, price: f64, size: f64) -> Tick {
        Tick::new(time_ms, price, size, true)
    }

    fn sell(time_ms: i64, price: f64, size: f64) -> Tick {
        Tick::new(time_ms, price, size, false)
    }

    #[test]
    fn buy_imbalance_detected() {
        // 90 % buy volume → clearly above 0.6 threshold.
        let mut det = VolumeImbalanceZone::new(60_000, 0.6);
        for i in 0..9 {
            det.update_tick(&buy(i * 100, 100.0 + i as f64, 1.0));
        }
        det.update_tick(&sell(1_000, 95.0, 1.0));
        let side = det.side();
        let low = det.low();
        let high = det.high();
        assert!((side - 1.0).abs() < 1e-9, "side should be +1.0, got {side}");
        assert!(low <= high, "zone_low {low} > zone_high {high}");
    }

    #[test]
    fn balanced_volume_no_signal() {
        // 50/50 split → below threshold.
        let mut det = VolumeImbalanceZone::new(60_000, 0.6);
        for i in 0..5 {
            det.update_tick(&buy(i * 100, 100.0, 1.0));
            det.update_tick(&sell(i * 100 + 50, 100.0, 1.0));
        }
        let side = det.side();
        assert!((side).abs() < 1e-9, "side should be 0.0 for balanced, got {side}");
    }

    #[test]
    fn stale_events_evicted() {
        let mut det = VolumeImbalanceZone::new(5_000, 0.6);
        // Sell-heavy at t=0..4
        for i in 0..5 {
            det.update_tick(&sell(i * 100, 100.0, 10.0));
        }
        // Now only a buy tick 10 s later — old events evicted, single buy → buy zone.
        det.update_tick(&buy(10_000, 200.0, 1.0));
        // Only 1 event: 100% buy → above 0.6 threshold.
        let side = det.side();
        assert!((side - 1.0).abs() < 1e-9, "expected +1.0 after eviction, got {side}");
    }

    #[test]
    fn reset_clears_state() {
        let mut det = VolumeImbalanceZone::new(60_000, 0.6);
        det.update_tick(&buy(0, 100.0, 5.0));
        assert!(det.is_ready());
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.side(), 0.0);
        assert_eq!(det.low(), 0.0);
        assert_eq!(det.high(), 0.0);
    }
}

impl Default for VolumeImbalanceZone {
    /// Factory default: window_ms=60000, delta_threshold=0.6.
    fn default() -> Self {
        Self::new(60_000, 0.6)
    }
}

// ---- Indicator contract ----

use crate::contract::{Param, sweep_f64};

/// Typed configuration for [`VolumeImbalanceZone`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VolumeImbalanceZoneConfig {
    pub window: Param<TimeWindow>,
    /// Minimum `|delta_vol| / total_vol` to signal an imbalance zone.
    pub delta_threshold: Param<f64>,
}

impl Indicator for VolumeImbalanceZone {
    const ID: IndicatorId = IndicatorId::VolumeImbalanceZone;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::VolumeImbalanceZoneSide),
        Output::price(IndicatorOutputId::VolumeImbalanceZoneLow),
        Output::price(IndicatorOutputId::VolumeImbalanceZoneHigh),
    ];
    type Config = VolumeImbalanceZoneConfig;
    type Runtime = VolumeImbalanceZone;

    fn create(cfg: VolumeImbalanceZoneConfig) -> VolumeImbalanceZone {
        VolumeImbalanceZone::new(cfg.window.resolved().as_millis(), cfg.delta_threshold.resolved())
    }
}

impl crate::contract::Config for VolumeImbalanceZoneConfig {
    fn defaults() -> Self {
        VolumeImbalanceZoneConfig {
            window: Param::Solo(TimeWindow::Minutes(1)),
            delta_threshold: Param::Solo(0.6),
        }
    }
    fn machine_defaults() -> Self {
        // window (TimeWindow): Class N — 11-point curated discrete set.
        // delta_threshold (f64): Class F threshold — |delta_vol|/total_vol ratio in (0,1];
        //   internally clamped to (0,1]. Swept as Class F 0.1..5.0 step 0.1; values >1.0 are
        //   clamped to 1.0 by the constructor. Flagged: effective range 0.1..=1.0.
        let mut s = Self::machine_defaults_auto();
        s.window = Param::many(vec![
            TimeWindow::Seconds(1), TimeWindow::Seconds(5), TimeWindow::Seconds(15),
            TimeWindow::Seconds(30), TimeWindow::Minutes(1), TimeWindow::Minutes(5),
            TimeWindow::Minutes(15), TimeWindow::Minutes(30),
            TimeWindow::Hours(1), TimeWindow::Hours(4), TimeWindow::Hours(24),
        ]);
        s.delta_threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VolumeImbalanceZone {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::VolumeImbalanceZoneSide, "Side", Color::hex(0x26C6DA), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::VolumeImbalanceZoneLow, "Zone Low", Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::VolumeImbalanceZoneHigh, "Zone High", Color::hex(0x4CAF50), 1.0))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_volume_imbalance_zone() {
        let mut f = IndicatorOrder::VolumeImbalanceZone(
            <<VolumeImbalanceZone as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let t = crate::core::types::Tick::new(0, 100.0, 1.0, true);
        f.feed(0, MarketSample::Tick(&t));
        assert!(f.primary().is_finite());
    }
}
