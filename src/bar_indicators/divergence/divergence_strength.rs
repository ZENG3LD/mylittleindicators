use crate::bar_indicators::divergence::divergence::{DivergenceDetector, DivergenceType};
use crate::bar_indicators::momentum::rsi::Rsi;
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct DivergenceStrength {
    detector: DivergenceDetector,
    rsi: Rsi,
    prices: ArrayVec<f64, 512>,
    rsi_values: ArrayVec<f64, 512>,
    lookback: usize,
    value: f64,
}

impl DivergenceStrength {
    pub fn new(period: usize, lookback: usize) -> Self {
        Self {
            detector: DivergenceDetector::new(),
            rsi: Rsi::new(period.max(1)),
            prices: ArrayVec::new(),
            rsi_values: ArrayVec::new(),
            lookback: lookback.max(5),
            value: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.detector = DivergenceDetector::new();
        self.rsi.reset();
        self.prices.clear();
        self.rsi_values.clear();
        self.value = 0.0;
    }

    pub fn is_ready(&self) -> bool {
        self.rsi.is_ready() && self.prices.len() >= self.lookback
    }

    pub fn value(&self) -> IndicatorValue {
        let signal = if self.value > 0.0 { 1i8 } else if self.value < 0.0 { -1i8 } else { 0i8 };
        IndicatorValue::Signal(signal)
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let rsi_value = self.rsi.update_bar(o, h, l, c, v);
        let price = c;

        if self.prices.len() >= 512 {
            self.prices.remove(0);
        }
        self.prices.push(price);

        if self.rsi_values.len() >= 512 {
            self.rsi_values.remove(0);
        }
        self.rsi_values.push(rsi_value);

        if self.prices.len() >= self.lookback {
            let len = self.prices.len();

            if len >= self.lookback * 2 {
                let price1 = self.prices[len - self.lookback];
                let price2 = self.prices[len - 1];
                let ind1 = self.rsi_values[len - self.lookback];
                let ind2 = self.rsi_values[len - 1];

                let dtype = DivergenceDetector::detect(price1, price2, ind1, ind2);

                // Calculate strength based on magnitude of divergence
                let price_change = ((price2 - price1) / price1.abs().max(0.01)).abs();
                let ind_change = ((ind2 - ind1) / 100.0).abs();

                let strength = (price_change + ind_change) / 2.0;

                self.value = match dtype {
                    DivergenceType::Bullish => strength.min(1.0),
                    DivergenceType::Bearish => -strength.min(1.0),
                    DivergenceType::HiddenBullish => strength.min(1.0) * 0.7,
                    DivergenceType::HiddenBearish => -strength.min(1.0) * 0.7,
                    DivergenceType::None => 0.0,
                };
            }
        }

        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divergence_strength_creation() {
        let div = DivergenceStrength::new(14, 10);
        assert!(!div.is_ready());
        assert_eq!(div.value().main(), 0.0);
    }

    #[test]
    fn test_divergence_strength_warmup() {
        let mut div = DivergenceStrength::new(14, 10);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            div.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(div.is_ready());
    }

    #[test]
    fn test_divergence_strength_values_range() {
        let mut div = DivergenceStrength::new(14, 10);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = div.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= -1.0 && value <= 1.0, "Value should be in [-1, 1]");
        }
    }

    #[test]
    fn test_divergence_strength_reset() {
        let mut div = DivergenceStrength::new(14, 10);
        for i in 0..50 {
            div.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        div.reset();
        assert!(!div.is_ready());
        assert_eq!(div.value().main(), 0.0);
    }
}
