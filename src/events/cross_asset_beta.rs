//! CrossAssetBeta — rolling beta of primary asset returns vs secondary asset returns.
//!
//! Consumer: `TickConsumer` (primary price via `update_tick`).
//! Secondary: `update_secondary_price(price, timestamp)`.
//!
//! Formula: β = cov(primary_returns, secondary_returns) / var(secondary_returns)
//! computed over a rolling window of return pairs.
//!
//! Output: `Single(beta)`.

use std::collections::VecDeque;

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::tick_consumer::TickConsumer;
use crate::core::types::Tick;

/// Rolling cross-asset beta indicator.
///
/// Primary prices arrive via `TickConsumer::update_tick` (or `update_bar` for close).
/// Secondary prices arrive via `update_secondary_price(price, timestamp)`.
#[derive(Debug, Clone)]
pub struct CrossAssetBeta {
    window: usize,
    primary_prices: VecDeque<f64>,
    secondary_prices: VecDeque<f64>,
    last_beta: f64,
}

impl CrossAssetBeta {
    /// Create a new indicator.
    ///
    /// - `window` — number of price samples for rolling beta (default 50).
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(4),
            primary_prices: VecDeque::with_capacity(window.max(4) + 1),
            secondary_prices: VecDeque::with_capacity(window.max(4) + 1),
            last_beta: 0.0,
        }
    }

    /// Update secondary symbol price.
    pub fn update_secondary_price(&mut self, price: f64, _timestamp: i64) -> IndicatorValue {
        if self.secondary_prices.len() >= self.window + 1 {
            self.secondary_prices.pop_front();
        }
        self.secondary_prices.push_back(price);
        self.recompute();
        self.indicator_value()
    }

    fn push_primary(&mut self, price: f64) {
        if self.primary_prices.len() >= self.window + 1 {
            self.primary_prices.pop_front();
        }
        self.primary_prices.push_back(price);
        self.recompute();
    }

    fn recompute(&mut self) {
        // Need at least 2 paired prices to compute returns
        let n_prices = self.primary_prices.len().min(self.secondary_prices.len());
        if n_prices < 2 {
            self.last_beta = 0.0;
            return;
        }
        let n_returns = n_prices - 1;
        if n_returns < 3 {
            self.last_beta = 0.0;
            return;
        }

        // Compute return series
        let primary_vec: Vec<f64> = self.primary_prices.iter().cloned().collect();
        let secondary_vec: Vec<f64> = self.secondary_prices.iter().cloned().collect();

        let use_p = primary_vec.len().min(secondary_vec.len());
        // take last `use_p` prices
        let pa = &primary_vec[primary_vec.len() - use_p..];
        let pb = &secondary_vec[secondary_vec.len() - use_p..];

        let ret_a: Vec<f64> = pa.windows(2).map(|w| if w[0].abs() > 1e-12 { (w[1] - w[0]) / w[0] } else { 0.0 }).collect();
        let ret_b: Vec<f64> = pb.windows(2).map(|w| if w[0].abs() > 1e-12 { (w[1] - w[0]) / w[0] } else { 0.0 }).collect();

        let n = ret_a.len();
        if n < 2 {
            self.last_beta = 0.0;
            return;
        }

        let mean_a = ret_a.iter().sum::<f64>() / n as f64;
        let mean_b = ret_b.iter().sum::<f64>() / n as f64;

        let cov: f64 = ret_a.iter().zip(ret_b.iter()).map(|(a, b)| (a - mean_a) * (b - mean_b)).sum::<f64>() / n as f64;
        let var_b: f64 = ret_b.iter().map(|b| (b - mean_b).powi(2)).sum::<f64>() / n as f64;

        self.last_beta = if var_b.abs() > 1e-12 { cov / var_b } else { 0.0 };
    }

    /// Passthrough for bar pipeline — uses close as primary.
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> IndicatorValue {
        self.push_primary(c);
        self.indicator_value()
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.last_beta)
    }

    /// True when enough return pairs are available.
    pub fn indicator_is_ready(&self) -> bool {
        self.primary_prices.len() >= 4 && self.secondary_prices.len() >= 4
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.primary_prices.clear();
        self.secondary_prices.clear();
        self.last_beta = 0.0;
    }
}

impl Default for CrossAssetBeta {
    fn default() -> Self {
        Self::new(50)
    }
}

impl TickConsumer for CrossAssetBeta {
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
    fn beta_near_one_for_identical_assets() {
        // A mirrors B with varying returns → β = 1.0
        let mut ind = CrossAssetBeta::new(20);
        // Use varying returns so variance(B) > 0
        let returns = [0.01, -0.02, 0.03, -0.01, 0.02, -0.015, 0.025, -0.005,
                       0.01, 0.02, -0.01, 0.015, -0.02, 0.01, -0.005, 0.03,
                       -0.01, 0.02, -0.015, 0.01];
        let mut price = 100.0;
        for &ret in &returns {
            price *= 1.0 + ret;
            ind.update_tick(&make_tick(price));
            ind.update_secondary_price(price, 0);
        }
        if let IndicatorValue::Single(beta) = ind.indicator_value() {
            assert!((beta - 1.0).abs() < 0.05, "beta={beta} expected ~1.0");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn beta_near_two_for_double_asset() {
        // A = 2 × returns of B → β ≈ 2.0
        let mut ind = CrossAssetBeta::new(30);
        let mut b_price = 100.0;
        let mut a_price = 100.0;
        for i in 1..30 {
            let ret = 0.01 * (i as f64 % 3.0 - 1.0); // oscillating returns
            b_price *= 1.0 + ret;
            a_price *= 1.0 + 2.0 * ret;
            ind.update_tick(&make_tick(a_price));
            ind.update_secondary_price(b_price, 0);
        }
        if let IndicatorValue::Single(beta) = ind.indicator_value() {
            assert!((beta - 2.0).abs() < 0.5, "beta={beta} expected ~2.0");
        } else {
            panic!("expected Single");
        }
    }

    #[test]
    fn reset_clears_state() {
        let mut ind = CrossAssetBeta::default();
        for i in 0..10 {
            ind.update_tick(&make_tick(100.0 + i as f64));
            ind.update_secondary_price(50.0 + i as f64, 0);
        }
        ind.indicator_reset();
        assert!(!ind.indicator_is_ready());
        assert_eq!(ind.indicator_value(), IndicatorValue::Single(0.0));
    }
}
