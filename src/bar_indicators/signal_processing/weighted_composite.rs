// Weighted Composite: weighted sum of 4 internal SMAs with different periods
// Self-contained version: uses SMA(5), SMA(10), SMA(20), SMA(50) internally
// Returns deviation of close from the weighted average

use crate::bar_indicators::average::sma::Sma;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct WeightedComposite {
    // weights for 4 components
    w: [f64; 4],
    // internal SMAs with periods 5, 10, 20, 50
    sma1: Sma,
    sma2: Sma,
    sma3: Sma,
    sma4: Sma,
    norm: bool, // if true, divide by sum(|w|)
    value: f64,
}

impl WeightedComposite {
    pub fn new(w1: f64, w2: f64, w3: f64, w4: f64, normalize: bool) -> Self {
        Self {
            w: [w1, w2, w3, w4],
            sma1: Sma::new(5),
            sma2: Sma::new(10),
            sma3: Sma::new(20),
            sma4: Sma::new(50),
            norm: normalize,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.sma1.reset();
        self.sma2.reset();
        self.sma3.reset();
        self.sma4.reset();
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.sma4.is_ready() // longest period determines readiness
    }

    /// Legacy method - kept for compatibility but no longer used internally
    #[inline]
    pub fn update_inputs(&mut self, _i1: f64, _i2: f64, _i3: f64, _i4: f64) {
        // No-op: SMAs computed internally
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> f64 {
        // Update all SMAs
        self.sma1.update_bar(0.0, 0.0, 0.0, close, 0.0);
        self.sma2.update_bar(0.0, 0.0, 0.0, close, 0.0);
        self.sma3.update_bar(0.0, 0.0, 0.0, close, 0.0);
        self.sma4.update_bar(0.0, 0.0, 0.0, close, 0.0);

        if self.is_ready() {
            let inputs = [
                self.sma1.value().main(),
                self.sma2.value().main(),
                self.sma3.value().main(),
                self.sma4.value().main(),
            ];

            let s: f64 = self.w.iter().zip(inputs.iter()).map(|(w, inp)| w * inp).sum();
            if self.norm {
                let denom = self.w.iter().map(|x| x.abs()).sum::<f64>().max(1e-9);
                self.value = s / denom;
            } else {
                self.value = s;
            }
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn weights(&self) -> [f64; 4] {
        self.w
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weighted_composite_creation() {
        let wc = WeightedComposite::new(1.0, 0.5, 0.25, 0.125, false);
        assert!(!wc.is_ready()); // Not ready until SMAs filled
        assert_eq!(wc.value().main(), 0.0);
        assert_eq!(wc.weights(), [1.0, 0.5, 0.25, 0.125]);
    }

    #[test]
    fn test_weighted_composite_with_data() {
        let mut wc = WeightedComposite::new(1.0, 1.0, 1.0, 1.0, true);

        // Warmup with 60 bars
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            wc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }

        assert!(wc.is_ready());
        let value = wc.value().main();
        // Normalized weighted average should be close to price
        assert!(value > 90.0 && value < 110.0, "Expected value near 100, got {}", value);
    }

    #[test]
    fn test_weighted_composite_reset() {
        let mut wc = WeightedComposite::new(1.0, 1.0, 1.0, 1.0, false);

        for i in 0..60 {
            wc.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }

        wc.reset();
        assert!(!wc.is_ready());
        assert_eq!(wc.value().main(), 0.0);
    }
}
