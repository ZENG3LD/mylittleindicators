// QQE (Quantitative Qualitative Estimation) — RSI smoothed with ATR-derived bands.
//
// Algorithm:
//   1. Compute RSI(period)
//   2. Smooth RSI with EMA(smooth) → smoothed_rsi
//   3. Compute |smoothed_rsi[i] - smoothed_rsi[i-1]| → rsi_delta
//   4. Smooth rsi_delta with Wilder EMA(smooth*4.236) → atr_rsi
//   5. Upper/lower bands = smoothed_rsi ± threshold_mult * atr_rsi
//   6. QQE line follows bands (trailing stop logic):
//      - rises when smoothed_rsi crosses above upper, falls when below lower
//
// Output: Double(qqe_line, smoothed_rsi)

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::momentum::rsi::Rsi;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct Qqe {
    rsi: Rsi,
    /// First EMA smoothing of RSI
    smoothing: MovingAverageProvider,
    /// Wilder-style smoothing of RSI delta magnitude → ATR-like band width
    atr_smooth: MovingAverageProvider,
    threshold_mult: f64,
    qqe_value: f64,
    smoothed_rsi: f64,
    prev_smoothed_rsi: f64,
    atr_rsi: f64,
    /// QQE long/short trailing level
    qqe_upper: f64,
    qqe_lower: f64,
    /// Long band is trailing below RSI, short band trailing above
    is_long: bool,
}

impl Qqe {
    pub fn new(period: usize, smooth: usize, threshold_mult: f64) -> Self {
        Self::with_ma_types(period, smooth, threshold_mult, MovingAverageType::EMA, MovingAverageType::RMA)
    }

    /// Create QQE with configurable smoothing MA types.
    ///
    /// # Arguments
    /// * `period`          - RSI period
    /// * `smooth`          - EMA smoothing period for RSI
    /// * `threshold_mult`  - Band multiplier (default 1.5 when <= 0)
    /// * `smooth_ma_type`  - MA type for first RSI smoothing (default EMA)
    /// * `atr_ma_type`     - MA type for ATR-band smoothing (default RMA = Wilder)
    pub fn with_ma_types(
        period: usize,
        smooth: usize,
        threshold_mult: f64,
        smooth_ma_type: MovingAverageType,
        atr_ma_type: MovingAverageType,
    ) -> Self {
        let p = period.max(1);
        let s = smooth.max(1);
        let tm = if threshold_mult > 0.0 { threshold_mult } else { 1.5 };
        // Wilder ATR period = smooth * 4.236 (classic QQE constant)
        let atr_period = ((s as f64 * 4.236).round() as usize).max(2);
        Self {
            rsi: Rsi::new(p),
            smoothing: MovingAverageProvider::new(smooth_ma_type, s),
            atr_smooth: MovingAverageProvider::new(atr_ma_type, atr_period),
            threshold_mult: tm,
            qqe_value: 0.0,
            smoothed_rsi: 0.0,
            prev_smoothed_rsi: 0.0,
            atr_rsi: 0.0,
            qqe_upper: 0.0,
            qqe_lower: 100.0,
            is_long: true,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.qqe_value = 0.0;
        self.smoothed_rsi = 0.0;
        self.prev_smoothed_rsi = 0.0;
        self.atr_rsi = 0.0;
        self.qqe_upper = 0.0;
        self.qqe_lower = 100.0;
        self.is_long = true;
        self.smoothing.reset();
        self.atr_smooth.reset();
        self.rsi.reset();
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.smoothing.is_ready() && self.atr_smooth.is_ready()
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.qqe_value, self.smoothed_rsi)
    }

    pub fn update_bar(&mut self, open: f64, high: f64, low: f64, close: f64, volume: f64) -> f64 {
        let _ = self.rsi.update_bar(open, high, low, close, volume);
        let rsi_val = self.rsi.value().main();

        self.prev_smoothed_rsi = self.smoothed_rsi;
        self.smoothed_rsi = self.smoothing.update_bar(0.0, 0.0, 0.0, rsi_val, 0.0);

        // RSI delta magnitude for ATR-like smoothing
        let rsi_delta = (self.smoothed_rsi - self.prev_smoothed_rsi).abs();
        self.atr_rsi = self.atr_smooth.update_bar(0.0, 0.0, 0.0, rsi_delta, 0.0);

        let band_width = self.threshold_mult * self.atr_rsi;

        // Trailing QQE line (long/short band)
        if self.is_long {
            // Long: trailing stop below
            let new_lower = self.smoothed_rsi - band_width;
            if new_lower > self.qqe_lower {
                self.qqe_lower = new_lower;
            }
            if self.smoothed_rsi < self.qqe_lower {
                // Cross below — switch to short
                self.is_long = false;
                self.qqe_upper = self.smoothed_rsi + band_width;
                self.qqe_value = self.qqe_upper;
            } else {
                self.qqe_value = self.qqe_lower;
            }
        } else {
            // Short: trailing stop above
            let new_upper = self.smoothed_rsi + band_width;
            if new_upper < self.qqe_upper {
                self.qqe_upper = new_upper;
            }
            if self.smoothed_rsi > self.qqe_upper {
                // Cross above — switch to long
                self.is_long = true;
                self.qqe_lower = self.smoothed_rsi - band_width;
                self.qqe_value = self.qqe_lower;
            } else {
                self.qqe_value = self.qqe_upper;
            }
        }

        self.qqe_value
    }

    pub fn threshold_mult(&self) -> f64 {
        self.threshold_mult
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qqe_creation() {
        let qqe = Qqe::new(14, 5, 4.236);
        assert!(!qqe.is_ready());
        assert_eq!(qqe.value().main(), 0.0);
        assert!((qqe.threshold_mult() - 4.236).abs() < 1e-10);
    }

    #[test]
    fn test_qqe_with_ma_types() {
        let mut qqe = Qqe::with_ma_types(14, 5, 4.236, MovingAverageType::SMA, MovingAverageType::EMA);
        for i in 1..=100 {
            let p = 100.0 + i as f64 * 0.5;
            let v = qqe.update_bar(p, p + 1.0, p - 1.0, p, 1000.0);
            assert!(v.is_finite());
        }
        assert!(qqe.is_ready());
    }

    #[test]
    fn test_qqe_default_threshold() {
        let qqe = Qqe::new(14, 5, 0.0);
        assert!((qqe.threshold_mult() - 1.5).abs() < 1e-10);
    }

    #[test]
    fn test_qqe_basic_calculation() {
        let mut qqe = Qqe::new(14, 5, 4.236);
        for i in 1..=100 {
            let price = 100.0 + i as f64;
            qqe.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(qqe.is_ready());
        let (qqe_line, smoothed_rsi) = match qqe.value() {
            IndicatorValue::Double(q, s) => (q, s),
            _ => panic!("Expected Double value"),
        };
        assert!(qqe_line.is_finite(), "QQE line should be finite");
        assert!(smoothed_rsi.is_finite(), "Smoothed RSI should be finite");
    }

    #[test]
    fn test_qqe_finite_values() {
        let mut qqe = Qqe::new(14, 5, 4.236);
        for i in 1..=200 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            let value = qqe.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "QQE should always be finite");
        }
    }

    #[test]
    fn test_qqe_reset() {
        let mut qqe = Qqe::new(14, 5, 4.236);
        for i in 1..=100 {
            let price = 100.0 + i as f64;
            qqe.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(qqe.is_ready());
        qqe.reset();
        assert!(!qqe.is_ready());
        assert_eq!(qqe.value().main(), 0.0);
    }
}
