use crate::bar_indicators::divergence::divergence::DivergenceDetector;
use crate::bar_indicators::momentum::obv::Obv;
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct ObvDivergence {
    detector: DivergenceDetector,
    obv: Obv,
    prices: ArrayVec<f64, 512>,
    obv_values: ArrayVec<f64, 512>,
    lookback: usize,
    value: f64,
}

impl ObvDivergence {
    pub fn new(lookback: usize) -> Self {
        Self {
            detector: DivergenceDetector::new(),
            obv: Obv::new(),
            prices: ArrayVec::new(),
            obv_values: ArrayVec::new(),
            lookback: lookback.max(5),
            value: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.detector = DivergenceDetector::new();
        self.obv.reset();
        self.prices.clear();
        self.obv_values.clear();
        self.value = 0.0;
    }

    pub fn is_ready(&self) -> bool {
        self.obv.is_ready() && self.prices.len() >= self.lookback
    }

    pub fn value(&self) -> IndicatorValue {
        let signal = if self.value > 0.0 { 1i8 } else if self.value < 0.0 { -1i8 } else { 0i8 };
        IndicatorValue::Signal(signal)
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let obv_value = self.obv.update_bar(o, h, l, c, v);
        let price = c;

        if self.prices.len() >= 512 {
            self.prices.remove(0);
        }
        self.prices.push(price);

        if self.obv_values.len() >= 512 {
            self.obv_values.remove(0);
        }
        self.obv_values.push(obv_value);

        if self.prices.len() >= self.lookback {
            let len = self.prices.len();

            if len >= self.lookback * 2 {
                let price1 = self.prices[len - self.lookback];
                let price2 = self.prices[len - 1];
                let ind1 = self.obv_values[len - self.lookback];
                let ind2 = self.obv_values[len - 1];

                // Classic bullish divergence: price lower, OBV higher
                if price2 < price1 && ind2 > ind1 {
                    let strength = ((ind2 - ind1) / ind1.abs().max(1.0)).abs().min(1.0);
                    self.value = strength;
                }
                // Classic bearish divergence: price higher, OBV lower
                else if price2 > price1 && ind2 < ind1 {
                    let strength = ((ind2 - ind1) / ind1.abs().max(1.0)).abs().min(1.0);
                    self.value = -strength;
                } else {
                    self.value = 0.0;
                }
            }
        }

        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_obv_divergence_creation() {
        let div = ObvDivergence::new(10);
        assert!(!div.is_ready());
        assert_eq!(div.value().main(), 0.0);
    }

    #[test]
    fn test_obv_divergence_warmup() {
        let mut div = ObvDivergence::new(10);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            div.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(div.is_ready());
    }

    #[test]
    fn test_obv_divergence_values_finite() {
        let mut div = ObvDivergence::new(10);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = div.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_obv_divergence_reset() {
        let mut div = ObvDivergence::new(10);
        for i in 0..30 {
            div.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        div.reset();
        assert!(!div.is_ready());
        assert_eq!(div.value().main(), 0.0);
    }
}
