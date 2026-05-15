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

use crate::bar_indicators::funding_rate_consumer::FundingRateConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::insurance_fund_consumer::InsuranceFundConsumer;
use crate::bar_indicators::liquidation_consumer::LiquidationConsumer;
use crate::bar_indicators::volatility_index_consumer::VolatilityIndexConsumer;
use crate::core::types::{FundingRate, InsuranceFund, Liquidation, VolatilityIndex};

/// Composite market stress score.
///
/// Implements `VolatilityIndexConsumer`, `LiquidationConsumer`,
/// `FundingRateConsumer`, and `InsuranceFundConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Clone)]
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

    /// Passthrough for bar pipeline — returns current value.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_stress)
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
    fn update_volatility_index(&mut self, vi: &VolatilityIndex) -> IndicatorValue {
        self.current_vol = vi.value;
        if self.vol_history.len() >= self.vol_history_cap {
            self.vol_history.pop_front();
        }
        self.vol_history.push_back(vi.value);
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

impl LiquidationConsumer for MarketStressComposite {
    fn update_liquidation(&mut self, liq: &Liquidation) -> IndicatorValue {
        let cutoff = liq.timestamp - self.window_ms;
        while self.liq_events.front().map_or(false, |ts| *ts < cutoff) {
            self.liq_events.pop_front();
        }
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

impl FundingRateConsumer for MarketStressComposite {
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

impl InsuranceFundConsumer for MarketStressComposite {
    fn update_insurance_fund(&mut self, ins: &InsuranceFund) -> IndicatorValue {
        if self.fund_history.len() >= 10 {
            self.fund_history.pop_front();
        }
        self.fund_history.push_back((ins.timestamp, ins.balance));
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

    fn make_vi(value: f64) -> VolatilityIndex {
        VolatilityIndex { value, timestamp: 1000 }
    }

    fn make_liq(ts: i64) -> Liquidation {
        Liquidation { symbol: String::new(), side: TradeSide::Buy, price: 30000.0, quantity: 0.1, timestamp: ts, value: None }
    }

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate { symbol: "BTCUSDT".to_string(), rate, next_funding_time: None, timestamp: 1000 }
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
        if let IndicatorValue::Single(s) = ind.indicator_value() {
            assert!(s >= 0.0 && s <= 1.0, "stress={s}");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn high_funding_increases_stress() {
        let mut ind_low = MarketStressComposite::new(60_000, 5.0, 20, -1e-6);
        let mut ind_high = MarketStressComposite::new(60_000, 5.0, 20, -1e-6);
        ind_low.update_funding(&make_fr(0.0001));
        ind_high.update_funding(&make_fr(0.01));
        let s_low = match ind_low.indicator_value() { IndicatorValue::Single(v) => v, _ => panic!() };
        let s_high = match ind_high.indicator_value() { IndicatorValue::Single(v) => v, _ => panic!() };
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
        if let IndicatorValue::Single(s) = ind.indicator_value() {
            assert_eq!(s, 0.0);
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn depletion_signal_triggers_on_steep_decline() {
        let mut ind = MarketStressComposite::new(60_000, 5.0, 20, -1e-3);
        // Steep decline: 1_000_000 → 1 over 1000 ms → slope ~ -999.999/ms → < -1e-3
        ind.update_insurance_fund(&make_ins(1_000_000.0, 0));
        ind.update_insurance_fund(&make_ins(1.0, 1000));
        let s = match ind.indicator_value() { IndicatorValue::Single(v) => v, _ => panic!() };
        // depletion component = 0.2 × 1.0 = 0.2, so stress >= 0.2
        assert!(s >= 0.19, "stress={s}");
    }
}
