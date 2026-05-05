//! Variable Index Dynamic Average (VIDYA) indicator.

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use super::super::ohlcv_field::OhlcvField;

/// Variable Index Dynamic Average (VIDYA) - volatility-adaptive moving average.
///
/// VIDYA = α × CMO% × Price + (1 - α × CMO%) × VIDYA_prev
///
/// where α = 2/(period+1) and CMO% is the absolute Chande Momentum Oscillator
/// value as a percentage.
///
/// Created by Tushar Chande. Adapts its smoothing factor based on market
/// volatility measured by CMO. More volatile = faster adaptation.
///
/// # Implementation
///
/// Uses internal CMO for volatility measurement. O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Vidya {
    period: usize,
    source: OhlcvField,
    value: f64,
    count: usize,
    alpha: f64,
    cmo: Cmo,
    cmo_pct: f64,
    ready: bool,
}

impl Vidya {
    /// Returns the period of this VIDYA.
    pub fn period(&self) -> usize {
        self.period
    }

    /// Creates a new VIDYA with the specified period.
    ///
    /// Uses Close as the default source.
    ///
    /// # Arguments
    /// * `period` - Smoothing period for CMO and base alpha calculation
    /// * `ma_type` - Moving average type (ignored, uses SMA internally)
    pub fn new(period: usize, ma_type: MovingAverageType) -> Self {
        Self::with_source(period, ma_type, OhlcvField::Close)
    }

    /// Creates a new VIDYA with the specified period and source.
    ///
    /// # Arguments
    /// * `period` - Smoothing period for CMO and base alpha calculation
    /// * `ma_type` - Moving average type (ignored, uses SMA internally)
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period: usize, _ma_type: MovingAverageType, source: OhlcvField) -> Self {
        Self {
            period,
            source,
            value: 0.0,
            count: 0,
            alpha: 2.0 / (period as f64 + 1.0),
            cmo: Cmo::new(period, MovingAverageType::SMA),
            cmo_pct: 0.0,
            ready: false,
        }
    }

    /// Updates the VIDYA with a new bar and returns the current value.
    ///
    /// Extracts the value from the configured source field (default: close).
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let value = self.source.extract(open, high, low, close, volume);
        self.cmo.update(value);
        self.cmo_pct = (self.cmo.value() / 100.0).abs();
        if self.ready {
            self.value = (self.alpha * self.cmo_pct) * value + (1.0 - self.alpha * self.cmo_pct) * self.value;
        }
        if !self.ready && self.cmo.is_ready() {
            self.ready = true;
            self.value = value;
        }
        self.count += 1;
        self.value
    }

    /// Returns the current VIDYA value as an `IndicatorValue`.
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the VIDYA has received enough bars to produce a valid value.
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    /// Resets the VIDYA to its initial state.
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.count = 0;
        self.cmo_pct = 0.0;
        self.ready = false;
        self.cmo = Cmo::new(self.period, MovingAverageType::SMA);
    }
}

/// Internal Chande Momentum Oscillator for VIDYA.
#[derive(Debug, Clone)]
struct Cmo {
    period: usize,
    #[allow(dead_code)]
    ma_type: MovingAverageType,
    gain_ma: MovingAverageProvider,
    loss_ma: MovingAverageProvider,
    prev: f64,
    value: f64,
    filled: bool,
    index: usize,
}

impl Cmo {
    fn new(period: usize, _ma_type: MovingAverageType) -> Self {
        let ma_type = MovingAverageType::SMA;
        Self {
            period,
            ma_type,
            gain_ma: MovingAverageProvider::new(ma_type, period),
            loss_ma: MovingAverageProvider::new(ma_type, period),
            prev: 0.0,
            value: 0.0,
            filled: false,
            index: 0,
        }
    }

    fn update(&mut self, close: f64) {
        if self.index == 0 && self.prev == 0.0 {
            self.prev = close;
            self.index = 1;
            return;
        }
        let diff = close - self.prev;
        let (gain, loss) = if diff > 0.0 { (diff, 0.0) } else { (0.0, -diff) };
        let avg_gain = self.gain_ma.update_bar(0.0, 0.0, 0.0, gain, 0.0);
        let avg_loss = self.loss_ma.update_bar(0.0, 0.0, 0.0, loss, 0.0);
        self.prev = close;
        self.index += 1;
        if self.index >= self.period {
            self.filled = true;
        }
        let denom = avg_gain + avg_loss;
        self.value = if self.filled && denom.abs() >= 1e-12 {
            100.0 * (avg_gain - avg_loss) / denom
        } else {
            0.0
        };
    }

    fn value(&self) -> f64 {
        self.value
    }

    fn is_ready(&self) -> bool {
        self.filled
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vidya_basic_calculation() {
        let mut vidya = Vidya::new(10, MovingAverageType::SMA);

        for i in 1..=20 {
            vidya.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }

        assert!(vidya.is_ready());
        assert!(vidya.value().main() > 0.0);
    }

    #[test]
    fn test_vidya_reset() {
        let mut vidya = Vidya::new(5, MovingAverageType::SMA);
        for i in 1..=10 {
            vidya.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }
        assert!(vidya.is_ready());

        vidya.reset();
        assert!(!vidya.is_ready());
    }

}
