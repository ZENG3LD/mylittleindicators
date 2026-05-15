//! RiskOffDetector — risk-off signal when 2+ stress components exceed threshold.
//!
//! Quad consumer: `VolatilityIndexConsumer` + `LiquidationConsumer` +
//! `FundingRateConsumer` + `InsuranceFundConsumer`.
//!
//! Logic: if 2+ components exceed threshold → Signal = +1 (risk-off), else 0.
//! Components:
//! 1. vol_idx > threshold
//! 2. liq_count_in_window >= threshold_count
//! 3. |funding_rate| × 100 > threshold
//! 4. fund_depletion active (recent slope < 0)
//!
//! Output: `Signal(i8)` — +1 risk-off, 0 normal.

use std::collections::VecDeque;

use crate::bar_indicators::funding_rate_consumer::FundingRateConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::insurance_fund_consumer::InsuranceFundConsumer;
use crate::bar_indicators::liquidation_consumer::LiquidationConsumer;
use crate::bar_indicators::volatility_index_consumer::VolatilityIndexConsumer;
use crate::core::types::{FundingRate, InsuranceFund, Liquidation, VolatilityIndex};

/// Risk-off detector composite.
///
/// Implements `VolatilityIndexConsumer`, `LiquidationConsumer`,
/// `FundingRateConsumer`, and `InsuranceFundConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Clone)]
pub struct RiskOffDetector {
    window_ms: i64,
    /// Threshold for vol index to be considered elevated.
    vol_threshold: f64,
    /// Minimum liq events in window to count as spike.
    liq_threshold: usize,
    /// |funding| × 100 threshold.
    funding_threshold: f64,

    current_vol: f64,
    liq_events: VecDeque<i64>,
    last_funding_abs: f64,
    fund_history: VecDeque<(i64, f64)>,

    last_signal: i8,
}

impl RiskOffDetector {
    /// Create a new indicator.
    ///
    /// - `window_ms`          — rolling window for liq events (default 60_000)
    /// - `vol_threshold`      — vol index level considered elevated (default 0.5)
    /// - `liq_threshold`      — min liquidation count for spike (default 3)
    /// - `funding_threshold`  — |funding| × 100 threshold (default 0.05)
    pub fn new(
        window_ms: i64,
        vol_threshold: f64,
        liq_threshold: usize,
        funding_threshold: f64,
    ) -> Self {
        Self {
            window_ms,
            vol_threshold,
            liq_threshold,
            funding_threshold,
            current_vol: 0.0,
            liq_events: VecDeque::new(),
            last_funding_abs: 0.0,
            fund_history: VecDeque::with_capacity(10),
            last_signal: 0,
        }
    }

    fn fund_depleting(&self) -> bool {
        if self.fund_history.len() < 2 {
            return false;
        }
        let first = self.fund_history.front().copied().unwrap_or((0, 0.0));
        let last = self.fund_history.back().copied().unwrap_or((0, 0.0));
        last.1 < first.1
    }

    fn recompute(&mut self, now: i64) {
        // Evict stale liq events
        let cutoff = now - self.window_ms;
        while self.liq_events.front().map_or(false, |ts| *ts < cutoff) {
            self.liq_events.pop_front();
        }

        let components_active: usize = [
            self.current_vol > self.vol_threshold,
            self.liq_events.len() >= self.liq_threshold,
            self.last_funding_abs * 100.0 > self.funding_threshold,
            self.fund_depleting(),
        ]
        .iter()
        .filter(|&&b| b)
        .count();

        self.last_signal = if components_active >= 2 { 1 } else { 0 };
    }

    /// Passthrough for bar pipeline — returns current value.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.last_signal)
    }

    /// True when at least one vol update has arrived.
    pub fn indicator_is_ready(&self) -> bool {
        self.current_vol > 0.0 || !self.liq_events.is_empty()
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.current_vol = 0.0;
        self.liq_events.clear();
        self.last_funding_abs = 0.0;
        self.fund_history.clear();
        self.last_signal = 0;
    }
}

impl Default for RiskOffDetector {
    fn default() -> Self {
        Self::new(60_000, 0.5, 3, 0.05)
    }
}

impl VolatilityIndexConsumer for RiskOffDetector {
    fn update_volatility_index(&mut self, vi: &VolatilityIndex) -> IndicatorValue {
        self.current_vol = vi.value;
        self.recompute(vi.timestamp);
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

impl LiquidationConsumer for RiskOffDetector {
    fn update_liquidation(&mut self, liq: &Liquidation) -> IndicatorValue {
        self.liq_events.push_back(liq.timestamp);
        self.recompute(liq.timestamp);
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

impl FundingRateConsumer for RiskOffDetector {
    fn update_funding(&mut self, fr: &FundingRate) -> IndicatorValue {
        self.last_funding_abs = fr.rate.abs();
        self.recompute(fr.timestamp);
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

impl InsuranceFundConsumer for RiskOffDetector {
    fn update_insurance_fund(&mut self, ins: &InsuranceFund) -> IndicatorValue {
        if self.fund_history.len() >= 10 {
            self.fund_history.pop_front();
        }
        self.fund_history.push_back((ins.timestamp, ins.balance));
        self.recompute(ins.timestamp);
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

    fn make_vi(value: f64, ts: i64) -> VolatilityIndex {
        VolatilityIndex { value, timestamp: ts }
    }

    fn make_liq(ts: i64) -> Liquidation {
        Liquidation { symbol: String::new(), side: TradeSide::Buy, price: 30000.0, quantity: 0.1, timestamp: ts, value: None }
    }

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate { symbol: "BTCUSDT".to_string(), rate, next_funding_time: None, timestamp: 1000 }
    }

    #[test]
    fn two_components_trigger_risk_off() {
        // vol high + funding high → 2 components
        let mut ind = RiskOffDetector::new(60_000, 0.5, 3, 0.05);
        ind.update_volatility_index(&make_vi(1.0, 1000)); // vol above 0.5
        ind.update_funding(&make_fr(0.001)); // |0.001| × 100 = 0.1 > 0.05
        if let IndicatorValue::Signal(s) = ind.indicator_value() {
            assert_eq!(s, 1, "expected risk-off");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn single_component_no_risk_off() {
        let mut ind = RiskOffDetector::new(60_000, 0.5, 3, 0.05);
        ind.update_volatility_index(&make_vi(1.0, 1000)); // only 1 component
        if let IndicatorValue::Signal(s) = ind.indicator_value() {
            assert_eq!(s, 0, "only 1 component, should be 0");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn liq_spike_plus_vol_triggers() {
        let mut ind = RiskOffDetector::new(60_000, 0.5, 2, 0.05);
        ind.update_volatility_index(&make_vi(1.0, 1000));
        for i in 0..2i64 {
            ind.update_liquidation(&make_liq(1000 + i * 100));
        }
        if let IndicatorValue::Signal(s) = ind.indicator_value() {
            assert_eq!(s, 1, "vol+liq should trigger risk-off");
        } else {
            panic!("expected Signal");
        }
    }

    #[test]
    fn reset_clears_signal() {
        let mut ind = RiskOffDetector::default();
        ind.update_volatility_index(&make_vi(1.0, 1000));
        ind.update_funding(&make_fr(0.001));
        ind.indicator_reset();
        if let IndicatorValue::Signal(s) = ind.indicator_value() {
            assert_eq!(s, 0);
        } else {
            panic!("expected Signal");
        }
    }
}
