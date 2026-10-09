//! TpoSessionBalance — TPO (Time/Price Opportunity) session balance point.
//!
//! Consumer: `TickConsumer`.
//!
//! Logic:
//! - Maintains rolling tick history in a time window.
//! - Buckets prices by `price_bucket` width.
//! - Balance Point = price level (bucket midpoint) with maximum TPO count.
//!
//! Output: `Triple(balance_price, max_tpo_count, total_buckets)`.

use std::collections::HashMap;
use std::collections::VecDeque;

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

/// Fixed price-window (rows) of the exposed TPO-count-by-price VECTOR, centered on the POC bucket.
const TSB_WINDOW: u16 = 64;

/// TPO session balance point indicator.
///
/// Implements `TickConsumer`.
/// Parameters:
/// - `window_ms`   — rolling time window in milliseconds.
/// - `price_bucket`— bucket width in price units.
#[derive(Debug, Clone)]
pub struct TpoSessionBalance {
    window_ms: i64,
    price_bucket: f64,
    events: VecDeque<(i64, f64)>, // (ts, price)
    last_balance: f64,
    last_max_count: f64,
    last_buckets: f64,
    /// The TPO-count-by-price histogram as a `TSB_WINDOW × 1` VECTOR (row = price bucket centered
    /// on the POC, value = TPO count). Read via `profile_grid` /
    /// `ContractFactory::grid(IndicatorOutputId::TpoSessionBalanceProfileGrid)`.
    grid: MatrixGrid,
}

impl TpoSessionBalance {
    /// Create a new indicator.
    pub fn new(window_ms: i64, price_bucket: f64) -> Self {
        Self {
            window_ms: window_ms.max(1),
            price_bucket: price_bucket.max(f64::EPSILON),
            events: VecDeque::with_capacity(512),
            last_balance: 0.0,
            last_max_count: 0.0,
            last_buckets: 0.0,
            grid: MatrixGrid::new(TSB_WINDOW, 1, false)
                .with_labels(AxisLabels::Values(Vec::new()), AxisLabels::Bars),
        }
    }

    /// Balance point price — bucket midpoint with maximum TPO count.
    pub fn price(&self) -> f64 {
        self.last_balance
    }

    /// Maximum TPO count across all buckets.
    pub fn max_count(&self) -> f64 {
        self.last_max_count
    }

    /// Number of distinct price buckets in the current window.
    pub fn buckets(&self) -> f64 {
        self.last_buckets
    }

    fn price_to_bucket(price: f64, bucket: f64) -> i64 {
        (price / bucket).floor() as i64
    }

    fn recompute(&mut self, fallback_price: f64) {
        if self.events.is_empty() {
            self.last_balance = fallback_price;
            self.last_max_count = 0.0;
            self.last_buckets = 0.0;
            return;
        }

        let mut counts: HashMap<i64, u64> = HashMap::new();
        for &(_, p) in &self.events {
            *counts.entry(Self::price_to_bucket(p, self.price_bucket)).or_insert(0) += 1;
        }

        let (&poc_bucket, &max_count) = counts
            .iter()
            .max_by_key(|&(_, &c)| c)
            .unwrap(); // safe: non-empty

        let balance_price = poc_bucket as f64 * self.price_bucket + self.price_bucket / 2.0;
        self.last_balance = balance_price;
        self.last_max_count = max_count as f64;
        self.last_buckets = counts.len() as f64;

        // Snapshot the TPO-count-by-price histogram into the fixed price-window vector (rows
        // centered on the POC bucket).
        self.grid.reset();
        let half = (TSB_WINDOW / 2) as i64;
        let pb = self.price_bucket;
        let mut row_prices = Vec::with_capacity(TSB_WINDOW as usize);
        for row in 0..TSB_WINDOW {
            let key = poc_bucket + (row as i64 - half);
            let price = key as f64 * pb + pb / 2.0;
            row_prices.push(price);
            if let Some(&cnt) = counts.get(&key) {
                self.grid.set_direction(MatrixCell::new(row, 0), cnt as f64);
            }
        }
        self.grid.set_row_values(&row_prices);
    }

    /// The TPO-count-by-price profile as a `TSB_WINDOW × 1` vector — the matrix output behind
    /// `IndicatorOutputId::TpoSessionBalanceProfileGrid`.
    pub fn profile_grid(&self) -> &MatrixGrid { &self.grid }


    /// True when at least one tick has been received.
    pub fn indicator_is_ready(&self) -> bool {
        !self.events.is_empty()
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.events.clear();
        self.last_balance = 0.0;
        self.last_max_count = 0.0;
        self.last_buckets = 0.0;
        self.grid.reset();
    }
}

impl Default for TpoSessionBalance {
    fn default() -> Self {
        Self::new(3_600_000, 1.0)
    }
}

// ---- Indicator contract ----

use crate::contract::Param;

/// Typed configuration for [`TpoSessionBalance`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct TpoSessionBalanceConfig {
    pub window: Param<TimeWindow>,
    /// Bucket width in price units.
    pub price_bucket: Param<f64>,
}

impl Indicator for TpoSessionBalance {
    const ID: IndicatorId = IndicatorId::TpoSessionBalance;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::TpoSessionBalancePrice),
        Output::count(IndicatorOutputId::TpoSessionBalanceMaxCount),
        Output::count(IndicatorOutputId::TpoSessionBalanceBuckets),
        Output::matrix(IndicatorOutputId::TpoSessionBalanceProfileGrid, ValueDomain::Count),
    ];
    type Config = TpoSessionBalanceConfig;
    type Runtime = TpoSessionBalance;

    fn create(cfg: TpoSessionBalanceConfig) -> TpoSessionBalance {
        TpoSessionBalance::new(cfg.window.resolved().as_millis(), cfg.price_bucket.resolved())
    }
}

impl crate::contract::Config for TpoSessionBalanceConfig {
    fn defaults() -> Self {
        TpoSessionBalanceConfig {
            window: Param::Solo(TimeWindow::Hours(1)),
            price_bucket: Param::Solo(1.0),
        }
    }
    fn machine_defaults() -> Self {
        // window (TimeWindow): Class N — 11-point curated discrete set.
        // price_bucket (f64): Class I — instrument-relative bucket width; NOT swept (PIN).
        //   Left as Solo from auto (f64 Solo by default from machine_defaults_auto).
        let mut s = Self::machine_defaults_auto();
        s.window = Param::many(vec![
            TimeWindow::Seconds(1), TimeWindow::Seconds(5), TimeWindow::Seconds(15),
            TimeWindow::Seconds(30), TimeWindow::Minutes(1), TimeWindow::Minutes(5),
            TimeWindow::Minutes(15), TimeWindow::Minutes(30),
            TimeWindow::Hours(1), TimeWindow::Hours(4), TimeWindow::Hours(24),
        ]);
        // price_bucket: Class I pin — left Solo (auto keeps f64 as Solo).
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for TpoSessionBalance {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::TpoSessionBalancePrice, "Balance Price", Color::hex(0xFF9800), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::TpoSessionBalanceMaxCount, "Max TPO Count", Color::hex(0x42A5F5), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::TpoSessionBalanceBuckets, "Total Buckets", Color::hex(0x78909C), 1.0))
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
    fn factory_feeds_resolved_tpo_session_balance() {
        let mut f = IndicatorOrder::TpoSessionBalance(
            <<TpoSessionBalance as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let t = crate::core::types::Tick::new(0, 100.0, 1.0, true);
        f.feed(0, MarketSample::Tick(&t));
        assert!(f.primary().is_finite());
    }

    #[test]
    fn row_label_at_center_is_poc_price() {
        use crate::engine::matrix_grid::Label;
        let bucket_size = 10.0_f64;
        let mut ind = TpoSessionBalance::new(60_000, bucket_size);
        // 5 ticks at 105 → bucket 10 (floor(105/10)=10); midpoint = 10*10 + 10/2 = 105.
        for i in 0..5i64 {
            ind.update_tick(&Tick::new(i * 100, 105.0, 1.0, true));
        }
        ind.update_tick(&Tick::new(600, 205.0, 1.0, true)); // different bucket
        let center = TSB_WINDOW / 2;
        let label = ind.profile_grid().row_label(center);
        let expected = 105.0_f64;
        if let Label::Value(p) = label {
            assert!((p - expected).abs() < bucket_size, "row_label at center = {p}, expected ≈ {expected}");
        } else {
            panic!("expected Label::Value, got {:?}", label);
        }
    }

    #[test]
    fn profile_vector_captures_tpo_count_around_poc() {
        let mut ind = TpoSessionBalance::new(60_000, 10.0);
        // 5 ticks at price 105 → bucket 10 (midpoint 105) becomes POC.
        for i in 0..5i64 {
            ind.update_tick(&Tick::new(i * 100, 105.0, 1.0, true));
        }
        // 1 tick at price 205 — different bucket.
        ind.update_tick(&Tick::new(600, 205.0, 1.0, true));

        let g = ind.profile_grid();
        assert_eq!(g.cols(), 1, "TPO-count-by-price vector has exactly 1 column");
        assert_eq!(g.rows(), TSB_WINDOW, "vector height == TSB_WINDOW");
        // POC bucket sits at center row; its TPO count must equal 5.
        let center = TSB_WINDOW / 2;
        assert_eq!(
            g.read_direction(MatrixCell::new(center, 0)) as u64,
            5,
            "POC bucket at center row should carry TPO count 5"
        );
    }

    #[test]
    fn factory_exposes_tpo_profile_vector() {
        let f = IndicatorOrder::TpoSessionBalance(
            <<TpoSessionBalance as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        let g = f
            .grid(IndicatorOutputId::TpoSessionBalanceProfileGrid)
            .expect("TpoSessionBalance emits the tpo profile vector");
        assert_eq!(g.rows(), 64);
        assert_eq!(g.cols(), 1);

        let sma = IndicatorOrder::from_defaults(crate::engine::indicator_id::IndicatorId::Sma)
            .unwrap()
            .build_solo()
            .unwrap();
        assert!(sma.grid(IndicatorOutputId::TpoSessionBalanceProfileGrid).is_none());
    }
}

impl TickConsumer for TpoSessionBalance {
    fn update_tick(&mut self, tick: &Tick) {
        // Evict stale events
        let cutoff = tick.time - self.window_ms;
        while self.events.front().map_or(false, |(ts, _)| *ts < cutoff) {
            self.events.pop_front();
        }
        self.events.push_back((tick.time, tick.price));
        self.recompute(tick.price);
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(time_ms: i64, price: f64) -> Tick {
        Tick::new(time_ms, price, 1.0, true)
    }

    #[test]
    fn balance_at_dominant_price_bucket() {
        let mut ind = TpoSessionBalance::new(60_000, 10.0);
        // 5 ticks at 100 (bucket 10), 1 at 200 (bucket 20)
        for i in 0..5 {
            ind.update_tick(&tick(i * 100, 105.0)); // bucket 10: midpoint 105
        }
        ind.update_tick(&tick(600, 205.0)); // bucket 20
        let balance = ind.price();
        let max_count = ind.max_count();
        let buckets = ind.buckets();
        assert!((balance - 105.0).abs() < 1.0, "balance={balance}");
        assert_eq!(max_count as u64, 5, "max_count={max_count}");
        assert_eq!(buckets as u64, 2, "buckets={buckets}");
    }

    #[test]
    fn stale_events_evicted() {
        let mut ind = TpoSessionBalance::new(10_000, 10.0);
        // 5 ticks at 100 at t=0
        for i in 0..5 {
            ind.update_tick(&tick(i * 100, 105.0));
        }
        // new tick 20s later at 200 — old events evicted
        ind.update_tick(&tick(20_000, 205.0));
        let buckets = ind.buckets();
        assert_eq!(buckets as u64, 1, "only 1 bucket should remain");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = TpoSessionBalance::new(60_000, 10.0);
        ind.update_tick(&tick(1000, 100.0));
        assert!(ind.indicator_is_ready());
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        assert_eq!(ind.price(), 0.0);
        assert_eq!(ind.max_count(), 0.0);
        assert_eq!(ind.buckets(), 0.0);
    }
}
