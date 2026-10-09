//! Liquidation Cluster Detector — detects price levels with concentrated
//! liquidation activity.
//!
//! # Algorithm
//!
//! Events `(timestamp, bucket_key, quote_value)` are stored in a rolling
//! `VecDeque`. On each update:
//! 1. Old events outside `window_ms` are evicted.
//! 2. Remaining events are grouped by `floor(price / price_bucket)`.
//! 3. The bucket with the highest event count is returned if its count
//!    reaches `min_cluster_count`.
//!
//! # Output
//! `Triple(cluster_price, cluster_count, cluster_volume)`:
//! - `cluster_price`  — representative price of the dominant bucket
//!   (`bucket_key * price_bucket + price_bucket / 2`).
//! - `cluster_count`  — number of events in the dominant bucket (as `f64`).
//! - `cluster_volume` — total USD volume in the dominant bucket.
//!
//! All three are `0.0` when no cluster meets `min_cluster_count`.
//!
//! # Parameters
//! - `price_bucket`      — price granularity for grouping (e.g. 100.0 for BTC).
//! - `window_ms`         — rolling window in milliseconds.
//! - `min_cluster_count` — minimum events in a bucket to declare a cluster.

use std::collections::{HashMap, VecDeque};

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::liquidation_consumer::LiquidationConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::time_window::TimeWindow;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::Liquidation;

/// Detects price-level clusters of liquidation activity.
#[derive(Clone, Debug)]
pub struct LiquidationClusterDetector {
    /// Price bucket size for grouping.
    price_bucket: f64,
    /// Rolling window length in milliseconds.
    window_ms: i64,
    /// Minimum events in a bucket to declare a cluster.
    min_cluster_count: usize,
    /// Buffered events: `(timestamp_ms, bucket_key, quote_value)`.
    events: VecDeque<(i64, i64, f64)>,
    /// Cached output.
    last_cluster_price: f64,
    last_cluster_count: f64,
    last_cluster_volume: f64,
}

impl Default for LiquidationClusterDetector {
    fn default() -> Self {
        Self::new(100.0, 60_000, 3)
    }
}

impl LiquidationClusterDetector {
    /// Create a new detector.
    ///
    /// - `price_bucket`      — width of each price level bucket.
    /// - `window_ms`         — look-back window in milliseconds.
    /// - `min_cluster_count` — threshold to declare a cluster.
    pub fn new(price_bucket: f64, window_ms: i64, min_cluster_count: usize) -> Self {
        Self {
            price_bucket: price_bucket.max(f64::MIN_POSITIVE),
            window_ms: window_ms.max(1),
            min_cluster_count: min_cluster_count.max(1),
            events: VecDeque::new(),
            last_cluster_price: 0.0,
            last_cluster_count: 0.0,
            last_cluster_volume: 0.0,
        }
    }

    /// Representative price of the dominant liquidation cluster bucket.
    pub fn price(&self) -> f64 {
        self.last_cluster_price
    }

    /// Number of events in the dominant cluster bucket.
    pub fn count(&self) -> f64 {
        self.last_cluster_count
    }

    /// Total USD volume in the dominant cluster bucket.
    pub fn volume(&self) -> f64 {
        self.last_cluster_volume
    }

    fn evict(&mut self, now: i64) {
        while let Some(&(ts, _, _)) = self.events.front() {
            if now - ts > self.window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }
    }

    fn price_to_bucket(&self, price: f64) -> i64 {
        (price / self.price_bucket).floor() as i64
    }

    fn recompute(&mut self) {
        // Aggregate per bucket.
        let mut bucket_count: HashMap<i64, usize> = HashMap::new();
        let mut bucket_volume: HashMap<i64, f64> = HashMap::new();
        for &(_, bucket, vol) in &self.events {
            *bucket_count.entry(bucket).or_insert(0) += 1;
            *bucket_volume.entry(bucket).or_insert(0.0) += vol;
        }

        // Find bucket with max count.
        let best = bucket_count
            .iter()
            .max_by_key(|&(_, &cnt)| cnt);

        if let Some((&bucket_key, &count)) = best {
            if count >= self.min_cluster_count {
                let vol = bucket_volume.get(&bucket_key).copied().unwrap_or(0.0);
                // Mid-point of the bucket as representative price.
                let cluster_price = bucket_key as f64 * self.price_bucket + self.price_bucket * 0.5;
                self.last_cluster_price = cluster_price;
                self.last_cluster_count = count as f64;
                self.last_cluster_volume = vol;
                return;
            }
        }

        self.last_cluster_price = 0.0;
        self.last_cluster_count = 0.0;
        self.last_cluster_volume = 0.0;
    }
}

impl LiquidationConsumer for LiquidationClusterDetector {
    fn update_liquidation(&mut self, liq: &Liquidation) {
        let bucket = self.price_to_bucket(liq.price);
        self.events.push_back((liq.timestamp, bucket, liq.quote_value()));
        self.evict(liq.timestamp);
        self.recompute();
    }


    fn reset(&mut self) {
        self.events.clear();
        self.last_cluster_price = 0.0;
        self.last_cluster_count = 0.0;
        self.last_cluster_volume = 0.0;
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

use crate::contract::Param;

/// Typed dual-mode configuration for [`LiquidationClusterDetector`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LiquidationClusterDetectorConfig {
    /// Price bucket width for grouping (e.g. 100.0 for BTC).
    pub price_bucket: Param<f64>,
    /// Rolling window as a typed [`TimeWindow`].
    pub window: Param<TimeWindow>,
    /// Minimum events in a bucket to declare a cluster.
    pub min_cluster_count: Param<usize>,
}

impl Indicator for LiquidationClusterDetector {
    const ID: IndicatorId = IndicatorId::LiquidationClusterDetector;
    const FAMILY: &'static [Family] = &[Family::Liquidations];
    const INPUT: &'static [StreamKind] = &[StreamKind::Liquidation];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::LiquidationClusterDetectorPrice),
        Output::count(IndicatorOutputId::LiquidationClusterDetectorCount),
        Output::count(IndicatorOutputId::LiquidationClusterDetectorVolume),
    ];
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);
    type Config = LiquidationClusterDetectorConfig;
    type Runtime = LiquidationClusterDetector;

    fn create(cfg: LiquidationClusterDetectorConfig) -> LiquidationClusterDetector {
        LiquidationClusterDetector::new(cfg.price_bucket.resolved(), cfg.window.resolved().as_millis(), cfg.min_cluster_count.resolved())
    }
}

impl crate::contract::Config for LiquidationClusterDetectorConfig {
    fn defaults() -> Self {
        LiquidationClusterDetectorConfig {
            price_bucket: Param::Solo(100.0),
            window: Param::Solo(TimeWindow::Minutes(1)),
            min_cluster_count: Param::Solo(3),
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
        // price_bucket: Class I (instrument-relative price granularity) — PIN Solo.
        // f64 auto already leaves Solo; restore defaults value explicitly for safety.
        s.price_bucket = Self::defaults().price_bucket;
        // window: Class N (TimeWindow enum) — 11-point curated discrete set.
        s.window = Param::many(vec![
            TimeWindow::Seconds(1), TimeWindow::Seconds(5), TimeWindow::Seconds(15),
            TimeWindow::Seconds(30), TimeWindow::Minutes(1), TimeWindow::Minutes(5),
            TimeWindow::Minutes(15), TimeWindow::Minutes(30),
            TimeWindow::Hours(1), TimeWindow::Hours(4), TimeWindow::Hours(24),
        ]);
        // min_cluster_count: Class B structural count — cluster event threshold, 1..=20 step 1.
        s.min_cluster_count = Param::range(1, 20, 1);
        s
    }
}


impl Render for LiquidationClusterDetector {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::LiquidationClusterDetectorPrice, "Cluster Price", Color::hex(0xE91E63))
            .line_output(IndicatorOutputId::LiquidationClusterDetectorCount, "Cluster Count", Color::hex(0xFF9800))
            .line_output(IndicatorOutputId::LiquidationClusterDetectorVolume, "Cluster Volume", Color::hex(0x9C27B0))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::TradeSide;

    fn liq(ts: i64, price: f64, qty: f64) -> Liquidation {
        Liquidation { symbol: String::new(), side: TradeSide::Buy, price, quantity: qty, timestamp: ts, value: None, ..Default::default()}
    }

    #[test]
    fn no_cluster_initially() {
        let lcd = LiquidationClusterDetector::new(100.0, 60_000, 3);
        assert_eq!(lcd.price(), 0.0);
        assert_eq!(lcd.count(), 0.0);
        assert_eq!(lcd.volume(), 0.0);
        assert!(!lcd.is_ready());
    }

    #[test]
    fn cluster_detected_at_threshold() {
        // price_bucket=100, window=60s, min_count=3.
        // Three events all in the [30_000, 30_100) bucket.
        let mut lcd = LiquidationClusterDetector::new(100.0, 60_000, 3);
        lcd.update_liquidation(&liq(0, 30_050.0, 1.0));
        lcd.update_liquidation(&liq(1_000, 30_070.0, 1.0));
        lcd.update_liquidation(&liq(2_000, 30_020.0, 1.0));
        let cc = lcd.count();
        let cp = lcd.price();
        assert!(cc >= 3.0, "count={cc}");
        // Cluster price should be mid-point of bucket 300 (30_000 + 50 = 30_050).
        assert!((cp - 30_050.0).abs() < 1.0, "cluster_price={cp}");
    }

    #[test]
    fn below_threshold_returns_zeros() {
        let mut lcd = LiquidationClusterDetector::new(100.0, 60_000, 3);
        lcd.update_liquidation(&liq(0, 30_050.0, 1.0));
        lcd.update_liquidation(&liq(1_000, 30_070.0, 1.0));
        // Only 2 events, threshold=3.
        assert_eq!(lcd.price(), 0.0);
        assert_eq!(lcd.count(), 0.0);
        assert_eq!(lcd.volume(), 0.0);
    }

    #[test]
    fn dominant_bucket_wins() {
        // Two events in bucket A (30_000-30_100), three in bucket B (31_000-31_100).
        let mut lcd = LiquidationClusterDetector::new(100.0, 60_000, 2);
        lcd.update_liquidation(&liq(0, 30_050.0, 1.0));
        lcd.update_liquidation(&liq(1_000, 30_070.0, 1.0));
        lcd.update_liquidation(&liq(2_000, 31_010.0, 1.0));
        lcd.update_liquidation(&liq(3_000, 31_020.0, 1.0));
        lcd.update_liquidation(&liq(4_000, 31_030.0, 1.0));
        assert_eq!(lcd.count(), 3.0, "bucket B should win with 3 events");
    }

    #[test]
    fn old_events_evicted() {
        // 5-second window. Two old events in one bucket, one new event elsewhere.
        let mut lcd = LiquidationClusterDetector::new(100.0, 5_000, 2);
        lcd.update_liquidation(&liq(0, 30_050.0, 1.0));
        lcd.update_liquidation(&liq(1_000, 30_070.0, 1.0));
        // New event outside window for those two.
        lcd.update_liquidation(&liq(20_000, 31_010.0, 1.0));
        // Old events evicted, only 1 event in bucket → below threshold.
        assert_eq!(lcd.price(), 0.0, "old cluster evicted");
        assert_eq!(lcd.count(), 0.0);
        assert_eq!(lcd.volume(), 0.0);
    }

    #[test]
    fn reset_clears_state() {
        let mut lcd = LiquidationClusterDetector::new(100.0, 60_000, 3);
        lcd.update_liquidation(&liq(0, 30_050.0, 1.0));
        lcd.update_liquidation(&liq(1_000, 30_070.0, 1.0));
        lcd.update_liquidation(&liq(2_000, 30_020.0, 1.0));
        lcd.reset();
        assert!(!lcd.is_ready());
        assert_eq!(lcd.price(), 0.0);
        assert_eq!(lcd.count(), 0.0);
        assert_eq!(lcd.volume(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_liquidation_cluster_detector() {
        use crate::engine::contract_engine::IndicatorOrder;
        
        let mut f = IndicatorOrder::LiquidationClusterDetector(
            <<LiquidationClusterDetector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        f.feed(0, crate::contract::MarketSample::Liquidation(&liq(0, 30_050.0, 1.0)));
        f.feed(0, crate::contract::MarketSample::Liquidation(&liq(1_000, 30_070.0, 1.0)));
        f.feed(0, crate::contract::MarketSample::Liquidation(&liq(2_000, 30_020.0, 1.0)));
        // 3 events in the [30_000, 30_100) bucket, threshold=3 → cluster active; f.value() = first output (price)
        // price should be non-zero (cluster price ≈ 30_050)
        assert!(f.primary() > 0.0, "cluster price should be > 0");
    }
}
