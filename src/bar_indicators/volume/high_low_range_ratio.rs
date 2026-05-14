//! HighLowRangeRatio — intraday volatility proxy from 24-hour high/low range.
//!
//! Computes (high_24h - low_24h) / last_price.
//! A dimensionless ratio: 0 = no range, higher = wider daily swing.
//!
//! Output: `Single(ratio)`.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ticker_consumer::TickerConsumer;
use crate::core::types::Ticker;

/// (high_24h - low_24h) / last_price as a volatility proxy.
#[derive(Clone)]
pub struct HighLowRangeRatio {
    last_ratio: f64,
    ready: bool,
}

impl HighLowRangeRatio {
    /// Create a new instance. No parameters needed — ratio is instantaneous.
    pub fn new() -> Self {
        Self { last_ratio: 0.0, ready: false }
    }
}

impl Default for HighLowRangeRatio {
    fn default() -> Self {
        Self::new()
    }
}

impl TickerConsumer for HighLowRangeRatio {
    fn update_ticker(&mut self, ticker: &Ticker) -> IndicatorValue {
        let ratio = match (ticker.high_24h, ticker.low_24h) {
            (Some(h), Some(l)) if ticker.last_price > 0.0 => {
                (h - l) / ticker.last_price
            }
            _ => 0.0,
        };
        self.last_ratio = ratio;
        self.ready = true;
        IndicatorValue::Single(ratio)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_ratio)
    }

    fn reset(&mut self) {
        self.last_ratio = 0.0;
        self.ready = false;
    }

    fn is_ready(&self) -> bool {
        self.ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticker(last: f64, high: Option<f64>, low: Option<f64>) -> Ticker {
        Ticker {
            symbol: "BTCUSDT".to_string(),
            last_price: last,
            bid_price: None,
            ask_price: None,
            high_24h: high,
            low_24h: low,
            volume_24h: None,
            quote_volume_24h: None,
            price_change_24h: None,
            price_change_percent_24h: None,
            timestamp: 1000,
        }
    }

    #[test]
    fn not_ready_initially() {
        let ind = HighLowRangeRatio::new();
        assert!(!ind.is_ready());
    }

    #[test]
    fn correct_ratio() {
        let mut ind = HighLowRangeRatio::new();
        // range = 2000, last = 50000 → ratio = 0.04
        ind.update_ticker(&ticker(50000.0, Some(51000.0), Some(49000.0)));
        assert!(ind.is_ready());
        if let IndicatorValue::Single(r) = ind.value() {
            assert!((r - 0.04).abs() < 1e-12, "expected 0.04, got {}", r);
        }
    }

    #[test]
    fn missing_high_low_returns_zero() {
        let mut ind = HighLowRangeRatio::new();
        ind.update_ticker(&ticker(50000.0, None, None));
        if let IndicatorValue::Single(r) = ind.value() {
            assert_eq!(r, 0.0);
        }
    }

    #[test]
    fn zero_last_price_returns_zero() {
        let mut ind = HighLowRangeRatio::new();
        ind.update_ticker(&ticker(0.0, Some(100.0), Some(90.0)));
        if let IndicatorValue::Single(r) = ind.value() {
            assert_eq!(r, 0.0);
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = HighLowRangeRatio::new();
        ind.update_ticker(&ticker(50000.0, Some(51000.0), Some(49000.0)));
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(r) = ind.value() {
            assert_eq!(r, 0.0);
        }
    }
}
