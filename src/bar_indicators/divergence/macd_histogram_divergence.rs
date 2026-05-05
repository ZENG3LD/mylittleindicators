use crate::bar_indicators::divergence::divergence::DivergenceDetector;
use crate::bar_indicators::momentum::macd_histogram::MacdHistogram;
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct MacdHistogramDivergence {
    detector: DivergenceDetector,
    macd_hist: MacdHistogram,
    prices: ArrayVec<f64, 512>,
    hist_values: ArrayVec<f64, 512>,
    lookback: usize,
    value: f64,
}

impl MacdHistogramDivergence {
    pub fn new(fast: usize, slow: usize, signal: usize, lookback: usize) -> Self {
        Self {
            detector: DivergenceDetector::new(),
            macd_hist: MacdHistogram::new(fast.max(1), slow.max(2), signal.max(1)),
            prices: ArrayVec::new(),
            hist_values: ArrayVec::new(),
            lookback: lookback.max(5),
            value: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.detector = DivergenceDetector::new();
        self.macd_hist.reset();
        self.prices.clear();
        self.hist_values.clear();
        self.value = 0.0;
    }

    pub fn is_ready(&self) -> bool {
        self.macd_hist.is_ready() && self.prices.len() >= self.lookback
    }

    pub fn value(&self) -> IndicatorValue {
        let signal = if self.value > 0.0 { 1i8 } else if self.value < 0.0 { -1i8 } else { 0i8 };
        IndicatorValue::Signal(signal)
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let hist_value = self.macd_hist.update_bar(o, h, l, c, v);
        let price = c;

        if self.prices.len() >= 512 {
            self.prices.remove(0);
        }
        self.prices.push(price);

        if self.hist_values.len() >= 512 {
            self.hist_values.remove(0);
        }
        self.hist_values.push(hist_value);

        if self.prices.len() >= self.lookback {
            let len = self.prices.len();

            if len >= self.lookback * 2 {
                let price1 = self.prices[len - self.lookback];
                let price2 = self.prices[len - 1];
                let ind1 = self.hist_values[len - self.lookback];
                let ind2 = self.hist_values[len - 1];

                // Classic bullish divergence: price lower, histogram higher
                if price2 < price1 && ind2 > ind1 {
                    self.value = ((ind2 - ind1).abs()).min(1.0);
                }
                // Classic bearish divergence: price higher, histogram lower
                else if price2 > price1 && ind2 < ind1 {
                    self.value = -((ind2 - ind1).abs()).min(1.0);
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
    fn test_macd_histogram_divergence_creation() {
        let div = MacdHistogramDivergence::new(12, 26, 9, 10);
        assert!(!div.is_ready());
        assert_eq!(div.value().main(), 0.0);
    }

    #[test]
    fn test_macd_histogram_divergence_warmup() {
        let mut div = MacdHistogramDivergence::new(12, 26, 9, 10);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            div.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(div.is_ready());
    }

    #[test]
    fn test_macd_histogram_divergence_values_finite() {
        let mut div = MacdHistogramDivergence::new(12, 26, 9, 10);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = div.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_macd_histogram_divergence_reset() {
        let mut div = MacdHistogramDivergence::new(12, 26, 9, 10);
        for i in 0..60 {
            div.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        div.reset();
        assert!(!div.is_ready());
        assert_eq!(div.value().main(), 0.0);
    }
}
