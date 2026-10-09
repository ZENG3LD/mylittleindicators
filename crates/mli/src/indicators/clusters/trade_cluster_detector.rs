//! TradeClusterDetector — detects series of trades at the same price level.
//!
//! Multiple trades at the same price bucket within a time window suggest
//! hidden iceberg orders or large institutional interest at that level.
//!
//! Outputs: `signal`, `cluster_price`, `cluster_size`
//!   signal: +1.0 = buy cluster, -1.0 = sell cluster, 0.0 = none

use std::collections::VecDeque;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::Tick;

/// Detects repeated trades at the same price bucket (iceberg / cluster signal).
#[derive(Debug, Clone)]
pub struct TradeClusterDetector {
    /// Price bucket size for rounding (e.g. 0.01 = 1 cent buckets).
    price_bucket: f64,
    /// Minimum ticks at same price level to declare a cluster.
    cluster_threshold: usize,
    /// Time window in milliseconds.
    window_ms: i64,
    /// Ring buffer of (price_bucket_id, timestamp_ms, is_buy, size).
    recent_ticks: VecDeque<(i64, i64, bool, f64)>,
    last_cluster_price: f64,
    last_cluster_size: f64,
    last_signal: f64,
}

impl TradeClusterDetector {
    /// Create detector.
    ///
    /// - `price_bucket`: rounding step for price levels (e.g. 0.01).
    /// - `cluster_threshold`: min ticks at same price bucket to trigger.
    /// - `window_ms`: rolling time window in milliseconds.
    pub fn new(price_bucket: f64, cluster_threshold: usize, window_ms: i64) -> Self {
        let bucket = if price_bucket > 0.0 { price_bucket } else { 0.01 };
        Self {
            price_bucket: bucket,
            cluster_threshold: cluster_threshold.max(2),
            window_ms: window_ms.max(1),
            recent_ticks: VecDeque::with_capacity(256),
            last_cluster_price: 0.0,
            last_cluster_size: 0.0,
            last_signal: 0.0,
        }
    }
}

impl TradeClusterDetector {
    /// Named output: brace `signal`.
    pub fn signal(&self) -> f64 { self.last_signal }
    /// Named output: brace `cluster_price`.
    pub fn cluster_price(&self) -> f64 { self.last_cluster_price }
    /// Named output: brace `cluster_size`.
    pub fn cluster_size(&self) -> f64 { self.last_cluster_size }
}

impl TickConsumer for TradeClusterDetector {
    fn update_tick(&mut self, tick: &Tick) {
        let bucket_id = (tick.price / self.price_bucket).floor() as i64;
        self.recent_ticks.push_back((bucket_id, tick.time, tick.is_buy, tick.size));

        // Evict ticks outside the time window.
        while let Some(&(_, ts, _, _)) = self.recent_ticks.front() {
            if tick.time - ts > self.window_ms {
                self.recent_ticks.pop_front();
            } else {
                break;
            }
        }

        // Count ticks in current bucket.
        let count = self.recent_ticks.iter()
            .filter(|&&(b, _, _, _)| b == bucket_id)
            .count();

        if count >= self.cluster_threshold {
            self.last_cluster_price = bucket_id as f64 * self.price_bucket;
            self.last_cluster_size = self.recent_ticks.iter()
                .filter(|&&(b, _, _, _)| b == bucket_id)
                .map(|&(_, _, _, s)| s)
                .sum();
            let buy_count = self.recent_ticks.iter()
                .filter(|&&(b, _, is_buy, _)| b == bucket_id && is_buy)
                .count();
            let sell_count = count - buy_count;
            self.last_signal = if buy_count > sell_count {
                1.0
            } else if sell_count > buy_count {
                -1.0
            } else {
                0.0
            };
        } else {
            self.last_signal = 0.0;
        }

    }


    fn reset(&mut self) {
        self.recent_ticks.clear();
        self.last_cluster_price = 0.0;
        self.last_cluster_size = 0.0;
        self.last_signal = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.recent_ticks.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::Tick;

    fn tick_at(price: f64, is_buy: bool, time_ms: i64) -> Tick {
        Tick::new(time_ms, price, 1.0, is_buy)
    }

    #[test]
    fn cluster_detected_when_threshold_reached() {
        // threshold=3, bucket=1.0, window=10000ms
        let mut det = TradeClusterDetector::new(1.0, 3, 10_000);
        // 3 buys at same price → cluster detected
        for i in 0..3 {
            det.update_tick(&tick_at(100.0, true, i as i64 * 100));
        }
        let signal = det.signal();
        let price = det.cluster_price();
        let size = det.cluster_size();
        assert!((signal - 1.0).abs() < 1e-9, "expected buy signal: {}", signal);
        assert!((price - 100.0).abs() < 1e-9, "expected price 100: {}", price);
        assert!((size - 3.0).abs() < 1e-9, "expected size 3: {}", size);
    }

    #[test]
    fn no_cluster_below_threshold() {
        let mut det = TradeClusterDetector::new(1.0, 5, 10_000);
        for i in 0..4 {
            det.update_tick(&tick_at(100.0, true, i as i64 * 100));
        }
        // signal should still be 0 — not enough ticks
        assert_eq!(det.signal(), 0.0);
        assert_eq!(det.cluster_price(), 0.0);
        assert_eq!(det.cluster_size(), 0.0);
    }

    #[test]
    fn old_ticks_evicted_by_time_window() {
        let mut det = TradeClusterDetector::new(1.0, 3, 1_000); // 1s window
        // 3 ticks within window
        det.update_tick(&tick_at(100.0, true, 0));
        det.update_tick(&tick_at(100.0, true, 500));
        det.update_tick(&tick_at(100.0, true, 900));
        assert!((det.signal() - 1.0).abs() < 1e-9);

        // New tick 2s later — all old ticks evicted
        det.update_tick(&tick_at(100.0, true, 2_100));
        // Only 1 tick left in window — no cluster
        assert_eq!(det.last_signal, 0.0);
    }

    #[test]
    fn reset_clears_state() {
        let mut det = TradeClusterDetector::new(1.0, 3, 10_000);
        for i in 0..3 {
            det.update_tick(&tick_at(100.0, true, i as i64 * 100));
        }
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.signal(), 0.0);
        assert_eq!(det.cluster_price(), 0.0);
        assert_eq!(det.cluster_size(), 0.0);
    }
}

impl Default for TradeClusterDetector {
    /// Factory default: price_bucket=0.01, cluster_threshold=3, window_ms=5000.
    fn default() -> Self {
        Self::new(0.01, 3, 5000)
    }
}

/// Typed dual-mode config for [`TradeClusterDetector`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct TradeClusterDetectorConfig {
    /// Price rounding step for bucket grouping (e.g. 0.01).
    pub price_bucket: Param<f64>,
    /// Minimum tick count at the same price bucket to declare a cluster (min 2).
    pub cluster_threshold: Param<usize>,
    /// Rolling time window in milliseconds.
    pub window_ms: Param<i64>,
}

impl Indicator for TradeClusterDetector {
    const ID: IndicatorId = IndicatorId::TradeClusterDetector;
    /// Not a pluggable family — a tick-stream iceberg/cluster detector.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::TradeClusterDetectorSignal),
        Output::price(IndicatorOutputId::TradeClusterDetectorClusterPrice),
        Output::count(IndicatorOutputId::TradeClusterDetectorClusterSize),
    ];
    /// O(n) per tick (counts matching buckets in the deque); rolling time-window VecDeque.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);
    type Config = TradeClusterDetectorConfig;
    type Runtime = Self;

    fn create(cfg: TradeClusterDetectorConfig) -> Self {
        Self::new(cfg.price_bucket.resolved(), cfg.cluster_threshold.resolved(), cfg.window_ms.resolved())
    }
}

impl crate::contract::Config for TradeClusterDetectorConfig {
    fn defaults() -> Self {
        TradeClusterDetectorConfig {
            price_bucket: Param::Solo(0.01),
            cluster_threshold: Param::Solo(3),
            window_ms: Param::Solo(5000),
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
        // price_bucket: Class I (instrument-relative price granularity) — PIN, leave Solo
        s.price_bucket = Self::defaults().price_bucket;
        // cluster_threshold: Class B count — override auto's 2..=4048 to 2..=20 step 1
        s.cluster_threshold = Param::range(2, 20, 1);
        // window_ms: Class M raw-ms — explicit TimeWindow-equivalent set in milliseconds
        s.window_ms = Param::many(vec![1000i64, 5000, 10000, 30000, 60000, 300000, 3600000]);
        s
    }
}


impl Render for TradeClusterDetector {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::TradeClusterDetectorSignal, "Signal", Color::hex(0xF44336))
            .line_output(IndicatorOutputId::TradeClusterDetectorClusterPrice, "Cluster Price", Color::hex(0xFF9800))
            .line_output(IndicatorOutputId::TradeClusterDetectorClusterSize, "Cluster Size", Color::hex(0x2196F3))
            .zero_baseline()
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;
    use crate::core::types::Tick as CoreTick;

    #[test]
    fn factory_feeds_resolved_tick() {
        let mut f = IndicatorOrder::TradeClusterDetector(
            <<TradeClusterDetector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();

        // 3 buy ticks at same price within window → cluster signal = +1
        for i in 0..3i64 {
            let t = CoreTick::new(i * 100, 100.0, 1.0, true);
            f.feed(0, MarketSample::Tick(&t));
        }
        // value() = signal field = +1.0
        let v = f.primary();
        assert!((v - 1.0).abs() < 1e-9, "expected buy cluster signal +1, got {}", v);
    }
}
