//! SqueezeProbability — composite squeeze probability from OI, price, and liquidations.
//!
//! Triple consumer: `OpenInterestConsumer` + `MarkPriceConsumer` + `LiquidationConsumer`.
//!
//! Score formula (empirical, not calibrated):
//! - `oi_score`    = clamp(|dOI/oi| × 10, 0, 1)
//! - `price_score` = clamp(|dPrice/price| × 100, 0, 1)
//! - `liq_score`   = clamp(liq_count / max_expected_liq, 0, 1)
//! - `prob`        = 0.4 × oi_score + 0.3 × price_score + 0.3 × liq_score
//! - `direction`   = sign of price move (+1 up, -1 down)
//!
//! Output: `Double(probability, direction_as_f64)`.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::liquidation_consumer::LiquidationConsumer;
use crate::engine::streams::mark_price_consumer::MarkPriceConsumer;
use crate::engine::streams::open_interest_consumer::OpenInterestConsumer;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::core::types::{Liquidation, MarkPrice, OpenInterest};
use crate::engine::stream_kind::StreamKind;

/// Composite squeeze probability indicator.
///
/// Implements `OpenInterestConsumer`, `MarkPriceConsumer`, and `LiquidationConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
///
/// **Note**: score formula is empirical and not statistically calibrated.
#[derive(Debug, Clone)]
pub struct SqueezeProbability {
    window_ms: i64,
    max_expected_liq: f64,
    oi_history: VecDeque<(i64, f64)>,
    price_history: VecDeque<(i64, f64)>,
    liq_events: VecDeque<i64>,
    last_prob: f64,
    last_direction: f64,
}

impl SqueezeProbability {
    /// Create a new indicator.
    ///
    /// - `window_ms` — rolling time window in milliseconds (default 60_000)
    /// - `max_expected_liq` — expected liquidations per window for normalization (default 10.0)
    pub fn new(window_ms: i64, max_expected_liq: f64) -> Self {
        Self {
            window_ms,
            max_expected_liq: max_expected_liq.max(1.0),
            oi_history: VecDeque::new(),
            price_history: VecDeque::new(),
            liq_events: VecDeque::new(),
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
        // OI score: relative change over window
        let oi_score = if self.oi_history.len() >= 2 {
            let oldest_oi = self.oi_history.front().map_or(0.0, |(_, v)| *v);
            let newest_oi = self.oi_history.back().map_or(0.0, |(_, v)| *v);
            if oldest_oi > 0.0 {
                ((newest_oi - oldest_oi).abs() / oldest_oi * 10.0).clamp(0.0, 1.0)
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Price score + direction
        let (price_score, direction) = if self.price_history.len() >= 2 {
            let oldest_p = self.price_history.front().map_or(0.0, |(_, v)| *v);
            let newest_p = self.price_history.back().map_or(0.0, |(_, v)| *v);
            if oldest_p > 0.0 {
                let rel = (newest_p - oldest_p) / oldest_p;
                let score = (rel.abs() * 100.0).clamp(0.0, 1.0);
                let dir = if rel > 0.0 { 1.0 } else if rel < 0.0 { -1.0 } else { 0.0 };
                (score, dir)
            } else {
                (0.0, 0.0)
            }
        } else {
            (0.0, 0.0)
        };

        // Liq score
        let liq_score = ((self.liq_events.len() as f64) / self.max_expected_liq).clamp(0.0, 1.0);

        self.last_prob = 0.4 * oi_score + 0.3 * price_score + 0.3 * liq_score;
        self.last_direction = direction;
    }

    /// Named output: brace `prob`.
    pub fn prob(&self) -> f64 { self.last_prob }
    /// Named output: brace `dir`.
    pub fn dir(&self) -> f64 { self.last_direction }


    /// True when OI and price streams have at least 2 data points each.
    pub fn indicator_is_ready(&self) -> bool {
        self.oi_history.len() >= 2 && self.price_history.len() >= 2
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.oi_history.clear();
        self.price_history.clear();
        self.liq_events.clear();
        self.last_prob = 0.0;
        self.last_direction = 0.0;
    }
}

impl Default for SqueezeProbability {
    fn default() -> Self {
        Self::new(60_000, 10.0)
    }
}

impl OpenInterestConsumer for SqueezeProbability {
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

impl MarkPriceConsumer for SqueezeProbability {
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

impl LiquidationConsumer for SqueezeProbability {
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

/// Typed configuration for [`SqueezeProbability`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct SqueezeProbabilityConfig {
    /// Rolling time window in milliseconds.
    pub window_ms: Param<i64>,
    /// Expected liquidation events per window for normalisation.
    pub max_expected_liq: Param<f64>,
}

impl Indicator for SqueezeProbability {
    const ID: IndicatorId = IndicatorId::SqueezeProbability;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[
        StreamKind::OpenInterest,
        StreamKind::MarkPrice,
        StreamKind::Liquidation,
    ];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::SqueezeProbabilityProb),
        Output::discrete(IndicatorOutputId::SqueezeProbabilityDir),
    ];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = SqueezeProbabilityConfig;
    type Runtime = SqueezeProbability;

    fn create(cfg: SqueezeProbabilityConfig) -> SqueezeProbability {
        SqueezeProbability::new(cfg.window_ms.resolved(), cfg.max_expected_liq.resolved())
    }
}

impl crate::contract::Config for SqueezeProbabilityConfig {
    fn defaults() -> Self {
        SqueezeProbabilityConfig {
            window_ms: Param::Solo(60_000),
            max_expected_liq: Param::Solo(10.0),
        }
    }
    fn machine_defaults() -> Self {
        // window_ms (i64): Class M raw-ms — canonical named durations (§3.8)
        // max_expected_liq (f64): ambiguous — normalization denominator for expected liq events per
        //   window (default 10.0). Swept as a discrete set of plausible event-count denominators.
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


impl Render for SqueezeProbability {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::SqueezeProbabilityProb, "Squeeze Probability", Color::hex(0xE91E63), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::SqueezeProbabilityDir, "Direction", Color::hex(0xFF9800), 1.0))
            .bounds(0.0, 1.0)
            .zero_baseline()
            .precision(4)
            .build()
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

    #[test]
    fn probability_in_range() {
        let mut ind = SqueezeProbability::new(60_000, 5.0);
        ind.update_oi(&make_oi(1000.0, 1000));
        ind.update_oi(&make_oi(900.0, 2000)); // dOI = -100
        ind.update_mark(&make_mp(30000.0, 1000));
        ind.update_mark(&make_mp(29000.0, 2000)); // -3.3%
        ind.update_liquidation(&make_liq(1500));
        ind.update_liquidation(&make_liq(1800));
        let prob = ind.prob();
        assert!(prob >= 0.0 && prob <= 1.0, "prob={prob}");
    }

    #[test]
    fn direction_negative_on_price_drop() {
        let mut ind = SqueezeProbability::new(60_000, 5.0);
        ind.update_oi(&make_oi(1000.0, 1000));
        ind.update_oi(&make_oi(900.0, 2000));
        ind.update_mark(&make_mp(30000.0, 1000));
        ind.update_mark(&make_mp(29000.0, 2000));
        assert_eq!(ind.dir(), -1.0);
    }

    #[test]
    fn not_ready_before_two_oi_and_price_updates() {
        let mut ind = SqueezeProbability::new(60_000, 5.0);
        ind.update_oi(&make_oi(1000.0, 1000));
        ind.update_mark(&make_mp(30000.0, 1000));
        assert!(!ind.indicator_is_ready());
        ind.update_oi(&make_oi(900.0, 2000));
        ind.update_mark(&make_mp(29000.0, 2000));
        assert!(ind.indicator_is_ready());
    }

    #[test]
    fn liq_spike_increases_probability() {
        let mut ind_no_liq = SqueezeProbability::new(60_000, 5.0);
        let mut ind_with_liq = SqueezeProbability::new(60_000, 5.0);

        for (ind, ts_offset) in [(&mut ind_no_liq, 0i64), (&mut ind_with_liq, 0)] {
            ind.update_oi(&make_oi(1000.0, 1000 + ts_offset));
            ind.update_oi(&make_oi(1000.0, 2000 + ts_offset)); // no OI change
            ind.update_mark(&make_mp(30000.0, 1000 + ts_offset));
            ind.update_mark(&make_mp(30000.0, 2000 + ts_offset)); // no price move
        }
        // Add liquidations only to ind_with_liq
        for i in 0..5 {
            ind_with_liq.update_liquidation(&make_liq(1000 + i * 200));
        }

        let prob_no_liq = ind_no_liq.prob();
        let prob_with_liq = ind_with_liq.prob();
        assert!(prob_with_liq > prob_no_liq, "liq={prob_with_liq} no_liq={prob_no_liq}");
    }
}
