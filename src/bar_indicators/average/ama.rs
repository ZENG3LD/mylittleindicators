//! Kaufman Adaptive Moving Average (AMA) indicator.

use crate::bar_indicators::ratio::efficiency_ratio_ring::EfficiencyRatioRingWindow;
use crate::bar_indicators::indicator_value::IndicatorValue;
use super::super::ohlcv_field::OhlcvField;

/// Kaufman Adaptive Moving Average (AMA) - adapts smoothing based on market efficiency.
///
/// AMA = AMA_prev + SC² × (Price - AMA_prev)
///
/// where SC = ER × (fast_α - slow_α) + slow_α
/// and ER = Efficiency Ratio (direction/volatility)
///
/// Created by Perry Kaufman. Speeds up in trending markets,
/// slows down in choppy markets.
///
/// # Implementation
///
/// Uses Efficiency Ratio with rolling window. O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Ama {
    period_efficiency_ratio: usize,
    source: OhlcvField,
    alpha_fast: f64,
    alpha_slow: f64,
    value: f64,
    count: usize,
    prior_value: Option<f64>,
    efficiency_ratio: EfficiencyRatioRingWindow,
    initialized: bool,
}

impl Ama {
    /// Returns the period of this AMA (efficiency ratio period).
    pub fn period(&self) -> usize {
        self.period_efficiency_ratio
    }

    /// Creates a new AMA with the specified parameters.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period_efficiency_ratio` - Period for efficiency ratio calculation
    /// * `fast_period` - Fast EMA period (default: 2)
    /// * `slow_period` - Slow EMA period (default: 30)
    pub fn new(period_efficiency_ratio: usize, fast_period: usize, slow_period: usize) -> Self {
        Self::with_source(period_efficiency_ratio, fast_period, slow_period, OhlcvField::Close)
    }

    /// Creates a new AMA with the specified parameters and source.
    ///
    /// # Arguments
    /// * `period_efficiency_ratio` - Period for efficiency ratio calculation
    /// * `fast_period` - Fast EMA period (default: 2)
    /// * `slow_period` - Slow EMA period (default: 30)
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period_efficiency_ratio: usize, fast_period: usize, slow_period: usize, source: OhlcvField) -> Self {
        let alpha_fast = 2.0 / (fast_period as f64 + 1.0);
        let alpha_slow = 2.0 / (slow_period as f64 + 1.0);
        Self {
            period_efficiency_ratio,
            source,
            alpha_fast,
            alpha_slow,
            value: 0.0,
            count: 0,
            prior_value: None,
            efficiency_ratio: EfficiencyRatioRingWindow::new(period_efficiency_ratio),
            initialized: false,
        }
    }

    /// Updates the AMA with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close).
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let value = self.source.extract(open, high, low, close, volume);
        if self.count == 0 {
            self.value = value;
            self.prior_value = Some(value);
            self.efficiency_ratio.update_raw(value);
            self.count += 1;
            return self.value;
        }
        self.efficiency_ratio.update_raw(value);
        let er = self.efficiency_ratio.value().main();
        let smoothing_constant = (er * (self.alpha_fast - self.alpha_slow) + self.alpha_slow).powi(2);
        let prior = self.prior_value.unwrap_or(self.value);
        self.value = prior + smoothing_constant * (value - prior);
        self.prior_value = Some(self.value);
        self.count += 1;
        if self.efficiency_ratio.is_initialized() {
            self.initialized = true;
        }
        self.value
    }

    /// Returns the current AMA value as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the AMA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.initialized
    }

    /// Resets the AMA to its initial state.
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.count = 0;
        self.prior_value = None;
        self.efficiency_ratio.reset();
        self.initialized = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ama_basic_calculation() {
        let mut ama = Ama::new(10, 2, 30);

        // Feed trending data - AMA should follow closely
        for i in 1..=20 {
            ama.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }

        assert!(ama.is_ready());
        // In a perfect trend, AMA should be close to recent prices
        assert!(ama.value().main() > 100.0);
    }

    #[test]
    fn test_ama_adapts_to_choppy_market() {
        let mut ama = Ama::new(10, 2, 30);

        // Feed choppy data - AMA should be slow
        for i in 0..30 {
            let price = if i % 2 == 0 { 100.0 } else { 110.0 };
            ama.update_bar(0.0, 0.0, 0.0, price, 0.0);
        }

        assert!(ama.is_ready());
        // In choppy market, AMA should be somewhere in the middle
        let v = ama.value().main();
        assert!(v > 100.0 && v < 110.0);
    }

    #[test]
    fn test_ama_reset() {
        let mut ama = Ama::new(10, 2, 30);
        for i in 1..=15 {
            ama.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }
        assert!(ama.is_ready());

        ama.reset();
        assert!(!ama.is_ready());
    }
}
