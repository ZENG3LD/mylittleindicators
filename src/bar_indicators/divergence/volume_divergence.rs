use crate::bar_indicators::divergence::divergence::DivergenceDetector;
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct VolumeDivergence {
    detector: DivergenceDetector,
    prices: ArrayVec<f64, 512>,
    volumes: ArrayVec<f64, 512>,
    lookback: usize,
    value: f64,
}

impl VolumeDivergence {
    pub fn new(lookback: usize) -> Self {
        Self {
            detector: DivergenceDetector::new(),
            prices: ArrayVec::new(),
            volumes: ArrayVec::new(),
            lookback: lookback.max(5),
            value: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.detector = DivergenceDetector::new();
        self.prices.clear();
        self.volumes.clear();
        self.value = 0.0;
    }

    pub fn is_ready(&self) -> bool {
        self.prices.len() >= self.lookback
    }

    pub fn value(&self) -> IndicatorValue {
        let signal = if self.value > 0.0 { 1i8 } else if self.value < 0.0 { -1i8 } else { 0i8 };
        IndicatorValue::Signal(signal)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, v: f64) -> f64 {
        let price = c;
        let volume = v;

        if self.prices.len() >= 512 {
            self.prices.remove(0);
        }
        self.prices.push(price);

        if self.volumes.len() >= 512 {
            self.volumes.remove(0);
        }
        self.volumes.push(volume);

        if self.prices.len() >= self.lookback {
            let len = self.prices.len();

            if len >= self.lookback * 2 {
                let price1 = self.prices[len - self.lookback];
                let price2 = self.prices[len - 1];
                let vol1 = self.volumes[len - self.lookback];
                let vol2 = self.volumes[len - 1];

                // Classic bullish divergence: price lower, volume higher
                if price2 < price1 && vol2 > vol1 {
                    let strength = ((vol2 - vol1) / vol1.max(1.0)).min(1.0);
                    self.value = strength;
                }
                // Classic bearish divergence: price higher, volume lower
                else if price2 > price1 && vol2 < vol1 {
                    let strength = ((vol1 - vol2) / vol1.max(1.0)).min(1.0);
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
    fn test_volume_divergence_creation() {
        let div = VolumeDivergence::new(10);
        assert!(!div.is_ready());
        assert_eq!(div.value().main(), 0.0);
    }

    #[test]
    fn test_volume_divergence_warmup() {
        let mut div = VolumeDivergence::new(10);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            div.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(div.is_ready());
    }

    #[test]
    fn test_volume_divergence_values_finite() {
        let mut div = VolumeDivergence::new(10);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = div.update_bar(price, price + 1.0, price - 1.0, price, 1000.0 + i as f64 * 10.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_volume_divergence_reset() {
        let mut div = VolumeDivergence::new(10);
        for i in 0..20 {
            div.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        div.reset();
        assert!(!div.is_ready());
        assert_eq!(div.value().main(), 0.0);
    }
}
