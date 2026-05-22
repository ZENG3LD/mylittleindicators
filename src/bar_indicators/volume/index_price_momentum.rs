//! IndexPriceMomentum — EMA-based momentum (slope) of the index/spot price
//! extracted from mark price feed.
//!
//! Uses the `index_price` field from `MarkPrice` when available; falls back to
//! `mark_price` if `index_price` is `None`.
//!
//! Output: `Double(ema, slope)`
//!   - ema:   exponential moving average of the index price
//!   - slope: ema[now] - ema[prev] (positive = trending up)

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::mark_price_consumer::MarkPriceConsumer;
use crate::core::types::MarkPrice;

/// EMA-smoothed momentum of the index/spot price from the mark price feed.
#[derive(Debug, Clone)]
pub struct IndexPriceMomentum {
    period: usize,
    alpha: f64,
    ema: f64,
    prev_ema: f64,
    count: usize,
}

impl IndexPriceMomentum {
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

impl MarkPriceConsumer for IndexPriceMomentum {
    fn update_mark(&mut self, mp: &MarkPrice) -> IndicatorValue {
        let price = mp.index_price.unwrap_or(mp.mark_price);
        self.prev_ema = self.ema;
        if self.count == 0 {
            self.ema = price;
        } else {
            self.ema = self.ema + self.alpha * (price - self.ema);
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

    fn mp_with_index(mark: f64, index: f64) -> MarkPrice {
        MarkPrice {
            mark_price: mark,
            index_price: Some(index),
            funding_rate: None,
            timestamp: 0,
        }
    }

    fn mp_no_index(mark: f64) -> MarkPrice {
        MarkPrice {
            mark_price: mark,
            index_price: None,
            funding_rate: None,
            timestamp: 0,
        }
    }

    #[test]
    fn not_ready_before_period() {
        let ind = IndexPriceMomentum::new(5);
        assert!(!ind.is_ready());
    }

    #[test]
    fn ready_after_period_updates() {
        let mut ind = IndexPriceMomentum::new(3);
        for i in 0..3 {
            ind.update_mark(&mp_no_index(50_000.0 + i as f64));
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn uses_index_price_when_available() {
        let mut ind = IndexPriceMomentum::new(1);
        // index=50_000, mark=51_000 → EMA should track index
        let v = ind.update_mark(&mp_with_index(51_000.0, 50_000.0));
        match v {
            IndicatorValue::Double(ema, _) => {
                assert!((ema - 50_000.0).abs() < 1e-6, "should use index price");
            }
            _ => panic!("expected Double"),
        }
    }

    #[test]
    fn falls_back_to_mark_price() {
        let mut ind = IndexPriceMomentum::new(1);
        let v = ind.update_mark(&mp_no_index(50_000.0));
        match v {
            IndicatorValue::Double(ema, _) => {
                assert!((ema - 50_000.0).abs() < 1e-6, "fallback to mark_price");
            }
            _ => panic!("expected Double"),
        }
    }

    #[test]
    fn slope_positive_on_rising_index() {
        let mut ind = IndexPriceMomentum::new(2);
        for i in 0..20 {
            ind.update_mark(&mp_with_index(0.0, 50_000.0 + i as f64 * 10.0));
        }
        match ind.value() {
            IndicatorValue::Double(_, slope) => {
                assert!(slope > 0.0, "slope should be positive on rising prices");
            }
            _ => panic!("expected Double"),
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = IndexPriceMomentum::new(3);
        for _ in 0..5 {
            ind.update_mark(&mp_no_index(50_000.0));
        }
        assert!(ind.is_ready());
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::Double(0.0, 0.0));
    }
}
