//! Footprint Imbalance — detects price levels with extreme buy/sell skew.
//!
//! Scans all price buckets accumulated in the current bar and finds the level
//! with the largest signed imbalance `(buy - sell) / total`. Reports a signal
//! when any level's absolute imbalance exceeds `threshold_pct`.
//!
//! Outputs: `signal`, `imb_price`, `imb_pct` where:
//! - `signal`: `+1` if buy-dominated above threshold, `-1` if sell-dominated, `0` otherwise.
//! - `imb_price`: price of the level with maximum absolute imbalance.
//! - `imb_pct`: absolute imbalance percent at that level (0..100).
//!
//! Call `close_bar()` to finalize each bar. In-bar `update_tick` returns the
//! current cached value unchanged until the next `close_bar`.

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::tick_consumer::TickConsumer;
use crate::engine::matrix_grid::{AxisLabels, MatrixCell, MatrixGrid};
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity, ValueDomain};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::contract::sweep_f64;
use crate::engine::stream_kind::StreamKind;
use crate::types::Tick;
use std::collections::HashMap;

/// Fixed price-window (rows) of the imbalance-profile vector, centered on the POC/anchor bucket.
const IMB_WINDOW: u16 = 64;

/// Footprint Imbalance — signals price levels with extreme buy/sell skew.
#[derive(Debug, Clone)]
pub struct FootprintImbalance {
    price_bucket: f64,
    /// Trigger threshold in percent. E.g. 75.0 means signal when ≥ 75% one-sided.
    threshold_pct: f64,

    /// In-progress bar: bucket_index → (buy_vol, sell_vol).
    levels: HashMap<i64, (f64, f64)>,

    // ── Cached results from last closed bar ──────────────────────────────────
    /// -1 = sell-dominated extreme, +1 = buy-dominated extreme, 0 = none.
    last_signal: i8,
    /// Price of the level with maximum absolute imbalance.
    last_imb_price: f64,
    /// Absolute imbalance percent at that level.
    last_imb_pct: f64,
    /// Signed buy−sell imbalance per price bucket: `IMB_WINDOW × 1` centered on the
    /// anchor (max-imbalance) bucket. Cell value = `buy - sell` at that bucket.
    grid: MatrixGrid,
}

impl FootprintImbalance {
    /// `price_bucket`: price-level quantization step.
    /// `threshold_pct`: minimum imbalance percent to fire a signal (0..100).
    pub fn new(price_bucket: f64, threshold_pct: f64) -> Self {
        Self {
            price_bucket: price_bucket.max(1e-9),
            threshold_pct: threshold_pct.clamp(0.0, 100.0),
            levels: HashMap::new(),
            last_signal: 0,
            last_imb_price: 0.0,
            last_imb_pct: 0.0,
            grid: MatrixGrid::new(IMB_WINDOW, 1, false)
                .with_labels(AxisLabels::Values(Vec::new()), AxisLabels::Bars),
        }
    }

    /// Accumulate one tick into the in-progress bar.
    ///
    /// Eagerly recomputes `last_signal`, `last_imb_price`, and `last_imb_pct`
    /// from the live in-bar levels so that live consumers see non-zero values
    /// without waiting for `close_bar()`. `close_bar()` still finalises the bar.
    pub fn update_tick(&mut self, tick: &Tick) {
        let bucket = (tick.price / self.price_bucket).floor() as i64;
        let entry = self.levels.entry(bucket).or_insert((0.0, 0.0));
        if tick.is_buy {
            entry.0 += tick.size;
        } else {
            entry.1 += tick.size;
        }
        // Eagerly update cached fields from live in-bar state
        let mut max_signed_pct = 0.0f64;
        let mut max_price = 0.0f64;
        let mut max_total = 0.0f64;
        for (&bkt, &(buy, sell)) in &self.levels {
            let total = buy + sell;
            if total <= 0.0 {
                continue;
            }
            let signed_pct = ((buy - sell) / total) * 100.0;
            // Same %-then-volume tie-break as close_bar, so the live scalar matches the finalized one.
            let stronger = signed_pct.abs() > max_signed_pct.abs() + 1e-9;
            let tie_higher_vol = signed_pct.abs() > 1e-9
                && (signed_pct.abs() - max_signed_pct.abs()).abs() <= 1e-9
                && total > max_total;
            if stronger || tie_higher_vol {
                max_signed_pct = signed_pct;
                max_price = bkt as f64 * self.price_bucket;
                max_total = total;
            }
        }
        self.last_signal = if max_signed_pct >= self.threshold_pct {
            1
        } else if max_signed_pct <= -self.threshold_pct {
            -1
        } else {
            0
        };
        self.last_imb_price = max_price;
        self.last_imb_pct = max_signed_pct.abs();
    }

    /// Finalize bar: find the level with maximum signed imbalance, snapshot the imbalance grid,
    /// compare to threshold, then reset accumulation.
    pub fn close_bar(&mut self) {
        let mut max_signed_pct = 0.0f64;
        let mut max_price = 0.0f64;
        let mut anchor_bucket = 0i64;
        let mut max_total = 0.0f64;

        for (&bucket, &(buy, sell)) in &self.levels {
            let total = buy + sell;
            if total <= 0.0 {
                continue;
            }
            let signed_pct = ((buy - sell) / total) * 100.0; // −100..+100
            // Strongest by |imbalance %|; ties (e.g. 100% buy vs 100% sell levels) resolve to
            // the HIGHER-VOLUME level — deterministic and meaningful, vs HashMap iteration order.
            let stronger = signed_pct.abs() > max_signed_pct.abs() + 1e-9;
            let tie_higher_vol = signed_pct.abs() > 1e-9
                && (signed_pct.abs() - max_signed_pct.abs()).abs() <= 1e-9
                && total > max_total;
            if stronger || tie_higher_vol {
                max_signed_pct = signed_pct;
                max_price = bucket as f64 * self.price_bucket;
                anchor_bucket = bucket;
                max_total = total;
            }
        }

        self.last_signal = if max_signed_pct >= self.threshold_pct {
            1
        } else if max_signed_pct <= -self.threshold_pct {
            -1
        } else {
            0
        };
        self.last_imb_price = max_price;
        self.last_imb_pct = max_signed_pct.abs();

        // Snapshot signed buy−sell imbalance per bucket into the profile grid, centered on the
        // anchor (max-imbalance) bucket, before clearing the level map.
        self.grid.reset();
        let half = (IMB_WINDOW / 2) as i64;
        let mut row_prices = Vec::with_capacity(IMB_WINDOW as usize);
        for row in 0..IMB_WINDOW {
            let key = anchor_bucket + (row as i64 - half);
            row_prices.push(key as f64 * self.price_bucket);
            if let Some(&(buy, sell)) = self.levels.get(&key) {
                self.grid.set_direction(MatrixCell::new(row, 0), buy - sell);
            }
        }
        self.grid.set_row_values(&row_prices);

        self.levels.clear();
    }


    pub fn reset(&mut self) {
        self.levels.clear();
        self.last_signal = 0;
        self.last_imb_price = 0.0;
        self.last_imb_pct = 0.0;
        self.grid.reset();
    }

    pub fn is_ready(&self) -> bool {
        !self.levels.is_empty() || self.last_imb_pct > 0.0
    }

    /// Last signal: +1 buy extreme, -1 sell extreme, 0 neutral.
    pub fn signal(&self) -> i8 { self.last_signal }
    /// Named output getter: brace `direction` (the imbalance signal as f64: +1 buy / -1 sell / 0 neutral).
    pub fn direction(&self) -> f64 { self.last_signal as f64 }
    /// Price level with the maximum imbalance from the last closed bar.
    pub fn imbalance_price(&self) -> f64 { self.last_imb_price }
    /// Absolute imbalance percent from the last closed bar.
    pub fn imbalance_pct(&self) -> f64 { self.last_imb_pct }

    /// Named output: brace `imb_price`.
    pub fn imb_price(&self) -> f64 { self.last_imb_price }
    /// Named output: brace `imb_pct`.
    pub fn imb_pct(&self) -> f64 { self.last_imb_pct }

    /// Signed buy−sell imbalance per price bucket: `IMB_WINDOW × 1` centered on the anchor
    /// (max-imbalance) bucket. The matrix output behind `IndicatorOutputId::FootprintImbalanceProfileGrid`.
    pub fn profile_grid(&self) -> &MatrixGrid { &self.grid }
}

impl TickConsumer for FootprintImbalance {
    fn update_tick(&mut self, tick: &Tick) {
        FootprintImbalance::update_tick(self, tick)
    }
    fn reset(&mut self) { FootprintImbalance::reset(self) }
    fn is_ready(&self) -> bool { FootprintImbalance::is_ready(self) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn buy_tick(price: f64, qty: f64) -> Tick {
        Tick::new(0, price, qty, true)
    }

    fn sell_tick(price: f64, qty: f64) -> Tick {
        Tick::new(0, price, qty, false)
    }

    #[test]
    fn test_buy_extreme_signals_plus_one() {
        let mut fi = FootprintImbalance::new(1.0, 75.0);
        // 10 buy, 0 sell at price 100 → 100% buy
        for _ in 0..10 {
            fi.update_tick(&buy_tick(100.0, 1.0));
        }
        fi.close_bar();
        assert_eq!(fi.signal(), 1);
        assert!((fi.imbalance_pct() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn test_sell_extreme_signals_minus_one() {
        let mut fi = FootprintImbalance::new(1.0, 75.0);
        for _ in 0..10 {
            fi.update_tick(&sell_tick(100.0, 1.0));
        }
        fi.close_bar();
        assert_eq!(fi.signal(), -1);
    }

    #[test]
    fn test_balanced_no_signal() {
        let mut fi = FootprintImbalance::new(1.0, 75.0);
        // exactly 50/50
        for _ in 0..5 {
            fi.update_tick(&buy_tick(100.0, 1.0));
            fi.update_tick(&sell_tick(100.0, 1.0));
        }
        fi.close_bar();
        assert_eq!(fi.signal(), 0);
    }

    #[test]
    fn imbalance_profile_row_labels_are_prices() {
        use crate::engine::matrix_grid::Label;
        let mut fi = FootprintImbalance::new(1.0, 75.0);
        for _ in 0..10 { fi.update_tick(&buy_tick(100.0, 1.0)); }
        fi.close_bar();

        let g = fi.profile_grid();
        let center = IMB_WINDOW / 2;
        // Anchor bucket = 100; center row price label = 100.0
        assert_eq!(g.row_label(center), Label::Value(100.0));
    }

    #[test]
    fn test_reset_clears_state() {
        let mut fi = FootprintImbalance::new(1.0, 75.0);
        fi.update_tick(&buy_tick(100.0, 10.0));
        fi.close_bar();
        fi.reset();
        assert_eq!(fi.signal(), 0);
        assert_eq!(fi.imbalance_pct(), 0.0);
        assert!(!fi.is_ready());
    }
}

impl Default for FootprintImbalance {
    /// Factory default: price_bucket=0.01, threshold_pct=75.0.
    fn default() -> Self {
        Self::new(0.01, 75.0)
    }
}

/// Typed dual-mode config for [`FootprintImbalance`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FootprintImbalanceConfig {
    /// Price-level quantization step.
    pub price_bucket: Param<f64>,
    /// Minimum imbalance percent to fire a signal (0..100).
    pub threshold_pct: Param<f64>,
}

impl Indicator for FootprintImbalance {
    const ID: IndicatorId = IndicatorId::FootprintImbalance;
    /// Not a pluggable family — a tick-stream footprint signal detector.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Tick];
    /// Non-bar stream — no kline source axis.
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::FootprintImbalanceDirection),
        Output::price(IndicatorOutputId::FootprintImbalanceImbPrice),
        Output::percent(IndicatorOutputId::FootprintImbalanceImbPct),
        Output::matrix(IndicatorOutputId::FootprintImbalanceProfileGrid, ValueDomain::Centered),
    ];
    /// O(n levels) per tick (rescans all live bucket entries); uses a HashMap for levels.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    type Config = FootprintImbalanceConfig;
    type Runtime = Self;

    fn create(cfg: FootprintImbalanceConfig) -> Self {
        Self::new(cfg.price_bucket.resolved(), cfg.threshold_pct.resolved())
    }
}

impl crate::contract::Config for FootprintImbalanceConfig {
    fn defaults() -> Self {
        FootprintImbalanceConfig { price_bucket: Param::Solo(0.01), threshold_pct: Param::Solo(75.0) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // price_bucket: Class I (instrument-relative price granularity) — PIN.
        // f64 auto already leaves it Solo; no override needed.
        // threshold_pct: Class D by name pattern, but field is in 0..100 percent scale
        // (default 75.0, clamped 0..100 in ::new). Swept as 50..=95 step 5.0.
        s.threshold_pct = Param::many(sweep_f64(50.0, 95.0, 5.0));
        s
    }
}


impl Render for FootprintImbalance {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::FootprintImbalanceDirection, "Imbalance Dir", Color::hex(0xF44336))
            .line_output(IndicatorOutputId::FootprintImbalanceImbPrice, "Imb Price", Color::hex(0xFF9800))
            .line_output(IndicatorOutputId::FootprintImbalanceImbPct, "Imb %", Color::hex(0x9C27B0))
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
        let mut f = IndicatorOrder::FootprintImbalance(
            <<FootprintImbalance as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();

        // 10 pure buy ticks → 100% buy imbalance → signal = +1
        for _ in 0..10 {
            let t = CoreTick::new(0, 100.0, 1.0, true);
            f.feed(0, MarketSample::Tick(&t));
        }
        // value() = direction (signal) field = +1.0
        let v = f.primary();
        assert!((v - 1.0).abs() < 1e-9, "expected buy signal +1, got {}", v);
    }

    #[test]
    fn profile_grid_shape_and_signed_content() {
        let mut fi = FootprintImbalance::new(1.0, 75.0);
        // 10 buy at bucket 100 (the anchor), 4 sell at bucket 101
        for _ in 0..10 {
            fi.update_tick(&Tick::new(0, 100.0, 1.0, true));
        }
        for _ in 0..4 {
            fi.update_tick(&Tick::new(0, 101.0, 1.0, false));
        }
        fi.close_bar();

        let g = fi.profile_grid();
        assert_eq!(g.rows(), IMB_WINDOW, "64 price rows");
        assert_eq!(g.cols(), 1, "single imbalance column");
        // Anchor bucket is 100 (100% buy → strongest imbalance); center row = IMB_WINDOW/2
        let center = IMB_WINDOW / 2;
        // buy−sell at bucket 100 = 10 − 0 = 10
        assert_eq!(
            g.read_direction(MatrixCell::new(center, 0)),
            10.0,
            "anchor bucket: buy-sell = 10"
        );
        // Bucket 101 = one above anchor; sell-only: 0 − 4 = -4
        assert_eq!(
            g.read_direction(MatrixCell::new(center + 1, 0)),
            -4.0,
            "adjacent bucket: buy-sell = -4"
        );
    }

    #[test]
    fn factory_exposes_imbalance_profile_vector() {
        let f = IndicatorOrder::FootprintImbalance(
            <<FootprintImbalance as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()
        ).build_solo().unwrap();
        let g = f
            .grid(IndicatorOutputId::FootprintImbalanceProfileGrid)
            .expect("FootprintImbalance emits the imbalance profile vector");
        assert_eq!(g.rows(), 64);
        assert_eq!(g.cols(), 1);

        let sma = IndicatorOrder::from_defaults(IndicatorId::Sma).unwrap().build_solo().unwrap();
        assert!(sma.grid(IndicatorOutputId::FootprintImbalanceProfileGrid).is_none());
    }
}
