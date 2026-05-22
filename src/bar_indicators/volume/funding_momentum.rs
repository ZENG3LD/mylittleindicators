//! Funding Momentum — EMA of funding rate with slope direction.
//!
//! Smooths funding rate updates via EMA and tracks slope direction.
//! Positive slope = funding trending up (longs paying more = bearish signal).
//! Negative slope = funding trending down.
//!
//! Output: `Double(ema_value, slope)` where slope is ema[now] - ema[prev].

use crate::bar_indicators::funding_rate_consumer::FundingRateConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::types::FundingRate;

/// EMA-smoothed funding rate with momentum slope.
#[derive(Clone)]
pub struct FundingMomentum {
    /// EMA period.
    period: usize,
    /// EMA smoothing factor (2 / (period + 1)).
    alpha: f64,
    /// Current EMA value.
    ema: f64,
    /// Previous EMA value for slope calculation.
    prev_ema: f64,
    /// Number of updates seen.
    count: usize,
}

impl FundingMomentum {
    /// Create with given EMA period.
    pub fn new(period: usize) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            alpha: 2.0 / (p as f64 + 1.0),
            ema: 0.0,
            prev_ema: 0.0,
            count: 0,
        }
    }
}

impl FundingRateConsumer for FundingMomentum {
    fn update_funding(&mut self, fr: &FundingRate) -> IndicatorValue {
        self.prev_ema = self.ema;
        if self.count == 0 {
            self.ema = fr.rate;
        } else {
            self.ema = self.ema + self.alpha * (fr.rate - self.ema);
        }
        self.count += 1;
        let slope = self.ema - self.prev_ema;
        IndicatorValue::Double(self.ema, slope)
    }

    fn value(&self) -> IndicatorValue {
        let slope = self.ema - self.prev_ema;
        IndicatorValue::Double(self.ema, slope)
    }

    fn reset(&mut self) {
        self.ema = 0.0;
        self.prev_ema = 0.0;
        self.count = 0;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate { rate, next_funding_time: None, timestamp: 1000 }
    }

    #[test]
    fn not_ready_initially() {
        let fm = FundingMomentum::new(5);
        assert!(!fm.is_ready());
    }

    #[test]
    fn ema_converges_to_constant_rate() {
        let mut fm = FundingMomentum::new(3);
        for _ in 0..30 {
            fm.update_funding(&make_fr(0.0001));
        }
        assert!(fm.is_ready());
        if let IndicatorValue::Double(ema, _) = fm.value() {
            assert!((ema - 0.0001).abs() < 1e-10);
        }
    }

    #[test]
    fn slope_positive_on_rising_rates() {
        let mut fm = FundingMomentum::new(2);
        for i in 0..20 {
            fm.update_funding(&make_fr(i as f64 * 0.0001));
        }
        if let IndicatorValue::Double(_, slope) = fm.value() {
            assert!(slope > 0.0, "slope should be positive on rising rates");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut fm = FundingMomentum::new(3);
        for _ in 0..10 {
            fm.update_funding(&make_fr(0.0001));
        }
        fm.reset();
        assert!(!fm.is_ready());
        if let IndicatorValue::Double(ema, slope) = fm.value() {
            assert_eq!(ema, 0.0);
            assert_eq!(slope, 0.0);
        }
    }
}
