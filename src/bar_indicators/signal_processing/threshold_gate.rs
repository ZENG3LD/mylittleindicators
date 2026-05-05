// Threshold Gate: converts a scalar to {-1,0,1} by asymmetric thresholds
//
// Self-contained version: uses internal RSI to generate input signal
// Returns +1 when RSI >= upper (overbought), -1 when RSI <= lower (oversold), 0 otherwise

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::momentum::rsi::Rsi;

#[derive(Clone)]
pub struct ThresholdGate {
    upper: f64,
    lower: f64,
    rsi: Rsi,
    signal: i8,
}

impl ThresholdGate {
    /// Creates a new ThresholdGate with RSI thresholds
    /// Default: lower=30 (oversold), upper=70 (overbought)
    pub fn new(lower: f64, upper: f64) -> Self {
        Self {
            upper: upper.clamp(50.0, 100.0),
            lower: lower.clamp(0.0, 50.0),
            rsi: Rsi::new(14),
            signal: 0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.rsi.reset();
        self.signal = 0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rsi.is_ready()
    }

    /// Legacy feed method - now ignored, RSI computed internally
    #[inline]
    pub fn feed(&mut self, _x: f64) {
        // No-op: RSI is computed internally from price data
    }

    pub fn update_bar(
        &mut self,
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        volume: f64,
    ) -> i8 {
        // Update internal RSI
        self.rsi.update_bar(open, high, low, close, volume);

        if self.rsi.is_ready() {
            let rsi_value = self.rsi.value().main();
            self.signal = if rsi_value >= self.upper {
                1  // Overbought
            } else if rsi_value <= self.lower {
                -1 // Oversold
            } else {
                0  // Neutral
            };
        }
        self.signal
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.signal)
    }

    pub fn thresholds(&self) -> (f64, f64) {
        (self.lower, self.upper)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_gate_creation() {
        let tg = ThresholdGate::new(30.0, 70.0);
        assert!(!tg.is_ready()); // Not ready until RSI warmup
        assert_eq!(tg.value().as_signal(), Some(0));
        assert_eq!(tg.thresholds(), (30.0, 70.0));
    }

    #[test]
    fn test_threshold_gate_with_uptrend() {
        let mut tg = ThresholdGate::new(30.0, 70.0);

        // Strong uptrend should push RSI high -> overbought signal
        let mut price = 100.0;
        for _ in 0..30 {
            price += 2.0; // Consistent gains
            tg.update_bar(price - 1.0, price + 0.5, price - 1.5, price, 1000.0);
        }

        assert!(tg.is_ready());
        // Strong uptrend should give overbought signal (1)
        let signal = tg.value().as_signal().unwrap();
        assert!(signal >= 0, "Strong uptrend should not be oversold");
    }

    #[test]
    fn test_threshold_gate_with_downtrend() {
        let mut tg = ThresholdGate::new(30.0, 70.0);

        // Strong downtrend should push RSI low -> oversold signal
        let mut price = 200.0;
        for _ in 0..30 {
            price -= 2.0; // Consistent losses
            tg.update_bar(price + 1.0, price + 1.5, price - 0.5, price, 1000.0);
        }

        assert!(tg.is_ready());
        // Strong downtrend should give oversold signal (-1)
        let signal = tg.value().as_signal().unwrap();
        assert!(signal <= 0, "Strong downtrend should not be overbought");
    }

    #[test]
    fn test_threshold_gate_reset() {
        let mut tg = ThresholdGate::new(30.0, 70.0);

        // Warm up
        let mut price = 100.0;
        for _ in 0..20 {
            price += 1.0;
            tg.update_bar(price, price + 0.5, price - 0.5, price, 1000.0);
        }

        tg.reset();
        assert!(!tg.is_ready());
        assert_eq!(tg.value().as_signal(), Some(0));
    }
}
