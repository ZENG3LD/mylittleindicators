//! IvHvSpread — implied volatility minus historical volatility (volatility risk premium).
//!
//! Dual consumer: `HistoricalVolatilityConsumer` + `VolatilityIndexConsumer`.
//!
//! Logic:
//! - IV  = `VolatilityIndex.value`
//! - HV  = `HistoricalVolatility.volatility`
//! - VRP = IV - HV
//!
//! Output: `Triple(iv, hv, spread)`.

use crate::bar_indicators::historical_volatility_consumer::HistoricalVolatilityConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::volatility_index_consumer::VolatilityIndexConsumer;
use crate::core::types::HistoricalVolatility;
use crate::core::types::VolatilityIndex;

/// Volatility risk premium: spread between implied and historical volatility.
///
/// Implements both `HistoricalVolatilityConsumer` and `VolatilityIndexConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Clone)]
pub struct IvHvSpread {
    last_iv: f64,
    last_hv: f64,
}

impl IvHvSpread {
    /// Create a new indicator.
    pub fn new() -> Self {
        Self { last_iv: 0.0, last_hv: 0.0 }
    }

    fn spread(&self) -> f64 {
        self.last_iv - self.last_hv
    }

    /// Passthrough for bar pipeline — returns current value.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, _v: f64) -> IndicatorValue {
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.last_iv, self.last_hv, self.spread())
    }

    /// True when both streams have delivered at least one update.
    pub fn indicator_is_ready(&self) -> bool {
        self.last_iv > 0.0 && self.last_hv > 0.0
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.last_iv = 0.0;
        self.last_hv = 0.0;
    }
}

impl Default for IvHvSpread {
    fn default() -> Self {
        Self::new()
    }
}

impl HistoricalVolatilityConsumer for IvHvSpread {
    fn update_historical_volatility(&mut self, hv: &HistoricalVolatility) -> IndicatorValue {
        self.last_hv = hv.volatility;
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

impl VolatilityIndexConsumer for IvHvSpread {
    fn update_volatility_index(&mut self, vi: &VolatilityIndex) -> IndicatorValue {
        self.last_iv = vi.value;
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

    fn make_hv(volatility: f64) -> HistoricalVolatility {
        HistoricalVolatility { volatility, timestamp: 1000 }
    }

    fn make_vi(value: f64) -> VolatilityIndex {
        VolatilityIndex { value, timestamp: 1000 }
    }

    #[test]
    fn spread_is_iv_minus_hv() {
        let mut ind = IvHvSpread::new();
        ind.update_volatility_index(&make_vi(0.30));
        ind.update_historical_volatility(&make_hv(0.20));
        if let IndicatorValue::Triple(iv, hv, spread) = ind.indicator_value() {
            assert!((iv - 0.30).abs() < 1e-9);
            assert!((hv - 0.20).abs() < 1e-9);
            assert!((spread - 0.10).abs() < 1e-9);
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn negative_spread_when_hv_exceeds_iv() {
        let mut ind = IvHvSpread::new();
        ind.update_volatility_index(&make_vi(0.20));
        ind.update_historical_volatility(&make_hv(0.35));
        if let IndicatorValue::Triple(_, _, spread) = ind.indicator_value() {
            assert!(spread < 0.0);
        } else {
            panic!("expected Triple");
        }
    }

    #[test]
    fn not_ready_until_both_streams_updated() {
        let mut ind = IvHvSpread::new();
        assert!(!ind.indicator_is_ready());
        ind.update_volatility_index(&make_vi(0.30));
        assert!(!ind.indicator_is_ready());
        ind.update_historical_volatility(&make_hv(0.20));
        assert!(ind.indicator_is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = IvHvSpread::new();
        ind.update_volatility_index(&make_vi(0.30));
        ind.update_historical_volatility(&make_hv(0.20));
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        if let IndicatorValue::Triple(iv, hv, _) = ind.indicator_value() {
            assert_eq!(iv, 0.0);
            assert_eq!(hv, 0.0);
        }
    }
}
