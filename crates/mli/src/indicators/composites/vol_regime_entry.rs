//! VolRegimeEntry — volatility regime transition entry signal.
//!
//! Dual consumer: `VolatilityIndexConsumer` + `MarkPriceConsumer`.
//!
//! Logic:
//! - Maintains rolling vol_idx history to compute p25 / p75 percentile thresholds.
//! - `high_regime` = current > p75; `low_regime` = current < p25.
//! - `price_momentum` = (last_price - prev_price) / prev_price.
//! - Signal:
//!   - vol_idx crosses into high regime AND price falling → `+1` (vol strategy entry)
//!   - vol_idx drops out of high regime → `-1` (vol strategy exit)
//!   - Otherwise → `0`
//!
//! Output: `Signal(i8)`.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
use crate::engine::streams::volatility_index_consumer::VolatilityIndexConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, HistogramStyle, RenderOutput, RenderSpec};
use crate::core::types::{MarkPrice, VolatilityIndex};
use crate::engine::stream_kind::StreamKind;

const MIN_HISTORY: usize = 4;

/// Volatility regime entry/exit signal.
///
/// Implements both `VolatilityIndexConsumer` and `MarkPriceConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Debug, Clone)]
pub struct VolRegimeEntry {
    vol_history: VecDeque<f64>,
    max_history: usize,
    last_vol: f64,
    prev_vol: f64,
    last_price: f64,
    prev_price: f64,
    was_high_regime: bool,
    last_signal: i8,
}

impl VolRegimeEntry {
    /// Create a new indicator.
    ///
    /// - `history_len` — rolling window for percentile computation (minimum 4, default 20).
    pub fn new(history_len: usize) -> Self {
        let cap = history_len.max(MIN_HISTORY);
        Self {
            vol_history: VecDeque::with_capacity(cap),
            max_history: cap,
            last_vol: 0.0,
            prev_vol: 0.0,
            last_price: 0.0,
            prev_price: 0.0,
            was_high_regime: false,
            last_signal: 0,
        }
    }

    fn percentile_of(&self, sorted_slice: &[f64], pct: f64) -> f64 {
        if sorted_slice.is_empty() {
            return 0.0;
        }
        let idx = ((sorted_slice.len() as f64) * pct).floor() as usize;
        sorted_slice[idx.min(sorted_slice.len().saturating_sub(1))]
    }

    fn recompute(&mut self) {
        // Need at least MIN_HISTORY history entries plus a current value to compare.
        if self.vol_history.len() < MIN_HISTORY || self.last_vol <= 0.0 {
            self.last_signal = 0;
            return;
        }

        // Compute percentile thresholds from stored history (does NOT include last_vol).
        // This avoids the current value contaminating its own threshold.
        let mut sorted: Vec<f64> = self.vol_history.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p75 = self.percentile_of(&sorted, 0.75);

        let is_high = self.last_vol >= p75;

        let price_falling = self.prev_price > 0.0 && self.last_price < self.prev_price;

        self.last_signal = if is_high && !self.was_high_regime && price_falling {
            1  // entering high vol + price falling → vol strategy entry
        } else if !is_high && self.was_high_regime {
            -1 // leaving high vol → exit signal
        } else {
            0
        };

        self.was_high_regime = is_high;
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> f64 {
        self.last_signal as f64
    }

    /// True when vol history has enough data for percentile computation.
    pub fn indicator_is_ready(&self) -> bool {
        self.vol_history.len() >= MIN_HISTORY
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.vol_history.clear();
        self.last_vol = 0.0;
        self.prev_vol = 0.0;
        self.last_price = 0.0;
        self.prev_price = 0.0;
        self.was_high_regime = false;
        self.last_signal = 0;
    }
}

impl Default for VolRegimeEntry {
    fn default() -> Self {
        Self::new(20)
    }
}

impl VolatilityIndexConsumer for VolRegimeEntry {
    fn update_volatility_index(&mut self, vi: &VolatilityIndex) {
        self.prev_vol = self.last_vol;
        self.last_vol = vi.value;
        // Push prev_vol to history (history contains previous values only).
        // This prevents the current reading from contaminating its own percentile threshold.
        if self.prev_vol > 0.0 {
            if self.vol_history.len() >= self.max_history {
                self.vol_history.pop_front();
            }
            self.vol_history.push_back(self.prev_vol);
        }
        self.recompute();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl MarkPriceConsumer for VolRegimeEntry {
    fn update_mark(&mut self, mp: &MarkPrice) {
        self.prev_price = self.last_price;
        self.last_price = mp.mark_price;
        self.recompute();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

/// Typed configuration for [`VolRegimeEntry`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VolRegimeEntryConfig {
    /// Rolling window size for percentile computation (minimum 4).
    pub history_len: Param<usize>,
}

impl Indicator for VolRegimeEntry {
    const ID: IndicatorId = IndicatorId::VolRegimeEntry;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::VolatilityIndex, StreamKind::MarkPrice];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::VolRegimeEntry)];
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear, // percentile sort on vol_history each update
        &[],
    );
    type Config = VolRegimeEntryConfig;
    type Runtime = VolRegimeEntry;

    fn create(cfg: VolRegimeEntryConfig) -> VolRegimeEntry {
        VolRegimeEntry::new(cfg.history_len.resolved())
    }
}

impl crate::contract::Config for VolRegimeEntryConfig {
    fn defaults() -> Self {
        VolRegimeEntryConfig { history_len: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // history_len (usize): Class B cap — rolling window for percentile computation;
        //   minimum 4 (hardcoded), default 20. Auto would give 2..4048 too wide.
        //   Corrected to 4..=100 step 1 — meaningful range for a percentile computation cap.
        let mut s = Self::machine_defaults_auto();
        s.history_len = Param::range(4, 100, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VolRegimeEntry {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(
                IndicatorOutputId::VolRegimeEntry,
                "Vol Regime Entry Signal",
                Color::hex(0xFF9800),
            ))
            .bounds(-1.5, 1.5)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_vi(value: f64) -> VolatilityIndex {
        VolatilityIndex { value, timestamp: 1000, ..Default::default()}
    }

    fn make_mp(mark_price: f64) -> MarkPrice {
        MarkPrice { mark_price, index_price: None, funding_rate: None, timestamp: 1000, ..Default::default()}
    }

    #[test]
    fn not_ready_before_min_history() {
        let mut ind = VolRegimeEntry::new(4);
        // With prev-value history model, need 5 updates to fill 4-entry history
        // (1st update has no prev to push, 2nd–5th push values → 4 entries).
        for i in 0..4 {
            ind.update_volatility_index(&make_vi(0.3));
            assert!(!ind.indicator_is_ready(), "should not be ready after {} updates", i + 1);
        }
        // 5th update fills history to MIN_HISTORY=4
        ind.update_volatility_index(&make_vi(0.3));
        assert!(ind.indicator_is_ready());
    }

    #[test]
    fn high_vol_entry_with_falling_price_gives_plus_one() {
        let mut ind = VolRegimeEntry::new(4);
        // Feed 4 low vol readings to establish history (4th update gives 3-entry history)
        for _ in 0..4 {
            ind.update_volatility_index(&make_vi(0.20));
        }
        // Set up falling price
        ind.update_mark(&make_mp(30000.0));
        ind.update_mark(&make_mp(29000.0));

        // 5th vol update: pushes 4th val (0.20) → history=[0.20,0.20,0.20,0.20], ready.
        // last_vol=0.90 >> p75=0.20 → is_high=true, was_high=false → entry signal.
        // price_falling: prev_price=30000 > last_price=29000 → true.
        ind.update_volatility_index(&make_vi(0.90));
        let s = ind.indicator_value().round() as i8;
        assert_eq!(s, 1, "expected +1 on high vol entry with falling price, got {s}");
    }

    #[test]
    fn exit_from_high_vol_gives_minus_one() {
        let mut ind = VolRegimeEntry::new(4);
        // Feed 4 high-vol readings (after 5th update, history has 4 high-vol entries)
        for _ in 0..4 {
            ind.update_volatility_index(&make_vi(0.80));
        }
        // 5th update: history=[0.80,0.80,0.80,0.80] ready; last_vol=0.80 >= p75=0.80 → was_high=true
        ind.update_volatility_index(&make_vi(0.80));
        assert!(ind.indicator_is_ready());

        // Non-falling price (exit signal doesn't require falling price)
        ind.update_mark(&make_mp(30000.0));
        ind.update_mark(&make_mp(30000.0));

        // 6th update: last_vol=0.10. history=[0.80,0.80,0.80,0.80]. p75=0.80.
        // is_high=(0.10>=0.80)=false. was_high=true → exit signal -1.
        ind.update_volatility_index(&make_vi(0.10));
        let s = ind.indicator_value().round() as i8;
        assert_eq!(s, -1, "expected -1 on exit from high regime, got {s}");
    }

    #[test]
    fn factory_feeds_vol_regime_entry() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::VolRegimeEntry(VolRegimeEntryConfig { history_len: Param::Solo(4) })
            .build_solo()
            .unwrap();
        // Prime with 4 low-vol readings via factory feed
        for _ in 0..4 {
            let vi = make_vi(0.20);
            f.feed(0, MarketSample::VolatilityIndex(&vi));
        }
        // Falling price setup
        let mp_hi = make_mp(30000.0);
        let mp_lo = make_mp(29000.0);
        f.feed(0, MarketSample::MarkPrice(&mp_hi));
        f.feed(0, MarketSample::MarkPrice(&mp_lo));
        // High-vol spike → entry signal
        let vi_spike = make_vi(0.90);
        f.feed(0, MarketSample::VolatilityIndex(&vi_spike));
        let s = f.primary().round() as i8;
        assert_eq!(s, 1, "expected +1 vol regime entry signal, got {s}");
    }
}
