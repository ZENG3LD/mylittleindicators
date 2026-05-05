//! Relative Strength Index (RSI) indicator.

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::ohlcv_field::OhlcvField;

/// RSI calculation mode for backward compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RsiMode {
    /// SMA-based RSI (Cutler's RSI)
    Classic,
    /// RMA-based RSI (Wilder's original)
    Wilder,
}

/// Relative Strength Index (RSI) - momentum oscillator measuring speed and change of price movements.
///
/// RSI = 100 - 100 / (1 + RS)
///
/// where RS = Average Gain / Average Loss over the specified period.
///
/// RSI oscillates between 0 and 100 (returned as 0.0-1.0). Traditional interpretation:
/// - Above 0.70 (70): Overbought condition
/// - Below 0.30 (30): Oversold condition
///
/// # Parameters
/// - `period`: Lookback period (default: 14)
/// - `ma_type`: Moving average type for smoothing gains/losses
///
/// # Implementation
///
/// Uses configurable moving average for gain/loss smoothing. O(1) update complexity.
#[derive(Debug, Clone)]
pub struct Rsi {
    prev: f64,
    has_inputs: bool,
    gain_ma: MovingAverageProvider,
    loss_ma: MovingAverageProvider,
    value: f64,
    count: usize,
    initialized: bool,
    source: OhlcvField,
}

impl Rsi {
    /// Creates a new RSI with the specified period using Wilder's RMA smoothing.
    ///
    /// # Arguments
    /// * `period` - Lookback period (typically 14)
    pub fn new(period: usize) -> Self {
        Self::with_source(period, MovingAverageType::RMA, OhlcvField::Close)
    }

    /// Creates a new RSI with custom moving average type.
    ///
    /// # Arguments
    /// * `period` - Lookback period
    /// * `ma_type` - Moving average type for gain/loss smoothing
    pub fn with_ma_type(period: usize, ma_type: MovingAverageType) -> Self {
        Self::with_source(period, ma_type, OhlcvField::Close)
    }

    /// Creates a new RSI with custom source field.
    ///
    /// # Arguments
    /// * `period` - Lookback period
    /// * `ma_type` - Moving average type for gain/loss smoothing
    /// * `source` - OHLCV field to use as input
    pub fn with_source(period: usize, ma_type: MovingAverageType, source: OhlcvField) -> Self {
        Self {
            prev: 0.0,
            has_inputs: false,
            gain_ma: MovingAverageProvider::new(ma_type, period),
            loss_ma: MovingAverageProvider::new(ma_type, period),
            value: 0.0,
            count: 0,
            initialized: false,
            source,
        }
    }

    /// Creates a new RSI with legacy mode selection.
    ///
    /// # Arguments
    /// * `period` - Lookback period
    /// * `mode` - Classic (SMA) or Wilder (RMA)
    pub fn with_mode(period: usize, mode: RsiMode) -> Self {
        let ma_type = match mode {
            RsiMode::Classic => MovingAverageType::SMA,
            RsiMode::Wilder => MovingAverageType::RMA,
        };
        Self::with_source(period, ma_type, OhlcvField::Close)
    }

    /// Updates the RSI with a new bar and returns the current value.
    ///
    /// Uses the configured source field to extract the value from OHLCV data.
    /// Returns RSI as 0.0-1.0 (multiply by 100 for traditional 0-100 scale).
    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let value = self.source.extract(open, high, low, close, volume);

        if !self.has_inputs {
            self.prev = value;
            self.has_inputs = true;
            return self.value;
        }

        let diff = value - self.prev;
        let gain = if diff > 0.0 { diff } else { 0.0 };
        let loss = if diff < 0.0 { -diff } else { 0.0 };
        self.prev = value;
        self.count += 1;

        let _gain_value = self.gain_ma.update_bar(0.0, 0.0, 0.0, gain, 0.0);
        let _loss_value = self.loss_ma.update_bar(0.0, 0.0, 0.0, loss, 0.0);

        if self.gain_ma.is_ready() && self.loss_ma.is_ready() {
            self.initialized = true;
            let avg_gain = self.gain_ma.value().main();
            let avg_loss = self.loss_ma.value().main();

            if avg_loss.abs() < 1e-12 {
                self.value = 100.0; // 100 if no losses
            } else {
                let rs = avg_gain / avg_loss;
                self.value = 100.0 * (1.0 - (1.0 / (1.0 + rs)));
            }
        }

        self.value
    }

    /// Returns the current RSI value as an `IndicatorValue`.
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    /// Returns `true` if the RSI has received enough bars to produce a valid value.
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.initialized
    }

    /// Resets the RSI to its initial state.
    pub fn reset(&mut self) {
        self.prev = 0.0;
        self.has_inputs = false;
        self.gain_ma.reset();
        self.loss_ma.reset();
        self.value = 0.0;
        self.count = 0;
        self.initialized = false;
    }

    /// Returns the number of bars processed.
    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns the period of this RSI.
    #[inline]
    pub fn period(&self) -> usize {
        self.gain_ma.period()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Functional tests
    // =========================================================================

    #[test]
    fn test_rsi_basic_calculation() {
        let mut rsi = Rsi::new(14);

        // Feed uptrend data - RSI should be high
        for i in 1..=20 {
            rsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(rsi.is_ready());
        let val = rsi.value().main();
        assert!(val > 50.0, "RSI in uptrend should be above 50, got {}", val);
    }

    #[test]
    fn test_rsi_downtrend() {
        let mut rsi = Rsi::new(14);

        // Feed downtrend data - RSI should be low
        for i in 1..=20 {
            rsi.update_bar(0.0, 0.0, 0.0, 200.0 - i as f64, 0.0);
        }

        assert!(rsi.is_ready());
        let val = rsi.value().main();
        assert!(val < 50.0, "RSI in downtrend should be below 50, got {}", val);
    }

    #[test]
    fn test_rsi_range() {
        let mut rsi = Rsi::new(14);

        // Feed mixed data
        for i in 1..=30 {
            let price = if i % 2 == 0 { 100.0 + i as f64 } else { 100.0 - i as f64 };
            rsi.update_bar(0.0, 0.0, 0.0, price, 0.0);
        }

        if rsi.is_ready() {
            let val = rsi.value().main();
            assert!(val >= 0.0 && val <= 100.0, "RSI should be in [0, 100], got {}", val);
        }
    }

    #[test]
    fn test_rsi_with_mode_classic() {
        let mut rsi = Rsi::with_mode(14, RsiMode::Classic);

        for i in 1..=20 {
            rsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(rsi.is_ready());
    }

    #[test]
    fn test_rsi_with_ma_type() {
        let mut rsi = Rsi::with_ma_type(14, MovingAverageType::EMA);

        for i in 1..=20 {
            rsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }

        assert!(rsi.is_ready());
    }

    #[test]
    fn test_rsi_reset() {
        let mut rsi = Rsi::new(14);

        for i in 1..=20 {
            rsi.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(rsi.is_ready());

        rsi.reset();
        assert!(!rsi.is_ready());
        assert_eq!(rsi.count(), 0);
    }

    #[test]
    fn test_rsi_period_getter() {
        let rsi = Rsi::new(14);
        assert_eq!(rsi.period(), 14);

        let rsi2 = Rsi::new(21);
        assert_eq!(rsi2.period(), 21);
    }

    #[test]
    fn test_rsi_all_gains() {
        let mut rsi = Rsi::new(5);

        // Strictly increasing prices - no losses
        for i in 1..=10 {
            rsi.update_bar(0.0, 0.0, 0.0, i as f64 * 10.0, 0.0);
        }

        if rsi.is_ready() {
            let val = rsi.value().main();
            assert!((val - 100.0).abs() < 1.0, "RSI with all gains should be ~100, got {}", val);
        }
    }

    #[test]
    fn test_rsi_with_source_hl2() {
        let mut rsi = Rsi::with_source(14, MovingAverageType::RMA, OhlcvField::HL2);

        // Feed data with clear high/low pattern
        for i in 1..=20 {
            let high = 110.0 + i as f64;
            let low = 90.0 + i as f64;
            rsi.update_bar(100.0, high, low, 105.0, 0.0);
        }

        assert!(rsi.is_ready());
        let val = rsi.value().main();
        assert!(val > 50.0, "RSI with HL2 in uptrend should be above 50, got {}", val);
    }

    #[test]
    fn test_rsi_with_source_open() {
        let mut rsi = Rsi::with_source(14, MovingAverageType::RMA, OhlcvField::Open);

        // Feed data with increasing open prices
        for i in 1..=20 {
            let open = 100.0 + i as f64;
            rsi.update_bar(open, 110.0, 90.0, 105.0, 0.0);
        }

        assert!(rsi.is_ready());
        let val = rsi.value().main();
        assert!(val > 50.0, "RSI with Open in uptrend should be above 50, got {}", val);
    }

    #[test]
    fn test_rsi_different_sources_produce_different_results() {
        let mut rsi_close = Rsi::with_source(5, MovingAverageType::RMA, OhlcvField::Close);
        let mut rsi_open = Rsi::with_source(5, MovingAverageType::RMA, OhlcvField::Open);

        // Feed same data to both with different open/close patterns
        // Close is trending up, open is trending down
        for i in 1..=15 {
            let open = 105.0 - (i as f64 * 0.5);  // Trending down
            let high = 120.0;
            let low = 80.0;
            let close = 100.0 + (i as f64 * 0.5);  // Trending up
            let volume = 1000.0;

            rsi_close.update_bar(open, high, low, close, volume);
            rsi_open.update_bar(open, high, low, close, volume);
        }

        if rsi_close.is_ready() && rsi_open.is_ready() {
            let val_close = rsi_close.value().main();
            let val_open = rsi_open.value().main();

            // Close is trending up (RSI should be high)
            assert!(val_close > 60.0, "RSI with uptrending close should be high, got {}", val_close);
            // Open is trending down (RSI should be low)
            assert!(val_open < 40.0, "RSI with downtrending open should be low, got {}", val_open);
            // They should definitely be different
            assert_ne!(val_close, val_open, "RSI with different sources should produce different results");
        }
    }
}
