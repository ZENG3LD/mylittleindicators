//! Jurik Moving Average (JMA) approximation indicator.

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use super::super::ohlcv_field::OhlcvField;

/// Jurik Moving Average (JMA) approximation - low-lag adaptive smoother.
///
/// This is an approximation of the proprietary Jurik Moving Average
/// using a blend of fast and slow EMAs controlled by a phase parameter.
///
/// JMA ≈ w × EMA_fast + (1-w) × EMA_slow
///
/// where w = (phase + 100) / 200.
///
/// # Parameters
/// - `period`: Base EMA period (fast uses period, slow uses period×2)
/// - `phase`: Controls weighting between fast/slow (-100 to +100)
///
/// # Implementation
///
/// Uses two O(1) EMA instances, so overall complexity is O(1).
#[derive(Debug, Clone)]
pub struct JurikMa {
    period: usize,
    source: OhlcvField,
    ema_fast: MovingAverageProvider,
    ema_slow: MovingAverageProvider,
    phase: f64,
    value: f64,
}

impl JurikMa {
    /// Creates a new JMA with the specified period and phase.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Base smoothing period
    /// * `phase` - Blend factor (-100 = slow, +100 = fast)
    pub fn new(period: usize, phase: f64) -> Self {
        Self::with_source(period, phase, OhlcvField::Close)
    }

    /// Creates a new JMA with the specified period, phase, and source.
    ///
    /// # Arguments
    /// * `period` - Base smoothing period
    /// * `phase` - Blend factor (-100 = slow, +100 = fast)
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period: usize, phase: f64, source: OhlcvField) -> Self {
        let p = period.max(1);
        Self {
            period: p,
            source,
            ema_fast: MovingAverageProvider::new(MovingAverageType::EMA, p),
            ema_slow: MovingAverageProvider::new(MovingAverageType::EMA, (p * 2).max(2)),
            phase,
            value: 0.0,
        }
    }

    /// Returns `true` if the JMA has received enough bars to produce a valid value.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ema_fast.is_ready() && self.ema_slow.is_ready()
    }

    /// Returns the current JMA value as an `IndicatorValue`.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Resets the JMA to its initial state.
    #[inline]
    pub fn reset(&mut self) {
        self.ema_fast.reset();
        self.ema_slow.reset();
        self.value = 0.0;
    }

    /// Updates the JMA with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close).
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let value = self.source.extract(open, high, low, close, volume);
        let f = self.ema_fast.update_bar(0.0, 0.0, 0.0, value, 0.0);
        let s = self.ema_slow.update_bar(0.0, 0.0, 0.0, value, 0.0);
        let w = (self.phase.clamp(-100.0, 100.0) + 100.0) / 200.0;
        self.value = w * f + (1.0 - w) * s;
        self.value
    }

    /// Returns the period of this JMA.
    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jma_basic_calculation() {
        let mut jma = JurikMa::new(10, 0.0);

        for i in 1..=30 {
            jma.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }

        assert!(jma.is_ready());
        assert!(jma.value().main() > 0.0);
    }

    #[test]
    fn test_jma_phase_effect() {
        let mut jma_fast = JurikMa::new(10, 100.0);
        let mut jma_slow = JurikMa::new(10, -100.0);

        for i in 1..=30 {
            jma_fast.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
            jma_slow.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }

        // Fast phase should be closer to current price (higher in uptrend)
        assert!(jma_fast.value().main() > jma_slow.value().main());
    }

    #[test]
    fn test_jma_reset() {
        let mut jma = JurikMa::new(5, 0.0);
        for i in 1..=20 {
            jma.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }
        assert!(jma.is_ready());

        jma.reset();
        assert!(!jma.is_ready());
    }

}
