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

use crate::bar_indicators::funding_rate_consumer::FundingRateConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::liquidation_consumer::LiquidationConsumer;
use crate::bar_indicators::mark_price_consumer::MarkPriceConsumer;
use crate::bar_indicators::open_interest_consumer::OpenInterestConsumer;
use crate::core::types::{FundingRate, Liquidation, MarkPrice, OpenInterest};

/// Compound squeeze probability with 4 factors.
///
/// Implements `OpenInterestConsumer`, `LiquidationConsumer`,
/// `MarkPriceConsumer`, and `FundingRateConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Clone)]
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

    /// Passthrough for bar pipeline — returns current value.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.last_prob, self.last_direction)
    }

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

impl OpenInterestConsumer for CompoundSqueezeProbability {
    fn update_oi(&mut self, oi: &OpenInterest) -> IndicatorValue {
        self.evict_oi(oi.timestamp);
        self.oi_history.push_back((oi.timestamp, oi.open_interest));
        self.recompute();
        self.indicator_value()
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
    }

    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl MarkPriceConsumer for CompoundSqueezeProbability {
    fn update_mark(&mut self, mp: &MarkPrice) -> IndicatorValue {
        self.evict_price(mp.timestamp);
        self.price_history.push_back((mp.timestamp, mp.mark_price));
        self.recompute();
        self.indicator_value()
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
    }

    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl LiquidationConsumer for CompoundSqueezeProbability {
    fn update_liquidation(&mut self, liq: &Liquidation) -> IndicatorValue {
        self.evict_liq(liq.timestamp);
        self.liq_events.push_back(liq.timestamp);
        self.recompute();
        self.indicator_value()
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
    }

    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl FundingRateConsumer for CompoundSqueezeProbability {
    fn update_funding(&mut self, fr: &FundingRate) -> IndicatorValue {
        self.last_funding_abs = fr.rate.abs();
        self.recompute();
        self.indicator_value()
    }

    fn value(&self) -> IndicatorValue {
        self.indicator_value()
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
        OpenInterest { open_interest, open_interest_value: None, timestamp: ts }
    }

    fn make_mp(mark_price: f64, ts: i64) -> MarkPrice {
        MarkPrice { mark_price, index_price: None, funding_rate: None, timestamp: ts }
    }

    fn make_liq(ts: i64) -> Liquidation {
        Liquidation { symbol: String::new(), side: TradeSide::Buy, price: 30000.0, quantity: 0.1, timestamp: ts, value: None }
    }

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate { rate, next_funding_time: None, timestamp: 1000 }
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
        if let IndicatorValue::Double(prob, _) = ind.indicator_value() {
            assert!(prob >= 0.0 && prob <= 1.0, "prob={prob}");
        } else {
            panic!("expected Double");
        }
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
        let p_base = match ind_base.indicator_value() { IndicatorValue::Double(p, _) => p, _ => panic!() };
        let p_fund = match ind_fund.indicator_value() { IndicatorValue::Double(p, _) => p, _ => panic!() };
        assert!(p_fund >= p_base, "fund={p_fund} base={p_base}");
    }

    #[test]
    fn direction_positive_on_price_rise() {
        let mut ind = CompoundSqueezeProbability::new(60_000, 5.0);
        ind.update_oi(&make_oi(1000.0, 1000));
        ind.update_oi(&make_oi(1000.0, 2000));
        ind.update_mark(&make_mp(29000.0, 1000));
        ind.update_mark(&make_mp(30000.0, 2000)); // rising
        if let IndicatorValue::Double(_, dir) = ind.indicator_value() {
            assert_eq!(dir, 1.0);
        } else {
            panic!("expected Double");
        }
    }
}
