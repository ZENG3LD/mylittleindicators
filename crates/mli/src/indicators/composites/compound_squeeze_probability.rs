//! CompoundSqueezeProbability — 4-factor squeeze probability with funding rate.
//!
//! Quad consumer: `OpenInterestConsumer` + `LiquidationConsumer` +
//! `MarkPriceConsumer` + `FundingRateConsumer`.
//!
//! Formula:
//! - `oi_drop_score`       = clamp(|dOI/oi| × 10, 0, 1)            (weight 0.3)
//! - `liq_rate_score`      = clamp(liq_count / max_liq, 0, 1)       (weight 0.25)
//! - `price_momentum_score`= clamp(|dPrice/price| × 100, 0, 1)      (weight 0.25)
//! - `funding_extreme_score` = clamp(|funding| × 1000, 0, 1)        (weight 0.2)
//! - `probability`         = weighted sum
//! - `direction`           = sign of price move
//!
//! Output: `Double(probability, direction)`.

use std::collections::VecDeque;

use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::liquidation_consumer::LiquidationConsumer;
use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
use crate::engine::streams::open_interest_consumer::OpenInterestConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::core::types::{FundingRate, Liquidation, MarkPrice, OpenInterest};

/// Compound squeeze probability with 4 factors.
///
/// Implements `OpenInterestConsumer`, `LiquidationConsumer`,
/// `MarkPriceConsumer`, and `FundingRateConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Debug, Clone)]
pub struct CompoundSqueezeProbability {
    window_ms: i64,
    max_expected_liq: f64,

    oi_history: VecDeque<(i64, f64)>,
    price_history: VecDeque<(i64, f64)>,
    liq_events: VecDeque<i64>,
    last_funding_abs: f64,

    last_prob: f64,
    last_direction: f64,
}

impl CompoundSqueezeProbability {
    /// Create a new indicator.
    ///
    /// - `window_ms`       — rolling time window in milliseconds (default 60_000)
    /// - `max_expected_liq`— expected liquidations per window for normalization (default 10.0)
    pub fn new(window_ms: i64, max_expected_liq: f64) -> Self {
        Self {
            window_ms,
            max_expected_liq: max_expected_liq.max(1.0),
            oi_history: VecDeque::new(),
            price_history: VecDeque::new(),
            liq_events: VecDeque::new(),
            last_funding_abs: 0.0,
            last_prob: 0.0,
            last_direction: 0.0,
        }
    }

    fn evict_oi(&mut self, now: i64) {
        let cutoff = now - self.window_ms;
        while self.oi_history.front().map_or(false, |(ts, _)| *ts < cutoff) {
            self.oi_history.pop_front();
        }
    }

    fn evict_price(&mut self, now: i64) {
        let cutoff = now - self.window_ms;
        while self.price_history.front().map_or(false, |(ts, _)| *ts < cutoff) {
            self.price_history.pop_front();
        }
    }

    fn evict_liq(&mut self, now: i64) {
        let cutoff = now - self.window_ms;
        while self.liq_events.front().map_or(false, |ts| *ts < cutoff) {
            self.liq_events.pop_front();
        }
    }

    fn recompute(&mut self) {
        let oi_score = if self.oi_history.len() >= 2 {
            let oldest = self.oi_history.front().map_or(0.0, |(_, v)| *v);
            let newest = self.oi_history.back().map_or(0.0, |(_, v)| *v);
            if oldest > 0.0 {
                ((newest - oldest).abs() / oldest * 10.0).clamp(0.0, 1.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        let (price_score, direction) = if self.price_history.len() >= 2 {
            let oldest = self.price_history.front().map_or(0.0, |(_, v)| *v);
            let newest = self.price_history.back().map_or(0.0, |(_, v)| *v);
            if oldest > 0.0 {
                let rel = (newest - oldest) / oldest;
                let score = (rel.abs() * 100.0).clamp(0.0, 1.0);
                let dir = if rel > 0.0 { 1.0 } else if rel < 0.0 { -1.0 } else { 0.0 };
                (score, dir)
            } else {
                (0.0, 0.0)
            }
        } else {
            (0.0, 0.0)
        };

        let liq_score = (self.liq_events.len() as f64 / self.max_expected_liq).clamp(0.0, 1.0);

        let funding_score = (self.last_funding_abs * 1000.0).clamp(0.0, 1.0);

        self.last_prob = 0.3 * oi_score + 0.25 * liq_score + 0.25 * price_score + 0.2 * funding_score;
        self.last_direction = direction;
    }

    /// Named output: brace `prob`.
    pub fn prob(&self) -> f64 { self.last_prob }
    /// Named output: brace `dir`.
    pub fn dir(&self) -> f64 { self.last_direction }


    /// True when OI and price have at least 2 samples.
    pub fn indicator_is_ready(&self) -> bool {
        self.oi_history.len() >= 2 && self.price_history.len() >= 2
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.oi_history.clear();
        self.price_history.clear();
        self.liq_events.clear();
        self.last_funding_abs = 0.0;
        self.last_prob = 0.0;
        self.last_direction = 0.0;
    }
}

impl Default for CompoundSqueezeProbability {
    fn default() -> Self {
        Self::new(60_000, 10.0)
    }
}

/// Typed configuration for [`CompoundSqueezeProbability`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct CompoundSqueezeProbabilityConfig {
    /// Rolling time window in milliseconds.
    pub window_ms: Param<i64>,
    /// Expected liquidation events per window for normalization.
    pub max_expected_liq: Param<f64>,
}

impl Indicator for CompoundSqueezeProbability {
    const ID: IndicatorId = IndicatorId::CompoundSqueezeProbability;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[
        StreamKind::OpenInterest,
        StreamKind::MarkPrice,
        StreamKind::Liquidation,
        StreamKind::Funding,
    ];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::CompoundSqueezeProbabilityProb),
        Output::discrete(IndicatorOutputId::CompoundSqueezeProbabilityDir),
    ];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = CompoundSqueezeProbabilityConfig;
    type Runtime = CompoundSqueezeProbability;

    fn create(cfg: CompoundSqueezeProbabilityConfig) -> CompoundSqueezeProbability {
        CompoundSqueezeProbability::new(cfg.window_ms.resolved(), cfg.max_expected_liq.resolved())
    }
}

impl crate::contract::Config for CompoundSqueezeProbabilityConfig {
    fn defaults() -> Self {
        CompoundSqueezeProbabilityConfig {
            window_ms: Param::Solo(60_000),
            max_expected_liq: Param::Solo(10.0),
        }
    }
    fn machine_defaults() -> Self {
        // window_ms (i64): Class M raw-ms — canonical named durations (§3.8)
        // max_expected_liq (f64): ambiguous — normalization denominator representing expected
        //   liquidation event count per window (default 10.0). Not a sigma/multiplier/fraction.
        //   Swept as a discrete set of plausible event-count denominators: 1,2,5,10,20,50,100.
        let mut s = Self::machine_defaults_auto();
        s.window_ms = Param::many(vec![1000i64, 5000, 10000, 30000, 60000, 300000, 3600000]);
        s.max_expected_liq = Param::many(vec![1.0f64, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0]);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for CompoundSqueezeProbability {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::CompoundSqueezeProbabilityProb, "Squeeze Probability", Color::hex(0xFF5722), 1.5))
            .output(RenderOutput::line(IndicatorOutputId::CompoundSqueezeProbabilityDir, "Direction", Color::hex(0x4CAF50), 1.0))
            .bounds(0.0, 1.0)
            .precision(3)
            .build()
    }
}

impl OpenInterestConsumer for CompoundSqueezeProbability {
    fn update_oi(&mut self, oi: &OpenInterest) {
        self.evict_oi(oi.timestamp);
        self.oi_history.push_back((oi.timestamp, oi.open_interest));
        self.recompute();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl MarkPriceConsumer for CompoundSqueezeProbability {
    fn update_mark(&mut self, mp: &MarkPrice) {
        self.evict_price(mp.timestamp);
        self.price_history.push_back((mp.timestamp, mp.mark_price));
        self.recompute();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl LiquidationConsumer for CompoundSqueezeProbability {
    fn update_liquidation(&mut self, liq: &Liquidation) {
        self.evict_liq(liq.timestamp);
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

impl FundingRateConsumer for CompoundSqueezeProbability {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::TradeSide;

    fn make_oi(open_interest: f64, ts: i64) -> OpenInterest {
        OpenInterest { open_interest, open_interest_value: None, timestamp: ts, ..Default::default()}
    }

    fn make_mp(mark_price: f64, ts: i64) -> MarkPrice {
        MarkPrice { mark_price, index_price: None, funding_rate: None, timestamp: ts, ..Default::default()}
    }

    fn make_liq(ts: i64) -> Liquidation {
        Liquidation { symbol: String::new(), side: TradeSide::Buy, price: 30000.0, quantity: 0.1, timestamp: ts, value: None, ..Default::default()}
    }

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate { rate, next_funding_time: None, timestamp: 1000, ..Default::default()}
    }

    #[test]
    fn probability_in_range() {
        let mut ind = CompoundSqueezeProbability::new(60_000, 5.0);
        ind.update_oi(&make_oi(1000.0, 1000));
        ind.update_oi(&make_oi(900.0, 2000));
        ind.update_mark(&make_mp(30000.0, 1000));
        ind.update_mark(&make_mp(29000.0, 2000));
        ind.update_liquidation(&make_liq(1500));
        ind.update_funding(&make_fr(0.001));
        let prob = ind.prob();
        assert!(prob >= 0.0 && prob <= 1.0, "prob={prob}");
    }

    #[test]
    fn funding_raises_probability_vs_no_funding() {
        let mut ind_base = CompoundSqueezeProbability::new(60_000, 5.0);
        let mut ind_fund = CompoundSqueezeProbability::new(60_000, 5.0);
        for ind in [&mut ind_base, &mut ind_fund] {
            ind.update_oi(&make_oi(1000.0, 1000));
            ind.update_oi(&make_oi(900.0, 2000));
            ind.update_mark(&make_mp(30000.0, 1000));
            ind.update_mark(&make_mp(29000.0, 2000));
        }
        ind_fund.update_funding(&make_fr(0.001));
        let p_base = ind_base.prob();
        let p_fund = ind_fund.prob();
        assert!(p_fund >= p_base, "fund={p_fund} base={p_base}");
    }

    #[test]
    fn direction_positive_on_price_rise() {
        let mut ind = CompoundSqueezeProbability::new(60_000, 5.0);
        ind.update_oi(&make_oi(1000.0, 1000));
        ind.update_oi(&make_oi(1000.0, 2000));
        ind.update_mark(&make_mp(29000.0, 1000));
        ind.update_mark(&make_mp(30000.0, 2000)); // rising
        let dir = ind.dir();
        assert_eq!(dir, 1.0);
    }

    #[test]
    fn factory_feeds_compound_squeeze_probability() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = CompoundSqueezeProbabilityConfig { window_ms: Param::Solo(60_000), max_expected_liq: Param::Solo(5.0) };
        let mut f = IndicatorOrder::CompoundSqueezeProbability(cfg).build_solo().unwrap();
        let oi1 = make_oi(1000.0, 1000);
        let oi2 = make_oi(900.0, 2000);
        let mp1 = make_mp(30000.0, 1000);
        let mp2 = make_mp(29000.0, 2000);
        let liq = make_liq(1500);
        let fr = make_fr(0.001);
        f.feed(0, MarketSample::OpenInterest(&oi1));
        f.feed(0, MarketSample::OpenInterest(&oi2));
        f.feed(0, MarketSample::MarkPrice(&mp1));
        f.feed(0, MarketSample::MarkPrice(&mp2));
        f.feed(0, MarketSample::Liquidation(&liq));
        f.feed(0, MarketSample::Funding(&fr));
        let prob = f.primary();
        assert!(prob >= 0.0 && prob <= 1.0, "prob={prob}");
    }
}
