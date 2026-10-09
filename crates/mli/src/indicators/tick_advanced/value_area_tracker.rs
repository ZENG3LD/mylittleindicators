//! ValueAreaTracker — rolling Volume Profile Value Area.
//!
//! Maintains a time-windowed volume profile bucketed by price. Computes:
//! - **POC** (Point of Control) — price bucket with maximum volume.
//! - **VAH** (Value Area High) — upper bound of the Value Area.
//! - **VAL** (Value Area Low)  — lower bound of the Value Area.
//!
//! The Value Area is expanded symmetrically from the POC (higher-volume
//! neighbour first) until accumulated volume ≥ `value_area_pct` × total.
//!
//! Outputs: `poc_price`, `vah`, `val`.

use std::collections::{HashMap, VecDeque};

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::engine::time_window::TimeWindow;
use crate::contract::{Family, Indicator, Output, SourceAxis, ValueDomain};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::core::types::Tick;
use crate::engine::stream_kind::StreamKind;
use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};

/// Fixed price-window (rows) of the exposed volume-profile VECTOR, centered on the POC bucket.
const VAT_WINDOW: u16 = 64;

/// Map raw price to a bucket index.
#[inline]
fn bucket(price: f64, bucket_size: f64) -> i64 {
    (price / bucket_size).floor() as i64
}

/// Rolling Volume Profile Value Area tracker.
///
/// Parameters:
/// - `window_ms`       — rolling time window in milliseconds.
/// - `price_bucket`    — bucket width in price units (e.g. `1.0` for BTC).
/// - `value_area_pct`  — fraction of total volume that defines the Value Area (default 0.70).
#[derive(Debug, Clone)]
pub struct ValueAreaTracker {
    window_ms: i64,
    price_bucket: f64,
    value_area_pct: f64,
    /// Circular buffer: `(timestamp_ms, price, qty)`.
    events: VecDeque<(i64, f64, f64)>,
    last_poc: f64,
    last_vah: f64,
    last_val: f64,
    /// The volume-by-price histogram as a `VAT_WINDOW × 1` VECTOR (row = price bucket centered
    /// on the POC, value = cumulative volume). Read via `profile_grid` /
    /// `ContractFactory::grid(IndicatorOutputId::ValueAreaTrackerProfileGrid)`.
    grid: MatrixGrid,
}

impl ValueAreaTracker {
    /// Create a new tracker.
    ///
    /// - `window_ms`      — rolling window in milliseconds (clamped ≥ 1).
    /// - `price_bucket`   — bucket width > 0.
    /// - `value_area_pct` — fraction in (0, 1] for the value area (default 0.70).
    pub fn new(window_ms: i64, price_bucket: f64, value_area_pct: f64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            price_bucket: price_bucket.max(f64::EPSILON),
            value_area_pct: value_area_pct.clamp(f64::EPSILON, 1.0),
            events: VecDeque::with_capacity(512),
            last_poc: 0.0,
            last_vah: 0.0,
            last_val: 0.0,
            grid: MatrixGrid::new(VAT_WINDOW, 1, false)
                .with_labels(AxisLabels::Values(Vec::new()), AxisLabels::Bars),
        }
    }

    /// Point of Control — price bucket midpoint with maximum volume.
    pub fn poc(&self) -> f64 {
        self.last_poc
    }

    /// Value Area High — upper boundary of the value area.
    pub fn vah(&self) -> f64 {
        self.last_vah
    }

    /// Value Area Low — lower boundary of the value area.
    pub fn val(&self) -> f64 {
        self.last_val
    }

    /// Convenience constructor with 70 % value area.
    pub fn with_window(window_ms: i64, price_bucket: f64) -> Self {
        Self::new(window_ms, price_bucket, 0.70)
    }

    /// Build volume profile and compute POC / VAH / VAL.
    /// Returns `(poc_price, vah, val, poc_bucket, profile)`.
    fn compute(
        events: &VecDeque<(i64, f64, f64)>,
        price_bucket: f64,
        value_area_pct: f64,
        fallback_price: f64,
    ) -> (f64, f64, f64, i64, HashMap<i64, f64>) {
        if events.is_empty() {
            return (fallback_price, fallback_price, fallback_price, 0, HashMap::new());
        }

        // Build profile: bucket_index → accumulated volume.
        let mut profile: HashMap<i64, f64> = HashMap::new();
        for &(_, p, q) in events {
            *profile.entry(bucket(p, price_bucket)).or_insert(0.0) += q;
        }

        // POC = bucket with maximum volume.
        let (&poc_bucket, &poc_vol) = profile
            .iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap(); // safe: profile is non-empty

        let total: f64 = profile.values().sum();
        let target = total * value_area_pct;

        // Expand from POC outward, always taking the higher-volume neighbour.
        let mut va_buckets: Vec<i64> = vec![poc_bucket];
        let mut accumulated = poc_vol;
        let mut up = poc_bucket + 1;
        let mut down = poc_bucket - 1;

        while accumulated < target {
            let up_vol = profile.get(&up).copied().unwrap_or(0.0);
            let down_vol = profile.get(&down).copied().unwrap_or(0.0);
            if up_vol == 0.0 && down_vol == 0.0 {
                break;
            }
            if up_vol >= down_vol {
                va_buckets.push(up);
                accumulated += up_vol;
                up += 1;
            } else {
                va_buckets.push(down);
                accumulated += down_vol;
                down -= 1;
            }
        }

        let poc_price = poc_bucket as f64 * price_bucket + price_bucket / 2.0;
        let max_bucket = *va_buckets.iter().max().unwrap(); // safe: vec non-empty
        let min_bucket = *va_buckets.iter().min().unwrap();
        let vah = (max_bucket + 1) as f64 * price_bucket;
        let val = min_bucket as f64 * price_bucket;

        (poc_price, vah, val, poc_bucket, profile)
    }

    /// The volume-by-price profile as a `VAT_WINDOW × 1` vector — the matrix output behind
    /// `IndicatorOutputId::ValueAreaTrackerProfileGrid`.
    pub fn profile_grid(&self) -> &MatrixGrid { &self.grid }
}

impl TickConsumer for ValueAreaTracker {
    fn update_tick(&mut self, tick: &Tick) {
        self.events.push_back((tick.time, tick.price, tick.size));

        // Evict stale events.
        while let Some(&(ts, _, _)) = self.events.front() {
            if tick.time - ts > self.window_ms {
                self.events.pop_front();
            } else {
                break;
            }
        }

        let (poc, vah, val, poc_bucket, profile) =
            Self::compute(&self.events, self.price_bucket, self.value_area_pct, tick.price);
        self.last_poc = poc;
        self.last_vah = vah;
        self.last_val = val;

        // Snapshot the volume-by-price histogram into the fixed price-window vector (rows centered
        // on the POC bucket).
        self.grid.reset();
        let half = (VAT_WINDOW / 2) as i64;
        let pb = self.price_bucket;
        let mut row_prices = Vec::with_capacity(VAT_WINDOW as usize);
        for row in 0..VAT_WINDOW {
            let key = poc_bucket + (row as i64 - half);
            let price = key as f64 * pb + pb / 2.0;
            row_prices.push(price);
            if let Some(&vol) = profile.get(&key) {
                self.grid.set_direction(MatrixCell::new(row, 0), vol);
            }
        }
        self.grid.set_row_values(&row_prices);
    }


    fn reset(&mut self) {
        self.events.clear();
        self.last_poc = 0.0;
        self.last_vah = 0.0;
        self.last_val = 0.0;
        self.grid.reset();
    }

    fn is_ready(&self) -> bool {
        !self.events.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(time_ms: i64, price: f64, size: f64) -> Tick {
        Tick::new(time_ms, price, size, true)
    }

    #[test]
    fn single_bucket_poc_equals_price() {
        let mut vat = ValueAreaTracker::new(60_000, 10.0, 0.7);
        // All trades at the same price → single bucket.
        vat.update_tick(&tick(0, 100.0, 5.0));
        vat.update_tick(&tick(100, 105.0, 2.0)); // same bucket (100–110)
        vat.update_tick(&tick(200, 102.0, 1.0));
        let poc = vat.poc();
        let vah = vat.vah();
        let val = vat.val();
        // VAL ≤ POC ≤ VAH
        assert!(val <= poc && poc <= vah, "val={val} poc={poc} vah={vah}");
    }

    #[test]
    fn poc_at_dominant_bucket() {
        // Bucket 0 (0–10): size 100
        // Bucket 1 (10–20): size 1
        // → POC should be in bucket 0
        let mut vat = ValueAreaTracker::new(60_000, 10.0, 0.7);
        for i in 0..10 {
            vat.update_tick(&tick(i * 100, 5.0, 10.0)); // 10 × 10 = 100 in bucket 0
        }
        vat.update_tick(&tick(1_100, 15.0, 1.0)); // bucket 1
        // POC midpoint of bucket 0 (0.0–10.0) = 5.0
        let poc = vat.poc();
        let vah = vat.vah();
        let val = vat.val();
        assert!((poc - 5.0).abs() < 1e-9, "POC expected 5.0, got {poc}");
        assert!(val <= poc && poc <= vah);
    }

    #[test]
    fn stale_events_evicted() {
        // Window = 10 s. Insert buys at t=0, then a single tick at t=15_000.
        // Old events should be dropped.
        let mut vat = ValueAreaTracker::new(10_000, 10.0, 0.7);
        for i in 0..5 {
            vat.update_tick(&tick(i * 100, 5.0, 100.0)); // in bucket 0
        }
        // New tick 15 s later — all old events gone.
        vat.update_tick(&tick(15_000, 250.0, 1.0));
        // Only the last tick is in window — POC is in bucket 25 (250/10=25).
        let poc = vat.poc();
        assert!((poc - 255.0).abs() < 1e-9, "POC should be midpoint of bucket 25, got {poc}");
    }

    #[test]
    fn reset_clears_state() {
        let mut vat = ValueAreaTracker::new(60_000, 10.0, 0.7);
        vat.update_tick(&tick(0, 100.0, 5.0));
        assert!(vat.is_ready());
        vat.reset();
        assert!(!vat.is_ready());
        assert_eq!(vat.poc(), 0.0);
        assert_eq!(vat.vah(), 0.0);
        assert_eq!(vat.val(), 0.0);
    }

    #[test]
    fn row_label_at_center_is_poc_price() {
        use crate::engine::matrix_grid::Label;
        let bucket_size = 10.0_f64;
        let mut vat = ValueAreaTracker::new(60_000, bucket_size, 0.7);
        // 10 trades at price 5.0 → bucket 0 (0–10), dominant.
        for i in 0..10i64 {
            vat.update_tick(&tick(i * 100, 5.0, 10.0));
        }
        vat.update_tick(&tick(1_100, 15.0, 1.0)); // bucket 1
        let center = VAT_WINDOW / 2;
        let label = vat.profile_grid().row_label(center);
        // POC bucket 0: price = 0 * 10.0 + 10.0/2 = 5.0
        let expected = 5.0_f64;
        if let Label::Value(p) = label {
            assert!((p - expected).abs() < bucket_size, "row_label at center = {p}, expected ≈ {expected}");
        } else {
            panic!("expected Label::Value, got {:?}", label);
        }
    }
}

impl Default for ValueAreaTracker {
    /// Factory default: window_ms=300000, price_bucket=1.0, value_area_pct=0.7.
    fn default() -> Self {
        Self::new(300_000, 1.0, 0.7)
    }
}

// ---- Indicator contract ----

use crate::contract::{Param, sweep_f64};

/// Typed configuration for [`ValueAreaTracker`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ValueAreaTrackerConfig {
    pub window: Param<TimeWindow>,
    /// Bucket width in price units.
    pub price_bucket: Param<f64>,
    /// Fraction of total volume defining the Value Area (0 < x ≤ 1).
    pub value_area_pct: Param<f64>,
}

impl Indicator for ValueAreaTracker {
    const ID: IndicatorId = IndicatorId::ValueAreaTracker;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::ValueAreaTrackerPoc),
        Output::price(IndicatorOutputId::ValueAreaTrackerVah),
        Output::price(IndicatorOutputId::ValueAreaTrackerVal),
        Output::matrix(IndicatorOutputId::ValueAreaTrackerProfileGrid, ValueDomain::Count),
    ];
    type Config = ValueAreaTrackerConfig;
    type Runtime = ValueAreaTracker;

    fn create(cfg: ValueAreaTrackerConfig) -> ValueAreaTracker {
        ValueAreaTracker::new(
            cfg.window.resolved().as_millis(),
            cfg.price_bucket.resolved(),
            cfg.value_area_pct.resolved(),
        )
    }
}

impl crate::contract::Config for ValueAreaTrackerConfig {
    fn defaults() -> Self {
        ValueAreaTrackerConfig {
            window: Param::Solo(TimeWindow::Minutes(5)),
            price_bucket: Param::Solo(1.0),
            value_area_pct: Param::Solo(0.7),
        }
    }
    fn machine_defaults() -> Self {
        // window (TimeWindow): Class N — 11-point curated discrete set.
        // price_bucket (f64): Class I — instrument-relative bucket width; NOT swept (PIN).
        //   Left as Solo from auto (f64 Solo by default from machine_defaults_auto).
        // value_area_pct (f64): Class D ratio — fraction of total volume for the value area;
        //   swept 0.0..=1.0 step 0.05.
        let mut s = Self::machine_defaults_auto();
        s.window = Param::many(vec![
            TimeWindow::Seconds(1), TimeWindow::Seconds(5), TimeWindow::Seconds(15),
            TimeWindow::Seconds(30), TimeWindow::Minutes(1), TimeWindow::Minutes(5),
            TimeWindow::Minutes(15), TimeWindow::Minutes(30),
            TimeWindow::Hours(1), TimeWindow::Hours(4), TimeWindow::Hours(24),
        ]);
        // price_bucket: Class I pin — left Solo (auto keeps f64 as Solo).
        s.value_area_pct = Param::many(sweep_f64(0.0, 1.0, 0.05));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for ValueAreaTracker {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::ValueAreaTrackerPoc, "POC", Color::hex(0xFF9800), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::ValueAreaTrackerVah, "VAH", Color::hex(0x4CAF50), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::ValueAreaTrackerVal, "VAL", Color::hex(0xF44336), 1.5))
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
    fn factory_feeds_resolved_value_area_tracker() {
        let mut f = IndicatorOrder::ValueAreaTracker(
            <<ValueAreaTracker as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let t = crate::core::types::Tick::new(0, 100.0, 1.0, true);
        f.feed(0, MarketSample::Tick(&t));
        assert!(f.primary().is_finite());
    }

    #[test]
    fn profile_vector_captures_volume_around_poc() {
        let mut vat = ValueAreaTracker::new(60_000, 10.0, 0.7);
        // Dominant bucket near price 100 (bucket 10), 10 × size 100 = 1000 vol.
        for i in 0..10i64 {
            vat.update_tick(&Tick::new(i * 100, 100.0 + i as f64 * 0.5, 100.0, true));
        }
        // Minor trade at a different bucket.
        vat.update_tick(&Tick::new(1_100, 200.0, 1.0, true));

        let g = vat.profile_grid();
        assert_eq!(g.cols(), 1, "volume-by-price vector has exactly 1 column");
        assert_eq!(g.rows(), VAT_WINDOW, "vector height == VAT_WINDOW");
        // POC bucket sits at center row — its cumulative volume must be non-zero.
        let center = VAT_WINDOW / 2;
        assert!(
            g.read_direction(MatrixCell::new(center, 0)) > 0.0,
            "POC bucket at center row should carry non-zero volume"
        );
    }

    #[test]
    fn factory_exposes_value_area_profile_vector() {
        let f = IndicatorOrder::ValueAreaTracker(
            <<ValueAreaTracker as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let g = f
            .grid(IndicatorOutputId::ValueAreaTrackerProfileGrid)
            .expect("ValueAreaTracker emits the value-area profile vector");
        assert_eq!(g.rows(), 64);
        assert_eq!(g.cols(), 1);

        let sma = IndicatorOrder::from_defaults(crate::engine::indicator_id::IndicatorId::Sma)
            .unwrap()
            .build_solo()
            .unwrap();
        assert!(sma.grid(IndicatorOutputId::ValueAreaTrackerProfileGrid).is_none());
    }
}
