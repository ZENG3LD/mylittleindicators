//! Rolling Volume Profile — POC, VAH, VAL over a sliding window.
//!
//! Builds a price-bucket histogram over the last `rolling_window` bars and
//! computes:
//!
//! - **POC** (Point of Control): price bucket with the highest cumulative volume.
//! - **VAH** (Value Area High): upper boundary of the Value Area.
//! - **VAL** (Value Area Low): lower boundary of the Value Area.
//!
//! The Value Area contains approximately 70 % of total volume (configurable
//! via `value_area_pct`). The algorithm expands from POC outward, adding
//! the bucket with the largest volume on each side alternately until the
//! target percentage is reached.
//!
//! **Bucket size** is `price_bucket_size` (absolute price units). Use a
//! value matched to the instrument's tick size (e.g. `0.01` for crypto at
//! 2-decimal precision, `1.0` for index futures, etc.).
//!
//! Output: `Triple(poc, vah, val)`.

use std::collections::{HashMap, VecDeque};

use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};

/// Fixed price-window (rows) of the exposed volume-profile VECTOR, centered on the POC bucket.
const RVP_WINDOW: u16 = 64;

/// Rolling Volume Profile indicator.
///
/// Returns `Triple(poc, vah, val)` once `rolling_window` bars have been fed.
/// Returns `Single(close)` during warm-up.
#[derive(Debug, Clone)]
pub struct RollingVolumeProfile {
    rolling_window: usize,
    price_bucket_size: f64,
    value_area_pct: f64,
    bars: VecDeque<(f64, f64)>, // (typical_price, volume)
    last_poc: f64,
    last_vah: f64,
    last_val: f64,
    ready: bool,
    /// The volume-by-price histogram as a `RVP_WINDOW × 1` VECTOR (row = price bucket centered
    /// on the POC, value = cumulative volume). The full profile the 3 scalars (poc/vah/val)
    /// summarize; read via `profile_grid` / `ContractFactory::matrix_grid`. Row→price maps as
    /// `poc + (row - WINDOW/2) * price_bucket_size`.
    grid: MatrixGrid,
}

impl RollingVolumeProfile {
    /// Create a new `RollingVolumeProfile`.
    ///
    /// - `rolling_window`   — number of bars in the sliding window (≥ 2).
    /// - `price_bucket_size`— absolute bucket width in price units (> 0).
    /// - `value_area_pct`   — fraction of total volume defining the value area (clamped 0.01–0.99).
    pub fn new(rolling_window: usize, price_bucket_size: f64, value_area_pct: f64) -> Self {
        let w = rolling_window.max(2);
        Self {
            rolling_window: w,
            price_bucket_size: price_bucket_size.max(1e-9),
            value_area_pct: value_area_pct.clamp(0.01, 0.99),
            bars: VecDeque::with_capacity(w + 1),
            last_poc: 0.0,
            last_vah: 0.0,
            last_val: 0.0,
            ready: false,
            grid: MatrixGrid::new(RVP_WINDOW, 1, false)
                .with_labels(AxisLabels::Values(Vec::new()), AxisLabels::Bars),
        }
    }

    /// Feed resolved lanes `[high, low, close, volume]` and return `Triple(poc, vah, val)` once
    /// ready, otherwise `Single(close)` during warm-up.
    pub fn feed(&mut self, lanes: &[f64]) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let volume = lanes[3];
        let typical = (high + low + close) / 3.0;
        self.bars.push_back((typical, volume));
        if self.bars.len() > self.rolling_window {
            self.bars.pop_front();
        }

        if self.bars.len() < self.rolling_window {
            return;
        }

        self.compute_profile();
        self.ready = true;
    }

    /// Recompute POC/VAH/VAL from current `bars`.
    fn compute_profile(&mut self) {
        let bs = self.price_bucket_size;

        // Build bucket map.
        let mut buckets: HashMap<i64, f64> = HashMap::new();
        for &(price, vol) in &self.bars {
            if vol > 0.0 {
                let key = (price / bs).floor() as i64;
                *buckets.entry(key).or_insert(0.0) += vol;
            }
        }

        if buckets.is_empty() {
            return;
        }

        // POC = bucket with max volume.
        let poc_key = buckets
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(&k, _)| k)
            .unwrap_or(0);

        let poc_price = (poc_key as f64 + 0.5) * bs;
        self.last_poc = poc_price;

        // Value Area: expand from POC outward until value_area_pct of total volume included.
        let total_vol: f64 = buckets.values().sum();
        let target = total_vol * self.value_area_pct;

        let mut sorted_keys: Vec<i64> = buckets.keys().copied().collect();
        sorted_keys.sort_unstable();

        let poc_pos = sorted_keys.partition_point(|&k| k < poc_key);

        let mut included_vol = buckets[&poc_key];
        let mut lo_idx = poc_pos; // exclusive lower pointer (moves left)
        let mut hi_idx = poc_pos; // exclusive upper pointer (moves right)

        // Expand greedily: at each step pick the side with higher next volume.
        loop {
            if included_vol >= target {
                break;
            }

            let lo_vol = if lo_idx > 0 {
                buckets[&sorted_keys[lo_idx - 1]]
            } else {
                f64::NEG_INFINITY
            };
            let hi_vol = if hi_idx + 1 < sorted_keys.len() {
                buckets[&sorted_keys[hi_idx + 1]]
            } else {
                f64::NEG_INFINITY
            };

            match (lo_vol > f64::NEG_INFINITY, hi_vol > f64::NEG_INFINITY) {
                (false, false) => break,
                (true, false) => {
                    included_vol += lo_vol;
                    lo_idx -= 1;
                }
                (false, true) => {
                    included_vol += hi_vol;
                    hi_idx += 1;
                }
                (true, true) => {
                    if lo_vol >= hi_vol {
                        included_vol += lo_vol;
                        lo_idx -= 1;
                    } else {
                        included_vol += hi_vol;
                        hi_idx += 1;
                    }
                }
            }
        }

        let val_key = sorted_keys[lo_idx];
        let vah_key = sorted_keys[hi_idx];
        self.last_val = val_key as f64 * bs;
        self.last_vah = (vah_key as f64 + 1.0) * bs;

        // Snapshot the volume-by-price histogram into the fixed price-window vector (rows
        // centered on the POC bucket).
        self.grid.reset();
        let half = (RVP_WINDOW / 2) as i64;
        let mut row_prices = Vec::with_capacity(RVP_WINDOW as usize);
        for row in 0..RVP_WINDOW {
            let key = poc_key + (row as i64 - half);
            let price = (key as f64 + 0.5) * bs;
            row_prices.push(price);
            if let Some(&vol) = buckets.get(&key) {
                self.grid.set_direction(MatrixCell::new(row, 0), vol);
            }
        }
        self.grid.set_row_values(&row_prices);
    }


    /// Named output: brace `poc`.
    pub fn poc(&self) -> f64 { self.last_poc }
    /// Named output: brace `vah`.
    pub fn vah(&self) -> f64 { self.last_vah }
    /// Named output: brace `val`.
    pub fn val(&self) -> f64 { self.last_val }

    /// The volume-by-price profile as an `RVP_WINDOW × 1` vector — the matrix output behind
    /// `IndicatorOutputId::RvpProfileGrid`.
    pub fn profile_grid(&self) -> &MatrixGrid { &self.grid }

    /// Returns `true` once `rolling_window` bars have been fed.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Clears all state.
    pub fn reset(&mut self) {
        self.bars.clear();
        self.last_poc = 0.0;
        self.last_vah = 0.0;
        self.last_val = 0.0;
        self.ready = false;
        self.grid.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(ind: &mut RollingVolumeProfile, price: f64, vol: f64) {
        ind.feed(&[price + 0.5, price - 0.5, price, vol])
    }

    #[test]
    fn warmup_returns_single() {
        let mut rvp = RollingVolumeProfile::new(5, 1.0, 0.7);
        for i in 0..4u32 {
            feed(&mut rvp, 100.0 + i as f64, 100.0);
            assert!(!rvp.is_ready(), "not ready during warmup");
        }
    }

    #[test]
    fn after_warmup_returns_triple() {
        let mut rvp = RollingVolumeProfile::new(5, 1.0, 0.7);
        for i in 0..5u32 {
            feed(&mut rvp, 100.0 + i as f64, 100.0);
        }
        assert!(rvp.is_ready(), "expected ready after warmup");
        assert!(rvp.poc().is_finite());
        assert!(rvp.vah().is_finite());
        assert!(rvp.val().is_finite());
    }

    #[test]
    fn poc_is_highest_volume_bucket() {
        let mut rvp = RollingVolumeProfile::new(5, 1.0, 0.7);
        // Feed 5 bars: price 100 with vol 1000 (dominant), others with vol 100.
        feed(&mut rvp, 100.0, 1000.0);
        feed(&mut rvp, 101.0, 100.0);
        feed(&mut rvp, 102.0, 100.0);
        feed(&mut rvp, 103.0, 100.0);
        feed(&mut rvp, 104.0, 100.0);
        let poc = rvp.poc();
        // POC should be near 100 (the dominant bucket centre).
        assert!(
            (poc - 100.5).abs() < 1.0,
            "expected POC near 100.5, got {poc}"
        );
    }

    #[test]
    fn vah_above_val() {
        let mut rvp = RollingVolumeProfile::new(5, 1.0, 0.7);
        for i in 0..5u32 {
            feed(&mut rvp, 100.0 + i as f64, 100.0 + i as f64 * 10.0);
        }
        let (poc, vah, val) = (rvp.poc(), rvp.vah(), rvp.val());
        assert!(vah >= poc, "VAH {vah} must be >= POC {poc}");
        assert!(poc >= val, "POC {poc} must be >= VAL {val}");
    }

    #[test]
    fn row_label_at_center_is_poc_price() {
        use crate::engine::matrix_grid::Label;
        let bucket_size = 1.0_f64;
        let mut rvp = RollingVolumeProfile::new(5, bucket_size, 0.7);
        // Dominant bucket near 100: typical = (100.5 + 99.5 + 100.0) / 3 = 100.0 → key = 100.
        feed(&mut rvp, 100.0, 1000.0);
        feed(&mut rvp, 101.0, 100.0);
        feed(&mut rvp, 102.0, 100.0);
        feed(&mut rvp, 103.0, 100.0);
        feed(&mut rvp, 104.0, 100.0);
        let center = RVP_WINDOW / 2;
        let label = rvp.profile_grid().row_label(center);
        // POC bucket key = 100; price = (100.0 + 0.5) * 1.0 = 100.5
        let expected = 100.5_f64;
        if let Label::Value(p) = label {
            assert!((p - expected).abs() < bucket_size, "row_label at center = {p}, expected ≈ {expected}");
        } else {
            panic!("expected Label::Value, got {:?}", label);
        }
    }

    #[test]
    fn profile_vector_captures_volume_around_poc() {
        let mut rvp = RollingVolumeProfile::new(5, 1.0, 0.7);
        feed(&mut rvp, 100.0, 1000.0); // dominant bucket -> POC
        feed(&mut rvp, 101.0, 100.0);
        feed(&mut rvp, 102.0, 100.0);
        feed(&mut rvp, 103.0, 100.0);
        feed(&mut rvp, 104.0, 100.0);
        let g = rvp.profile_grid();
        assert_eq!(g.cols(), 1, "a 1-D volume-by-price vector");
        // The POC bucket sits at the center row carrying the dominant volume.
        let center = RVP_WINDOW / 2;
        assert!(
            g.read_direction(MatrixCell::new(center, 0)) >= 1000.0,
            "POC volume at the center row"
        );
    }

    #[test]
    fn reset_clears_state() {
        let mut rvp = RollingVolumeProfile::new(3, 1.0, 0.7);
        for i in 0..3u32 {
            feed(&mut rvp, 100.0 + i as f64, 100.0);
        }
        assert!(rvp.is_ready());
        rvp.reset();
        assert!(!rvp.is_ready());
        feed(&mut rvp, 100.0, 100.0);
        assert!(!rvp.is_ready(), "still not ready after single bar post-reset");
    }
}

impl Default for RollingVolumeProfile {
    fn default() -> Self {
        Self::new(50, 1.0, 0.7)
    }
}

// ---- Indicator contract ----

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity, Store, StoreKind, ValueDomain, sweep_f64};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`RollingVolumeProfile`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RvpConfig {
    /// Number of bars in the rolling window.
    pub rolling_window: Param<usize>,
    /// Absolute price bucket width.
    pub price_bucket_size: Param<f64>,
    /// Fraction of total volume defining the value area (0.01–0.99).
    pub value_area_pct: Param<f64>,
}

impl Indicator for RollingVolumeProfile {
    const ID: IndicatorId = IndicatorId::Rvp;
    /// Standalone rolling volume profile — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/C + volume lanes for typical-price bucketing.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const NEEDS_VOLUME: bool = true;
    /// O(window) per bar — rescans price buckets each update.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Deque)]);
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::RvpPoc),
        Output::price(IndicatorOutputId::RvpVah),
        Output::price(IndicatorOutputId::RvpVal),
        Output::matrix(IndicatorOutputId::RvpProfileGrid, ValueDomain::Count),
    ];
    type Config = RvpConfig;
    type Runtime = RollingVolumeProfile;

    fn create(cfg: RvpConfig) -> RollingVolumeProfile {
        RollingVolumeProfile::new(
            cfg.rolling_window.resolved().max(1),
            cfg.price_bucket_size.resolved(),
            cfg.value_area_pct.resolved(),
        )
    }
}

impl crate::contract::Config for RvpConfig {
    fn defaults() -> Self {
        RvpConfig {
            rolling_window: Param::Solo(50),
            price_bucket_size: Param::Solo(1.0),
            value_area_pct: Param::Solo(0.7),
        }
    }
    fn machine_defaults() -> Self {
        // rolling_window: Class A period/window — auto gives range(2,4048,1)
        // price_bucket_size (f64): Class I PIN — instrument-relative price step, leave Solo (auto leaves f64 Solo)
        // value_area_pct (f64): Class D ratio 0..=1 — sweep 0.0..=1.0 step 0.05
        let mut s = Self::machine_defaults_auto();
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


impl Render for RollingVolumeProfile {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::RvpPoc, "POC", Color::hex(0xFF9800))
            .line_output(IndicatorOutputId::RvpVah, "VAH", Color::hex(0x4CAF50))
            .line_output(IndicatorOutputId::RvpVal, "VAL", Color::hex(0xF44336))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_lanes() {
        let mut f = IndicatorOrder::Rvp(<<RollingVolumeProfile as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Fill the rolling window
        for i in 0..50u32 {
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 100.5 + i as f64,
                low: 99.5 + i as f64,
                close: 100.0 + i as f64,
                volume: 100.0,
            });
        }
        let v = f.read(IndicatorOutputId::RvpPoc);
        assert!(v > 0.0 && v.is_finite(), "POC should be a positive price level, got {v}");
    }

    #[test]
    fn factory_exposes_rvp_profile_vector() {
        let f = IndicatorOrder::Rvp(<<RollingVolumeProfile as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        let g = f
            .grid(IndicatorOutputId::RvpProfileGrid)
            .expect("Rvp emits the volume-profile vector");
        assert_eq!(g.rows(), 64);
        assert_eq!(g.cols(), 1);

        let sma = IndicatorOrder::from_defaults(IndicatorId::Sma).unwrap().build_solo().unwrap();
        assert!(sma.grid(IndicatorOutputId::RvpProfileGrid).is_none());
    }
}
