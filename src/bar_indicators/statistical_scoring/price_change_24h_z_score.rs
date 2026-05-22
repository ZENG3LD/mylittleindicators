//! PriceChange24hZScore — rolling Z-score of `price_change_percent_24h`.
//!
//! Measures how unusual the current 24-hour price-change percentage is
//! relative to the recent rolling window (mean ± std).
//!
//! Z-score = (current - mean) / std
//!
//! Returns 0 when std is near-zero (flat distribution).
//!
//! Output: `Single(z_score)`.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ticker_consumer::TickerConsumer;
use crate::core::types::Ticker;

/// Rolling Z-score of 24-hour price-change percentage.
#[derive(Clone)]
pub struct PriceChange24hZScore {
    /// Window length.
    period: usize,
    /// Ring buffer of historical price_change_percent_24h values.
    history: Vec<f64>,
    /// Write cursor.
    cursor: usize,
    /// Number of values seen (capped at period).
    count: usize,
    /// Last computed z-score.
    last_z: f64,
}

impl PriceChange24hZScore {
    /// Create with the given rolling window size.
    pub fn new(period: usize) -> Self {
        let p = period.max(2);
        Self {
            period: p,
            history: vec![0.0; p],
            cursor: 0,
            count: 0,
            last_z: 0.0,
        }
    }

    fn compute_z(&self, current: f64) -> f64 {
        let n = self.count.min(self.period);
        if n < 2 {
            return 0.0;
        }
        let sum: f64 = self.history[..self.period].iter().sum();
        let mean = sum / n as f64;
        let variance: f64 = self.history[..self.period]
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                // only include filled slots
                if self.count >= self.period {
                    true
                } else {
                    *i < self.count
                }
            })
            .map(|(_, &v)| {
                let d = v - mean;
                d * d
            })
            .sum::<f64>()
            / n as f64;
        let std = variance.sqrt();
        if std < 1e-14 {
            0.0
        } else {
            (current - mean) / std
        }
    }
}

impl TickerConsumer for PriceChange24hZScore {
    fn update_ticker(&mut self, ticker: &Ticker) -> IndicatorValue {
        let pct = ticker.price_change_percent_24h.unwrap_or(0.0);
        self.history[self.cursor] = pct;
        self.cursor = (self.cursor + 1) % self.period;
        if self.count < self.period {
            self.count += 1;
        }
        self.last_z = self.compute_z(pct);
        IndicatorValue::Single(self.last_z)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_z)
    }

    fn reset(&mut self) {
        self.history.fill(0.0);
        self.cursor = 0;
        self.count = 0;
        self.last_z = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.count >= self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticker(pct: Option<f64>) -> Ticker {
        Ticker {
            last_price: 50000.0,
            bid_price: None,
            ask_price: None,
            high_24h: None,
            low_24h: None,
            volume_24h: None,
            quote_volume_24h: None,
            price_change_24h: None,
            price_change_percent_24h: pct,
            timestamp: 1000,
        }
    }

    #[test]
    fn not_ready_initially() {
        let ind = PriceChange24hZScore::new(5);
        assert!(!ind.is_ready());
    }

    #[test]
    fn zero_z_on_constant_change() {
        let mut ind = PriceChange24hZScore::new(5);
        for _ in 0..10 {
            ind.update_ticker(&ticker(Some(1.5)));
        }
        assert!(ind.is_ready());
        if let IndicatorValue::Single(z) = ind.value() {
            assert!(z.abs() < 1e-10, "constant change → z=0, got {}", z);
        }
    }

    #[test]
    fn extreme_value_gives_high_z() {
        let mut ind = PriceChange24hZScore::new(5);
        // Feed mostly small values
        for _ in 0..4 {
            ind.update_ticker(&ticker(Some(0.1)));
        }
        // Feed a large outlier
        ind.update_ticker(&ticker(Some(10.0)));
        if let IndicatorValue::Single(z) = ind.value() {
            assert!(z > 1.0, "outlier → z > 1, got {}", z);
        }
    }

    #[test]
    fn missing_pct_treated_as_zero() {
        let mut ind = PriceChange24hZScore::new(3);
        for _ in 0..3 {
            ind.update_ticker(&ticker(None));
        }
        // All zeros → std = 0 → z = 0
        if let IndicatorValue::Single(z) = ind.value() {
            assert_eq!(z, 0.0);
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = PriceChange24hZScore::new(3);
        for _ in 0..5 {
            ind.update_ticker(&ticker(Some(2.0)));
        }
        ind.reset();
        assert!(!ind.is_ready());
        if let IndicatorValue::Single(z) = ind.value() {
            assert_eq!(z, 0.0);
        }
    }
}
