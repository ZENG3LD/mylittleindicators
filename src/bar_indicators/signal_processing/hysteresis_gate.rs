// Hysteresis Gate: sticky {-1,0,1} with lower/upper thresholds and hold logic
//
// Self-contained version: uses internal RSI to generate input signal
// Holds state until RSI crosses the opposite threshold (reduces whipsaws)

use crate::bar_indicators::indicator_value::IndicatorValue;
use crate::bar_indicators::momentum::rsi::Rsi;

#[derive(Clone)]
pub struct HysteresisGate {
    lower: f64,
    upper: f64,
    state: i8,
    rsi: Rsi,
}

impl HysteresisGate {
    /// Creates a new HysteresisGate with RSI thresholds
    /// Default: lower=30 (oversold), upper=70 (overbought)
    pub fn new(lower: f64, upper: f64) -> Self {
        Self {
            lower: lower.clamp(0.0, 50.0),
            upper: upper.clamp(50.0, 100.0),
            state: 0,
            rsi: Rsi::new(14),
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.state = 0;
        self.rsi.reset();
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

    // Rules: if state<=0 and RSI>=upper => state=+1; if state>=0 and RSI<=lower => state=-1; else keep state
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
            if self.state <= 0 && rsi_value >= self.upper {
                self.state = 1;  // Flip to overbought
            } else if self.state >= 0 && rsi_value <= self.lower {
                self.state = -1; // Flip to oversold
            }
            // Otherwise hold current state (hysteresis)
        }
        self.state
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.state)
    }

    pub fn lower(&self) -> f64 {
        self.lower
    }

    pub fn upper(&self) -> f64 {
        self.upper
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hysteresis_gate_creation() {
        let hg = HysteresisGate::new(30.0, 70.0);
        assert!(!hg.is_ready()); // Not ready until RSI warmup
        assert_eq!(hg.value().as_signal(), Some(0));
        assert!((hg.lower() - 30.0).abs() < 1e-9);
        assert!((hg.upper() - 70.0).abs() < 1e-9);
    }

    #[test]
    fn test_hysteresis_gate_with_trend() {
        let mut hg = HysteresisGate::new(30.0, 70.0);

        // Strong uptrend should eventually trigger overbought state
        let mut price = 100.0;
        for _ in 0..30 {
            price += 2.0;
            hg.update_bar(price - 1.0, price + 0.5, price - 1.5, price, 1000.0);
        }

        assert!(hg.is_ready());
        // State should be either 0 or 1 after uptrend
        let state = hg.value().as_signal().unwrap();
        assert!(state >= 0, "Uptrend should not give oversold signal");
    }

    #[test]
    fn test_hysteresis_gate_reset() {
        let mut hg = HysteresisGate::new(30.0, 70.0);

        let mut price = 100.0;
        for _ in 0..20 {
            price += 1.0;
            hg.update_bar(price, price + 0.5, price - 0.5, price, 1000.0);
        }

        hg.reset();
        assert!(!hg.is_ready());
        assert_eq!(hg.value().as_signal(), Some(0));
    }
}
