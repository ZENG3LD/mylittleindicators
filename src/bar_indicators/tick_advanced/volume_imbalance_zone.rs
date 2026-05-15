//! VolumeImbalanceZone — rolling buy/sell imbalance zone detector.
//!
//! Within a rolling `window_ms` window, computes `delta = buy_vol - sell_vol`
//! and `total_vol = buy_vol + sell_vol`. If `|delta| / total_vol > delta_threshold`
//! an imbalance zone is present. The zone bounds are the min/max price of the
//! dominant-side ticks inside the window.
//!
//! Output: `IndicatorValue::Triple(side, zone_low, zone_high)`.
//! - `side`: `+1.0` buy zone, `-1.0` sell zone, `0.0` neutral.
//! - `zone_low` / `zone_high`: price range of the dominant-side ticks.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

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

    /// Convenience constructor using the default 60 % threshold.
    pub fn with_window(window_ms: i64) -> Self {
        Self::new(window_ms, 0.6)
    }
}

impl TickConsumer for VolumeImbalanceZone {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
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
            return IndicatorValue::Triple(0.0, 0.0, 0.0);
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
            return IndicatorValue::Triple(0.0, 0.0, 0.0);
        }

        let delta = (buy_vol - sell_vol).abs() / total;
        if delta <= self.delta_threshold {
            self.last_side = 0.0;
            self.last_zone_low = 0.0;
            self.last_zone_high = 0.0;
            return IndicatorValue::Triple(0.0, 0.0, 0.0);
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

        IndicatorValue::Triple(side, zone_low, zone_high)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_side, self.last_zone_low, self.last_zone_high)
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
        if let IndicatorValue::Triple(side, low, high) = det.value() {
            assert!((side - 1.0).abs() < 1e-9, "side should be +1.0, got {side}");
            assert!(low <= high, "zone_low {low} > zone_high {high}");
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn balanced_volume_no_signal() {
        // 50/50 split → below threshold.
        let mut det = VolumeImbalanceZone::new(60_000, 0.6);
        for i in 0..5 {
            det.update_tick(&buy(i * 100, 100.0, 1.0));
            det.update_tick(&sell(i * 100 + 50, 100.0, 1.0));
        }
        if let IndicatorValue::Triple(side, _, _) = det.value() {
            assert!((side).abs() < 1e-9, "side should be 0.0 for balanced, got {side}");
        } else {
            panic!("expected Triple");
        }
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
        if let IndicatorValue::Triple(side, _, _) = det.value() {
            // Only 1 event: 100% buy → above 0.6 threshold.
            assert!((side - 1.0).abs() < 1e-9, "expected +1.0 after eviction, got {side}");
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut det = VolumeImbalanceZone::new(60_000, 0.6);
        det.update_tick(&buy(0, 100.0, 5.0));
        assert!(det.is_ready());
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.value(), IndicatorValue::Triple(0.0, 0.0, 0.0));
    }
}
