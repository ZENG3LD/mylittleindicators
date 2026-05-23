//! PairsCointegrationProxy — simplified cointegration proxy via z-score of rolling spread.
//!
//! Consumer: `TickConsumer` (symbol A primary price via `update_tick` / close via `update_bar`).
//! Secondary symbol B: `update_secondary_price(price, timestamp)`.
//!
//! **Honest note**: This is NOT Engle-Granger ADF cointegration testing.
//! It is a simplified proxy:
//! - Rolling OLS regression β = cov(A,B) / var(B) over a fixed window.
//! - Spread = price_A - β × price_B.
//! - Z-score = (spread - mean_spread) / std_spread.
//!
//! Output: `Single(z_score_spread)`.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Cointegration proxy via rolling OLS spread z-score.
///
/// Primary prices arrive via `TickConsumer::update_tick` (or `update_bar` for close).
/// Secondary prices arrive via `update_secondary_price(price, timestamp)`.
///
/// **Note**: β is estimated by simple rolling OLS (cov/var), not ADF test.
/// This is a practical approximation, not a formal cointegration test.
#[derive(Debug, Clone)]
pub struct PairsCointegrationProxy {
    window: usize,
    primary_prices: VecDeque<f64>,
    secondary_prices: VecDeque<f64>,
    last_zscore: f64,
}

impl PairsCointegrationProxy {
    /// Create a new indicator.
    ///
    /// - `window` — number of paired samples for rolling regression (default 50).
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(4),
            primary_prices: VecDeque::with_capacity(window.max(4)),
            secondary_prices: VecDeque::with_capacity(window.max(4)),
            last_zscore: 0.0,
        }
    }

    /// Update secondary symbol price. Call this for each secondary price update.
    /// Returns current z-score of the spread.
    pub fn update_secondary_price(&mut self, price: f64, _timestamp: i64) -> IndicatorValue {
        // Store secondary price; will pair with primary on next sync
        if self.secondary_prices.len() >= self.window {
            self.secondary_prices.pop_front();
        }
        self.secondary_prices.push_back(price);
        self.recompute();
        self.indicator_value()
    }

    fn push_primary(&mut self, price: f64) {
        if self.primary_prices.len() >= self.window {
            self.primary_prices.pop_front();
        }
        self.primary_prices.push_back(price);
        self.recompute();
    }

    fn recompute(&mut self) {
        let n = self.primary_prices.len().min(self.secondary_prices.len());
        if n < 4 {
            self.last_zscore = 0.0;
            return;
        }
        // Use last n paired observations
        let pa: Vec<f64> = self.primary_prices.iter().rev().take(n).cloned().collect();
        let pb: Vec<f64> = self.secondary_prices.iter().rev().take(n).cloned().collect();

        let mean_a = pa.iter().sum::<f64>() / n as f64;
        let mean_b = pb.iter().sum::<f64>() / n as f64;

        let cov_ab: f64 = pa.iter().zip(pb.iter()).map(|(a, b)| (a - mean_a) * (b - mean_b)).sum::<f64>() / n as f64;
        let var_b: f64 = pb.iter().map(|b| (b - mean_b).powi(2)).sum::<f64>() / n as f64;

        let beta = if var_b.abs() > 1e-12 { cov_ab / var_b } else { 1.0 };

        // Compute spreads
        let spreads: Vec<f64> = pa.iter().zip(pb.iter()).map(|(a, b)| a - beta * b).collect();
        let mean_spread = spreads.iter().sum::<f64>() / n as f64;
        let var_spread: f64 = spreads.iter().map(|s| (s - mean_spread).powi(2)).sum::<f64>() / n.saturating_sub(1).max(1) as f64;
        let std_spread = var_spread.sqrt();

        if std_spread < 1e-12 {
            self.last_zscore = 0.0;
        } else {
            let current_spread = spreads[0]; // most recent (reversed order)
            self.last_zscore = (current_spread - mean_spread) / std_spread;
        }
    }

    /// Passthrough for bar pipeline — uses close price as primary.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> IndicatorValue {
        self.push_primary(c);
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_zscore)
    }

    /// True when window is filled with paired observations.
    pub fn indicator_is_ready(&self) -> bool {
        self.primary_prices.len() >= 4 && self.secondary_prices.len() >= 4
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.primary_prices.clear();
        self.secondary_prices.clear();
        self.last_zscore = 0.0;
    }
}

impl Default for PairsCointegrationProxy {
    fn default() -> Self {
        Self::new(50)
    }
}

impl TickConsumer for PairsCointegrationProxy {
    fn update_tick(&mut self, tick: &Tick) -> IndicatorValue {
        self.push_primary(tick.price);
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

    fn make_tick(price: f64) -> Tick {
        Tick::new(0, price, 1.0, true)
    }

    #[test]
    fn zscore_near_zero_for_cointegrated_pair() {
        // A = 2×B → β should converge to 2, spread ≈ 0
        let mut ind = PairsCointegrationProxy::new(20);
        for i in 0..20 {
            let b = 100.0 + i as f64;
            let a = 2.0 * b;
            ind.update_tick(&make_tick(a));
            ind.update_secondary_price(b, 0);
        }
        if let IndicatorValue::Single(z) = ind.indicator_value() {
            // spread should be near 0, z-score should be near 0
            assert!(z.abs() < 1.0, "z={z} expected near 0 for cointegrated pair");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn not_ready_before_min_samples() {
        let mut ind = PairsCointegrationProxy::new(20);
        ind.update_tick(&make_tick(100.0));
        ind.update_secondary_price(50.0, 0);
        assert!(!ind.indicator_is_ready());
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = PairsCointegrationProxy::default();
        for i in 0..10 {
            ind.update_tick(&make_tick(100.0 + i as f64));
            ind.update_secondary_price(50.0 + i as f64, 0);
        }
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        assert_eq!(ind.indicator_value(), IndicatorValue::Single(0.0));
    }
}
