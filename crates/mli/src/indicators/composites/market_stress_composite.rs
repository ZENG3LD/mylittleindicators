//! MarketStressComposite — composite stress score from volatility, liquidations,
//! funding rate, and insurance fund depletion.
//!
//! Quad consumer: `VolatilityIndexConsumer` + `LiquidationConsumer` +
//! `FundingRateConsumer` + `InsuranceFundConsumer`.
//!
//! Formula:
//! - `vol_score`   = clamp(current_vol / rolling_95p_vol, 0, 1)  (0.3 weight)
//! - `liq_score`   = clamp(liq_count / max_expected_liq, 0, 1)   (0.3 weight)
//! - `fund_score`  = clamp(|funding_rate| × 100, 0, 1)           (0.2 weight)
//! - `depletion`   = 1.0 if recent fund slope < -threshold, else 0.0 (0.2 weight)
//! - `stress`      = 0.3×vol + 0.3×liq + 0.2×fund + 0.2×depletion
//!
//! Output: `Single(stress_score)` ∈ [0, 1].

use std::collections::VecDeque;

use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::insurance_fund_consumer::InsuranceFundConsumer;
use crate::engine::streams::liquidation_consumer::LiquidationConsumer;
use crate::engine::streams::volatility_index_consumer::VolatilityIndexConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::core::types::{FundingRate, InsuranceFund, Liquidation, VolatilityIndex};
use crate::engine::stream_kind::StreamKind;

/// Composite market stress score.
///
/// Implements `VolatilityIndexConsumer`, `LiquidationConsumer`,
/// `FundingRateConsumer`, and `InsuranceFundConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Debug, Clone)]
pub struct MarketStressComposite {
    window_ms: i64,
    max_expected_liq: f64,
    depletion_slope_threshold: f64,

    vol_history: VecDeque<f64>,
    vol_history_cap: usize,
    current_vol: f64,

    liq_events: VecDeque<i64>,

    last_funding_abs: f64,

    fund_history: VecDeque<(i64, f64)>,

    last_stress: f64,
}

impl MarketStressComposite {
    /// Create a new indicator.
    ///
    /// - `window_ms`               — rolling window for liq events (default 60_000)
    /// - `max_expected_liq`        — expected liquidations per window (default 10.0)
    /// - `vol_history_cap`         — number of vol samples for 95th percentile (default 100)
    /// - `depletion_slope_threshold` — slope (balance/ms) below which fund is depleting (default -1e-6)
    pub fn new(
        window_ms: i64,
        max_expected_liq: f64,
        vol_history_cap: usize,
        depletion_slope_threshold: f64,
    ) -> Self {
        Self {
            window_ms,
            max_expected_liq: max_expected_liq.max(1.0),
            depletion_slope_threshold,
            vol_history: VecDeque::with_capacity(vol_history_cap),
            vol_history_cap: vol_history_cap.max(4),
            current_vol: 0.0,
            liq_events: VecDeque::new(),
            last_funding_abs: 0.0,
            fund_history: VecDeque::with_capacity(10),
            last_stress: 0.0,
        }
    }

    fn percentile_95(&self) -> f64 {
        if self.vol_history.is_empty() {
            return 1.0;
        }
        let mut sorted: Vec<f64> = self.vol_history.iter().copied().collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let idx = ((sorted.len() as f64 * 0.95) as usize).min(sorted.len().saturating_sub(1));
        sorted[idx].max(1e-12)
    }

    fn fund_depletion_signal(&self) -> f64 {
        if self.fund_history.len() < 2 {
            return 0.0;
        }
        let first = self.fund_history.front().copied().unwrap_or((0, 0.0));
        let last = self.fund_history.back().copied().unwrap_or((0, 0.0));
        let dt = (last.0 - first.0).max(1) as f64;
        let slope = (last.1 - first.1) / dt;
        if slope < self.depletion_slope_threshold {
            1.0
        } else {
            0.0
        }
    }

    fn recompute(&mut self) {
        let p95 = self.percentile_95();
        let vol_score = (self.current_vol / p95).clamp(0.0, 1.0);

        let liq_score = (self.liq_events.len() as f64 / self.max_expected_liq).clamp(0.0, 1.0);

        let fund_score = (self.last_funding_abs * 100.0).clamp(0.0, 1.0);

        let depletion = self.fund_depletion_signal();

        self.last_stress = 0.3 * vol_score + 0.3 * liq_score + 0.2 * fund_score + 0.2 * depletion;
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> f64 {
        self.last_stress
    }

    /// True when at least one update from each stream has arrived.
    pub fn indicator_is_ready(&self) -> bool {
        self.current_vol > 0.0 || !self.liq_events.is_empty()
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.vol_history.clear();
        self.current_vol = 0.0;
        self.liq_events.clear();
        self.last_funding_abs = 0.0;
        self.fund_history.clear();
        self.last_stress = 0.0;
    }
}

impl Default for MarketStressComposite {
    fn default() -> Self {
        Self::new(60_000, 10.0, 100, -1e-6)
    }
}

impl VolatilityIndexConsumer for MarketStressComposite {
    fn update_volatility_index(&mut self, vi: &VolatilityIndex) {
        self.current_vol = vi.value;
        if self.vol_history.len() >= self.vol_history_cap {
            self.vol_history.pop_front();
        }
        self.vol_history.push_back(vi.value);
        self.recompute();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl LiquidationConsumer for MarketStressComposite {
    fn update_liquidation(&mut self, liq: &Liquidation) {
        let cutoff = liq.timestamp - self.window_ms;
        while self.liq_events.front().map_or(false, |ts| *ts < cutoff) {
            self.liq_events.pop_front();
        }
        self.liq_events.push_back(liq.timestamp);
        self.recompute();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl FundingRateConsumer for MarketStressComposite {
    fn update_funding(&mut self, fr: &FundingRate) {
        self.last_funding_abs = fr.rate.abs();
        self.recompute();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl InsuranceFundConsumer for MarketStressComposite {
    fn update_insurance_fund(&mut self, ins: &InsuranceFund) {
        if self.fund_history.len() >= 10 {
            self.fund_history.pop_front();
        }
        self.fund_history.push_back((ins.timestamp, ins.balance));
        self.recompute();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

/// Typed configuration for [`MarketStressComposite`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct MarketStressCompositeConfig {
    /// Rolling window for liquidation event counting, in milliseconds.
    pub window_ms: Param<i64>,
    /// Expected maximum liquidations per window (denominator for liq_score).
    pub max_expected_liq: Param<f64>,
    /// Number of vol samples retained for 95th-percentile computation.
    pub vol_history_cap: Param<usize>,
    /// Insurance fund slope threshold below which depletion signal fires.
    pub depletion_slope_threshold: Param<f64>,
}

impl Indicator for MarketStressComposite {
    const ID: IndicatorId = IndicatorId::MarketStressComposite;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[
        StreamKind::VolatilityIndex,
        StreamKind::Liquidation,
        StreamKind::Funding,
        StreamKind::InsuranceFund,
    ];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::MarketStressComposite)];
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Deque)],
    );
    type Config = MarketStressCompositeConfig;
    type Runtime = MarketStressComposite;

    fn create(cfg: MarketStressCompositeConfig) -> MarketStressComposite {
        MarketStressComposite::new(
            cfg.window_ms.resolved(),
            cfg.max_expected_liq.resolved(),
            cfg.vol_history_cap.resolved(),
            cfg.depletion_slope_threshold.resolved(),
        )
    }
}

impl crate::contract::Config for MarketStressCompositeConfig {
    fn defaults() -> Self {
        MarketStressCompositeConfig {
            window_ms: Param::Solo(60_000),
            max_expected_liq: Param::Solo(10.0),
            vol_history_cap: Param::Solo(100),
            depletion_slope_threshold: Param::Solo(-1e-6),
        }
    }
    fn machine_defaults() -> Self {
        // window_ms (i64): Class M raw-ms — canonical named durations (§3.8)
        // max_expected_liq (f64): ambiguous — normalization denominator for expected liq events per
        //   window (default 10.0). Swept as a discrete set of plausible event-count denominators.
        // vol_history_cap (usize): Class B cap — number of vol samples for 95th-pct computation;
        //   auto would give 2..4048 which is too wide. Corrected to 10..=500 step 10.
        // depletion_slope_threshold (f64): ambiguous — negative slope of insurance fund balance/ms.
        //   Default -1e-6. Dimensioned (balance units / ms) — instrument-relative. Swept over a
        //   log-spaced set of negative slope thresholds covering typical depletion rates.
        let mut s = Self::machine_defaults_auto();
        s.window_ms = Param::many(vec![1000i64, 5000, 10000, 30000, 60000, 300000, 3600000]);
        s.max_expected_liq = Param::many(vec![1.0f64, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0]);
        s.vol_history_cap = Param::range(10, 500, 10);
        s.depletion_slope_threshold = Param::many(vec![-1e-4f64, -1e-5, -1e-6, -1e-7, -1e-8]);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for MarketStressComposite {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::MarketStressComposite, "Market Stress", Color::hex(0xFF5722))
            .bounds(0.0, 1.0)
            .precision(3)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::TradeSide;

    fn make_vi(value: f64) -> VolatilityIndex {
        VolatilityIndex { value, timestamp: 1000, ..Default::default()}
    }

    fn make_liq(ts: i64) -> Liquidation {
        Liquidation { symbol: String::new(), side: TradeSide::Buy, price: 30000.0, quantity: 0.1, timestamp: ts, value: None, ..Default::default()}
    }

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate { rate, next_funding_time: None, timestamp: 1000, ..Default::default()}
    }

    fn make_ins(balance: f64, ts: i64) -> InsuranceFund {
        InsuranceFund { balance, timestamp: ts }
    }

    #[test]
    fn stress_in_range() {
        let mut ind = MarketStressComposite::new(60_000, 5.0, 20, -1e-6);
        ind.update_volatility_index(&make_vi(0.5));
        ind.update_volatility_index(&make_vi(1.0));
        ind.update_liquidation(&make_liq(1000));
        ind.update_liquidation(&make_liq(2000));
        ind.update_funding(&make_fr(0.001));
        let s = ind.indicator_value();
        assert!(s >= 0.0 && s <= 1.0, "stress={s}");
    }

    #[test]
    fn high_funding_increases_stress() {
        let mut ind_low = MarketStressComposite::new(60_000, 5.0, 20, -1e-6);
        let mut ind_high = MarketStressComposite::new(60_000, 5.0, 20, -1e-6);
        ind_low.update_funding(&make_fr(0.0001));
        ind_high.update_funding(&make_fr(0.01));
        let s_low = ind_low.indicator_value();
        let s_high = ind_high.indicator_value();
        assert!(s_high > s_low, "high={s_high} low={s_low}");
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = MarketStressComposite::default();
        ind.update_volatility_index(&make_vi(1.0));
        ind.update_liquidation(&make_liq(1000));
        ind.update_funding(&make_fr(0.01));
        ind.update_insurance_fund(&make_ins(1000.0, 1000));
        ind.indicator_reset();
        let s = ind.indicator_value();
        assert_eq!(s, 0.0);
    }

    #[test]
    fn depletion_signal_triggers_on_steep_decline() {
        let mut ind = MarketStressComposite::new(60_000, 5.0, 20, -1e-3);
        // Steep decline: 1_000_000 → 1 over 1000 ms → slope ~ -999.999/ms → < -1e-3
        ind.update_insurance_fund(&make_ins(1_000_000.0, 0));
        ind.update_insurance_fund(&make_ins(1.0, 1000));
        let s = ind.indicator_value();
        // depletion component = 0.2 × 1.0 = 0.2, so stress >= 0.2
        assert!(s >= 0.19, "stress={s}");
    }
}
