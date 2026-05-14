//! Funding Z-Score — rolling Z-score of funding rate vs window mean and std.
//!
//! Measures how extreme the current funding rate is relative to recent history.
//! High positive = unusually high funding (market leaning long).
//! High negative = unusually low/negative funding (market leaning short).
//!
//! Output: `Single(z_score)`.

use std::collections::VecDeque;

use crate::bar_indicators::funding_rate_consumer::FundingRateConsumer;
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::core::types::FundingRate;

/// Rolling Z-score of funding rate.
#[derive(Clone)]
pub struct FundingZScore {
    window: usize,
    rates: VecDeque<f64>,
    last_zscore: f64,
}

impl FundingZScore {
    /// Create with given lookback window.
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            rates: VecDeque::new(),
            last_zscore: 0.0,
        }
    }

    fn compute_zscore(&self, current: f64) -> f64 {
        let n = self.rates.len();
        if n < 2 {
            return 0.0;
        }
        let mean = self.rates.iter().sum::<f64>() / n as f64;
        let variance = self.rates.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        let std = variance.sqrt();
        if std < 1e-15 {
            0.0
        } else {
            (current - mean) / std
        }
    }
}

impl FundingRateConsumer for FundingZScore {
    fn update_funding(&mut self, fr: &FundingRate) -> IndicatorValue {
        self.rates.push_back(fr.rate);
        if self.rates.len() > self.window {
            self.rates.pop_front();
        }
        self.last_zscore = self.compute_zscore(fr.rate);
        IndicatorValue::Single(self.last_zscore)
    }

    fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_zscore)
    }

    fn reset(&mut self) {
        self.rates.clear();
        self.last_zscore = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.rates.len() >= self.window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate { symbol: "BTCUSDT".to_string(), rate, next_funding_time: None, timestamp: 1000 }
    }

    #[test]
    fn not_ready_until_window_full() {
        let mut fz = FundingZScore::new(5);
        for i in 0..4 {
            fz.update_funding(&make_fr(i as f64 * 0.0001));
        }
        assert!(!fz.is_ready());
        fz.update_funding(&make_fr(0.0005));
        assert!(fz.is_ready());
    }

    #[test]
    fn zscore_near_zero_for_mean_value() {
        let mut fz = FundingZScore::new(10);
        // Fill with uniform rates so std is small and current = mean
        for _ in 0..10 {
            fz.update_funding(&make_fr(0.0001));
        }
        // Z-score of constant series is 0
        assert!((fz.value().main()).abs() < 1e-9);
    }

    #[test]
    fn zscore_positive_for_high_rate() {
        let mut fz = FundingZScore::new(10);
        for _ in 0..9 {
            fz.update_funding(&make_fr(0.0001));
        }
        // Now push a very high rate
        fz.update_funding(&make_fr(0.01));
        assert!(fz.is_ready());
        assert!(fz.value().main() > 1.0, "high rate should give positive z-score");
    }

    #[test]
    fn reset_clears_state() {
        let mut fz = FundingZScore::new(5);
        for _ in 0..10 {
            fz.update_funding(&make_fr(0.0001));
        }
        fz.reset();
        assert!(!fz.is_ready());
        assert_eq!(fz.value().main(), 0.0);
    }
}
