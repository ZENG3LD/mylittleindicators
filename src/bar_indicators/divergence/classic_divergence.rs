use crate::bar_indicators::divergence::divergence::{DivergenceDetector, DivergenceType};
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct ClassicDivergence {
    detector: DivergenceDetector,
    prices: ArrayVec<f64, 512>,
    lookback: usize,
    value: f64,
}

impl ClassicDivergence {
    pub fn new(lookback: usize) -> Self {
        Self {
            detector: DivergenceDetector::new(),
            prices: ArrayVec::new(),
            lookback: lookback.max(5),
            value: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.detector = DivergenceDetector::new();
        self.prices.clear();
        self.value = 0.0;
    }

    pub fn is_ready(&self) -> bool {
        self.prices.len() >= self.lookback
    }

    pub fn value(&self) -> IndicatorValue {
        let signal = if self.value > 0.0 { 1i8 } else if self.value < 0.0 { -1i8 } else { 0i8 };
        IndicatorValue::Signal(signal)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let price = c;

        if self.prices.len() >= 512 {
            self.prices.remove(0);
        }
        self.prices.push(price);

        // Simple momentum-based divergence using price rate of change
        if self.prices.len() >= self.lookback * 2 {
            let len = self.prices.len();
            let price1 = self.prices[len - self.lookback];
            let price2 = self.prices[len - 1];

            // Calculate simple momentum over two periods
            let mom1 = if len >= self.lookback * 2 {
                price1 - self.prices[len - self.lookback * 2]
            } else {
                0.0
            };
            let mom2 = price2 - price1;

            // Detect divergence between price direction and momentum
            let dtype = DivergenceDetector::detect(price1, price2, mom1, mom2);

            self.value = match dtype {
                DivergenceType::Bullish => 1.0,
                DivergenceType::Bearish => -1.0,
                DivergenceType::HiddenBullish => 0.5,
                DivergenceType::HiddenBearish => -0.5,
                DivergenceType::None => 0.0,
            };
        }

        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classic_divergence_creation() {
        let div = ClassicDivergence::new(10);
        assert!(!div.is_ready());
        assert_eq!(div.value().main(), 0.0);
    }

    #[test]
    fn test_classic_divergence_warmup() {
        let mut div = ClassicDivergence::new(10);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            div.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(div.is_ready());
    }

    #[test]
    fn test_classic_divergence_values_range() {
        let mut div = ClassicDivergence::new(10);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = div.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= -1.0 && value <= 1.0, "Value should be in [-1, 1]");
        }
    }

    #[test]
    fn test_classic_divergence_reset() {
        let mut div = ClassicDivergence::new(10);
        for i in 0..30 {
            div.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        div.reset();
        assert!(!div.is_ready());
        assert_eq!(div.value().main(), 0.0);
    }
}
