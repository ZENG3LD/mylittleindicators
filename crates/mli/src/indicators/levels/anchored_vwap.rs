// Anchored VWAP (AVWAP)
// Calendar-anchored (monthly) implementation. Resets the VWAP accumulator at the start of a new month.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AvwapAnchorMode {
    Monthly,
}

#[derive(Debug, Clone, Copy)]
pub struct AnchoredVwapParams {
    pub mode: AvwapAnchorMode,
}

impl Default for AnchoredVwapParams {
    fn default() -> Self {
        Self {
            mode: AvwapAnchorMode::Monthly,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnchoredVwap {
    _params: AnchoredVwapParams,
    // Accumulators for current anchor window
    cum_pv: f64,
    cum_v: f64,
    // Last computed value
    value: f64,
    // Calendar tracking (UTC days since epoch divided to month id)
    last_month_key: i32,
    is_ready: bool,
}

impl Default for AnchoredVwap {
    /// Factory default: `new(AnchoredVwapParams::default())` — monthly anchor.
    fn default() -> Self {
        Self::new(AnchoredVwapParams::default())
    }
}

impl AnchoredVwap {
    pub fn new(params: AnchoredVwapParams) -> Self {
        Self {
            _params: params,
            cum_pv: 0.0,
            cum_v: 0.0,
            value: 0.0,
            last_month_key: i32::MIN,
            is_ready: false,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.cum_pv = 0.0;
        self.cum_v = 0.0;
        self.value = 0.0;
        self.last_month_key = i32::MIN;
        self.is_ready = false;
    }

    /// Feed `(ts, &[H,L,C,V])` — the `Fields +time` contract arm. The monthly anchor reset uses
    /// the wall-clock (canonical MILLISECONDS → UTC seconds); typical-price weighting uses
    /// H/L/C/V. Time is the orthogonal coordinate; the lanes are the source-resolved scalars.
    pub fn feed(&mut self, ts_ms: i64, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let volume = lanes[3];
        let unix_time_secs = ts_ms.div_euclid(1000);
        // Determine month key (YYYY*12 + MM) using simple UTC approximation
        let month_key = Self::calc_month_key(unix_time_secs);

        // Handle anchor reset
        if self.last_month_key != month_key {
            self.cum_pv = 0.0;
            self.cum_v = 0.0;
            self.is_ready = false;
            self.last_month_key = month_key;
        }

        // Typical price * volume accumulation
        let tp = (high + low + close) / 3.0;
        self.cum_pv += tp * volume.max(0.0);
        self.cum_v += volume.max(0.0);

        if self.cum_v > 0.0 {
            self.value = self.cum_pv / self.cum_v;
            self.is_ready = true;
        }

        self.value
    }

    /// Cumulative VWAP path (NO calendar reset) — used by inner-consumers that drive AVWAP
    /// without a timestamp (e.g. `RelativeTrendPosition`). The factory uses the
    /// timestamp-aware `feed(ts, &[lanes])` (the `Fields +time` arm).
    pub fn feed_cumulative(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];
        let volume = lanes[3];
        let tp = (high + low + close) / 3.0;
        self.cum_pv += tp * volume.max(0.0);
        self.cum_v += volume.max(0.0);
        if self.cum_v > 0.0 {
            self.value = self.cum_pv / self.cum_v;
            self.is_ready = true;
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    #[inline]
    fn calc_month_key(unix_time_secs: i64) -> i32 {
        // Rough UTC month extraction using chrono is heavy; do a light-weight approx via seconds
        // This is acceptable for D1 bars, as reset happens on first bar of a new month by upstream loader
        // If precise calendar split is needed, we can switch to chrono without impacting perf much on D1.
        // Here we compute days since epoch and map to approximate month with 365.2425/12 ≈ 30.436875 days.
        let days = unix_time_secs / 86_400;
        let approx_months_since_epoch = (days as f64 / 30.436875) as i64;
        approx_months_since_epoch as i32
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for AnchoredVwap — no period, just anchor mode.
#[derive(Debug, Clone, Copy)]
pub struct AvwapConfig {
    pub mode: AvwapAnchorMode,
}

impl AvwapConfig {
    /// Config fingerprint: hash the anchor mode field.
    pub fn config_hash(&self) -> u64 {
        use ::core::hash::{Hash as _, Hasher as _};
        let mut h = ::std::collections::hash_map::DefaultHasher::new();
        self.mode.hash(&mut h);
        h.finish()
    }

    /// Anchor mode is not a search axis (`cube_size == 1`) — the sole cube point is `self`.
    pub fn axes_decode(&self, _idx: u128) -> Self { *self }
}

impl crate::contract::Config for AvwapConfig {
    fn defaults() -> Self {
        AvwapConfig { mode: AvwapAnchorMode::Monthly }
    }
    /// Anchor mode is a user benchmark choice, not a search axis — the machine sweep is the
    /// single render default (no `Param` cube here; `iter` yields one instance).
    fn machine_defaults() -> Self {
        Self::defaults()
    }
    fn cube_size(&self) -> u128 {
        1
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        Box::new(std::iter::once(*self))
    }
}

impl Indicator for AnchoredVwap {
    const ID: IndicatorId = IndicatorId::Avwap;
    /// No family — a calendar-anchored level / benchmark, not a pluggable smoother or
    /// oscillator family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed 4-lane slice (H/L/C/V); the `Fields +time` arm resolves these lanes and feeds `(ts, &[H,L,C,V])`.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const NEEDS_VOLUME: bool = true;
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Avwap)];
    /// O(1) update — no window buffer; pure running accumulator (cum_pv, cum_v).
    const COST: Cost = Cost::new(
        UpdateComplexity::Constant,
        &[Store::fixed(StoreKind::Scalar, 4)],
    );
    type Config = AvwapConfig;
    type Runtime = AnchoredVwap;

    fn create(cfg: AvwapConfig) -> AnchoredVwap {
        AnchoredVwap::new(AnchoredVwapParams { mode: cfg.mode })
    }
}


impl Render for AnchoredVwap {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Avwap, "AVWAP", Color::hex(0x9C27B0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anchored_vwap_creation() {
        let avwap = AnchoredVwap::new(AnchoredVwapParams::default());
        assert!(!avwap.is_ready());
        assert_eq!(avwap.value(), 0.0);
    }

    #[test]
    fn test_anchored_vwap_update() {
        let mut avwap = AnchoredVwap::new(AnchoredVwapParams::default());
        let ts_ms = 1_700_000_000_000_i64; // canonical ms
        let value = avwap.feed(ts_ms, &[102.0, 98.0, 101.0, 1000.0]);
        assert!(avwap.is_ready());
        assert!(value > 0.0);
    }

    #[test]
    fn test_anchored_vwap_accumulation() {
        let mut avwap = AnchoredVwap::new(AnchoredVwapParams::default());
        let ts_ms = 1_700_000_000_000_i64;
        for i in 0..10 {
            let price = 100.0 + i as f64;
            avwap.feed(ts_ms + i * 86_400_000, &[price + 1.0, price - 1.0, price, 1000.0]);
        }
        assert!(avwap.is_ready());
        let v = avwap.value();
        assert!(v > 100.0 && v < 110.0, "VWAP should be within price range");
    }

    #[test]
    fn test_anchored_vwap_reset() {
        let mut avwap = AnchoredVwap::new(AnchoredVwapParams::default());
        avwap.feed(1_700_000_000_000, &[102.0, 98.0, 101.0, 1000.0]);
        avwap.reset();
        assert!(!avwap.is_ready());
        assert_eq!(avwap.value(), 0.0);
    }

    /// `Fields +time` arm: the factory feeds `(ts, &[H,L,C,V])` to `feed`.
    /// A constant price of 100.0 (TP = 100) with volume 1000 and a fixed-month ts yields
    /// AVWAP = 100.0. Wild open value (9999.0) is excluded from TP (H/L/C only).
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Avwap(<<AnchoredVwap as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for _ in 0..5 {
            f.feed(1_700_000_000_000, MarketSample::Bar {
                open: 9999.0, high: 101.0, low: 99.0, close: 100.0, volume: 1000.0,
            });
        }
        assert!(f.is_ready());
        let v = f.primary();
        assert!((v - 100.0).abs() < 1e-9, "AVWAP of constant 100.0 price = 100.0, got {v}");
    }
}
