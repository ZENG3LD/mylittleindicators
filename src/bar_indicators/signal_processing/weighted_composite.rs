// Weighted Composite: weighted sum of 4 internal MAs with different periods
// Self-contained version: defaults to SMA(5), SMA(10), SMA(20), SMA(50) internally
// Returns deviation of close from the weighted average

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct WeightedComposite {
    // weights for 4 components
    w: [f64; 4],
    // internal MAs — configurable type and period
    ma1: MovingAverageProvider,
    ma2: MovingAverageProvider,
    ma3: MovingAverageProvider,
    ma4: MovingAverageProvider,
    norm: bool, // if true, divide by sum(|w|)
    value: f64,
}

impl WeightedComposite {
    /// Default: SMA(5), SMA(10), SMA(20), SMA(50).
    pub fn new(w1: f64, w2: f64, w3: f64, w4: f64, normalize: bool) -> Self {
        Self::with_ma(
            w1, w2, w3, w4, normalize,
            MovingAverageType::SMA,
            5, 10, 20, 50,
        )
    }

    /// Richer ctor: shared MA type + configurable periods for all 4 legs.
    ///
    /// Old defaults: `ma_type=SMA`, `p1=5, p2=10, p3=20, p4=50`.
    pub fn with_ma(
        w1: f64,
        w2: f64,
        w3: f64,
        w4: f64,
        normalize: bool,
        ma_type: MovingAverageType,
        p1: usize,
        p2: usize,
        p3: usize,
        p4: usize,
    ) -> Self {
        Self {
            w: [w1, w2, w3, w4],
            ma1: MovingAverageProvider::new(ma_type, p1),
            ma2: MovingAverageProvider::new(ma_type, p2),
            ma3: MovingAverageProvider::new(ma_type, p3),
            ma4: MovingAverageProvider::new(ma_type, p4),
            norm: normalize,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.ma1.reset();
        self.ma2.reset();
        self.ma3.reset();
        self.ma4.reset();
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ma4.is_ready() // longest period determines readiness
    }

    /// Legacy method - kept for compatibility but no longer used internally
    #[inline]
    pub fn update_inputs(&mut self, _i1: f64, _i2: f64, _i3: f64, _i4: f64) {
        // No-op: MAs computed internally
    }

    pub fn update_bar(
        &mut self,
        _open: f64,
        _high: f64,
        _low: f64,
        close: f64,
        _volume: f64,
    ) -> f64 {
        // Update all MAs (all use close price via the 0,0,0,close,0 convention)
        self.ma1.update_bar(0.0, 0.0, 0.0, close, 0.0);
        self.ma2.update_bar(0.0, 0.0, 0.0, close, 0.0);
        self.ma3.update_bar(0.0, 0.0, 0.0, close, 0.0);
        self.ma4.update_bar(0.0, 0.0, 0.0, close, 0.0);

        if self.is_ready() {
            let inputs = [
                self.ma1.value().main(),
                self.ma2.value().main(),
                self.ma3.value().main(),
                self.ma4.value().main(),
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
        assert!(!wc.is_ready()); // Not ready until MAs filled
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

    #[test]
    fn test_weighted_composite_with_ma_ema() {
        // Richer ctor: EMA with non-default periods
        let mut wc = WeightedComposite::with_ma(
            1.0, 1.0, 1.0, 1.0, true,
            MovingAverageType::EMA,
            3, 7, 14, 28,
        );
        assert!(!wc.is_ready());
        for i in 0..40 {
            let price = 100.0 + i as f64 * 0.5;
            wc.update_bar(price, price + 0.5, price - 0.5, price, 1000.0);
        }
        assert!(wc.is_ready());
        let v = wc.value().main();
        assert!(v.is_finite(), "EMA weighted composite must be finite, got {}", v);
    }
}
