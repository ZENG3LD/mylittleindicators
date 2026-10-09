//! Iceberg Detector — tracks level replenishment events to detect hidden iceberg orders.
//!
//! An iceberg order repeatedly shows a small visible size at a price level.
//! When a level disappears (size=0) and reappears (size>0), that is one
//! replenishment cycle. A high replenishment count at one price suggests
//! a large hidden order being worked there.
//!
//! Output: `Triple(side, price, count)` where:
//! - `side`: +1.0 = bid-side iceberg, -1.0 = ask-side iceberg, 0.0 = none detected
//! - `price`: price of the most recently detected iceberg level
//! - `count`: replenishment count at that level (above threshold)

use std::collections::HashMap;

use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::orderbook_delta_consumer::OrderbookDeltaConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::OrderbookDelta;

/// State for a single price level tracked by the detector.
#[derive(Default, Clone, Debug)]
struct LevelState {
    /// Last observed size (0.0 = removed).
    last_size: f64,
    /// How many times this level was replenished (removed then appeared again).
    replenishment_count: u32,
}

/// Detects iceberg orders by tracking level replenishment patterns in delta updates.
#[derive(Clone, Debug)]
pub struct IcebergDetector {
    /// Price bucket granularity for grouping nearby levels.
    price_bucket: f64,
    /// Minimum replenishment count to declare an iceberg.
    replenishment_threshold: u32,
    /// Per-bucket bid state.
    bid_levels: HashMap<i64, LevelState>,
    /// Per-bucket ask state.
    ask_levels: HashMap<i64, LevelState>,
    /// Side of last detected iceberg (+1 bid, -1 ask, 0 none).
    last_side: f64,
    /// Price of last detected iceberg level.
    last_price: f64,
    /// Replenishment count of last detected iceberg.
    last_count: f64,
    /// Whether at least one delta has been processed.
    has_data: bool,
}

impl IcebergDetector {
    /// Create a new iceberg detector.
    ///
    /// - `price_bucket`: bucket size for grouping price levels (e.g. 1.0 for whole numbers)
    /// - `replenishment_threshold`: min replenishments to flag as iceberg (e.g. 3)
    pub fn new(price_bucket: f64, replenishment_threshold: u32) -> Self {
        Self {
            price_bucket: price_bucket.max(1e-9),
            replenishment_threshold: replenishment_threshold.max(1),
            bid_levels: HashMap::new(),
            ask_levels: HashMap::new(),
            last_side: 0.0,
            last_price: 0.0,
            last_count: 0.0,
            has_data: false,
        }
    }

    #[inline]
    fn bucket(&self, price: f64) -> i64 {
        (price / self.price_bucket).floor() as i64
    }

    fn process_side(
        levels: &mut HashMap<i64, LevelState>,
        price: f64,
        size: f64,
        bucket: i64,
        threshold: u32,
        side: f64,
        last_side: &mut f64,
        last_price: &mut f64,
        last_count: &mut f64,
    ) {
        let entry = levels.entry(bucket).or_default();
        if size == 0.0 {
            // Level removed
            entry.last_size = 0.0;
        } else if entry.last_size == 0.0 && size > 0.0 {
            // Level reappeared — replenishment event
            entry.replenishment_count += 1;
            entry.last_size = size;
            if entry.replenishment_count >= threshold {
                *last_side = side;
                *last_price = price;
                *last_count = entry.replenishment_count as f64;
            }
        } else {
            // Size update (not a removal/replenishment)
            entry.last_size = size;
        }
    }
}

impl IcebergDetector {
    /// Side of last detected iceberg: +1.0 = bid, -1.0 = ask, 0.0 = none.
    pub fn side(&self) -> f64 {
        self.last_side
    }

    /// Price of the most recently detected iceberg level.
    pub fn price(&self) -> f64 {
        self.last_price
    }

    /// Replenishment count at the detected iceberg level.
    pub fn count(&self) -> f64 {
        self.last_count
    }
}

impl Default for IcebergDetector {
    fn default() -> Self {
        Self::new(1.0, 3)
    }
}

impl OrderbookDeltaConsumer for IcebergDetector {
    fn update_delta(&mut self, delta: &OrderbookDelta) {
        self.has_data = true;

        for bid in &delta.bids {
            let bucket = self.bucket(bid.price);
            Self::process_side(
                &mut self.bid_levels,
                bid.price,
                bid.size,
                bucket,
                self.replenishment_threshold,
                1.0,
                &mut self.last_side,
                &mut self.last_price,
                &mut self.last_count,
            );
        }

        for ask in &delta.asks {
            let bucket = self.bucket(ask.price);
            Self::process_side(
                &mut self.ask_levels,
                ask.price,
                ask.size,
                bucket,
                self.replenishment_threshold,
                -1.0,
                &mut self.last_side,
                &mut self.last_price,
                &mut self.last_count,
            );
        }

    }


    fn reset(&mut self) {
        self.bid_levels.clear();
        self.ask_levels.clear();
        self.last_side = 0.0;
        self.last_price = 0.0;
        self.last_count = 0.0;
        self.has_data = false;
    }

    fn is_ready(&self) -> bool {
        self.has_data
    }
}

/// Typed configuration for [`IcebergDetector`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct IcebergDetectorConfig {
    /// Price bucket size for grouping nearby levels.
    pub price_bucket: crate::contract::Param<f64>,
    /// Minimum replenishment count before declaring an iceberg.
    pub replenishment_threshold: crate::contract::Param<u32>,
}

impl Indicator for IcebergDetector {
    const ID: IndicatorId = IndicatorId::IcebergDetector;
    /// Orderbook delta iceberg-pattern detector.
    const FAMILY: &'static [Family] = &[Family::OrderBook];
    const INPUT: &'static [StreamKind] = &[StreamKind::OrderbookDelta];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::IcebergDetectorSide),
        Output::price(IndicatorOutputId::IcebergDetectorPrice),
        Output::count(IndicatorOutputId::IcebergDetectorCount),
    ];
    /// O(N_levels_in_delta) per update — two HashMap lookups per level.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
        ],
    );
    type Config = IcebergDetectorConfig;
    type Runtime = IcebergDetector;

    fn create(cfg: IcebergDetectorConfig) -> IcebergDetector {
        IcebergDetector::new(cfg.price_bucket.resolved(), cfg.replenishment_threshold.resolved())
    }
}

impl crate::contract::Config for IcebergDetectorConfig {
    fn defaults() -> Self {
        IcebergDetectorConfig {
            price_bucket: crate::contract::Param::Solo(1.0),
            replenishment_threshold: crate::contract::Param::Solo(3),
        }
    }
    fn machine_defaults() -> Self {
        use crate::contract::Param;
        let mut s = Self::machine_defaults_auto();
        // price_bucket: PIN (Class I, instrument-relative) — f64 auto leaves Solo; confirmed untouched.
        // replenishment_threshold: Class B (u32 event count, 1–20) — Param::many over explicit Vec<u32>.
        s.replenishment_threshold = Param::many((1u32..=20).collect());
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for IcebergDetector {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::IcebergDetectorSide, "Side", Color::hex(0x9C27B0), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::IcebergDetectorPrice, "Price", Color::hex(0xFF9800), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::IcebergDetectorCount, "Count", Color::hex(0xF44336), 2.0))
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

    fn make_delta(
        bids: &[(f64, f64)],
        asks: &[(f64, f64)],
        ts: i64,
    ) -> OrderbookDelta {
        OrderbookDelta {
            bids: bids.iter().map(|&(p, s)| OrderBookLevel::new(p, s)).collect(),
            asks: asks.iter().map(|&(p, s)| OrderBookLevel::new(p, s)).collect(),
            timestamp: ts,
            first_update_id: None,
            last_update_id: None,
            prev_update_id: None,
            ..Default::default()
        }
    }

    #[test]
    fn not_ready_initially() {
        let det = IcebergDetector::new(1.0, 3);
        assert!(!det.is_ready());
    }

    #[test]
    fn ready_after_first_delta() {
        let mut det = IcebergDetector::new(1.0, 3);
        det.update_delta(&make_delta(&[(100.0, 10.0)], &[], 1000));
        assert!(det.is_ready());
    }

    #[test]
    fn detects_bid_iceberg_after_replenishments() {
        let mut det = IcebergDetector::new(1.0, 3);
        // Level appears, disappears, reappears — 3 replenishments
        for i in 0..6 {
            if i % 2 == 0 {
                det.update_delta(&make_delta(&[(100.0, 5.0)], &[], i as i64 * 100));
            } else {
                det.update_delta(&make_delta(&[(100.0, 0.0)], &[], i as i64 * 100));
            }
        }
        assert!((det.side() - 1.0).abs() < 1e-9, "expected bid side +1");
        assert!((det.price() - 100.0).abs() < 1e-9);
        assert!(det.count() >= 3.0);
    }

    #[test]
    fn no_detection_below_threshold() {
        let mut det = IcebergDetector::new(1.0, 5);
        // Only 2 replenishments — below threshold of 5
        for i in 0..4 {
            if i % 2 == 0 {
                det.update_delta(&make_delta(&[(100.0, 5.0)], &[], i as i64 * 100));
            } else {
                det.update_delta(&make_delta(&[(100.0, 0.0)], &[], i as i64 * 100));
            }
        }
        assert!((det.side() - 0.0).abs() < 1e-9, "no iceberg below threshold");
    }

    #[test]
    fn reset_clears_state() {
        let mut det = IcebergDetector::new(1.0, 2);
        det.update_delta(&make_delta(&[(100.0, 5.0)], &[], 1000));
        assert!(det.is_ready());
        det.reset();
        assert!(!det.is_ready());
        assert_eq!(det.side(), 0.0);
        assert_eq!(det.price(), 0.0);
        assert_eq!(det.count(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_iceberg_detector() {
        let mut f = IndicatorOrder::IcebergDetector(
            <<IcebergDetector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let delta = make_delta(&[(100.0, 5.0)], &[], 1000);
        f.feed(0, MarketSample::OrderbookDelta(&delta));
        assert!(f.is_ready());
        // value() returns primary getter (side)
        let _ = f.primary();
    }
}
