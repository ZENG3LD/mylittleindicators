use crate::bar_indicators::divergence::divergence::DivergenceDetector;
use crate::bar_indicators::momentum::rsi::Rsi;
use crate::bar_indicators::momentum::cci::Cci;
use crate::bar_indicators::momentum::macd::Macd;
use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct MultiDivergence {
    detector: DivergenceDetector,
    rsi: Rsi,
    cci: Cci,
    macd: Macd,
    prices: ArrayVec<f64, 512>,
    rsi_values: ArrayVec<f64, 512>,
    cci_values: ArrayVec<f64, 512>,
    macd_values: ArrayVec<f64, 512>,
    lookback: usize,
    value: f64,
}

impl MultiDivergence {
    pub fn new(period: usize, lookback: usize) -> Self {
        Self {
            detector: DivergenceDetector::new(),
            rsi: Rsi::new(period.max(1)),
            cci: Cci::new(period.max(1), 0.015, None),
            macd: Macd::new(12, 26),
            prices: ArrayVec::new(),
            rsi_values: ArrayVec::new(),
            cci_values: ArrayVec::new(),
            macd_values: ArrayVec::new(),
            lookback: lookback.max(5),
            value: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.detector = DivergenceDetector::new();
        self.rsi.reset();
        self.cci.reset();
        self.macd.reset();
        self.prices.clear();
        self.rsi_values.clear();
        self.cci_values.clear();
        self.macd_values.clear();
        self.value = 0.0;
    }

    pub fn is_ready(&self) -> bool {
        self.rsi.is_ready() && self.cci.is_ready() && self.macd.is_ready()
            && self.prices.len() >= self.lookback
    }

    pub fn value(&self) -> IndicatorValue {
        let signal = if self.value > 0.0 { 1i8 } else if self.value < 0.0 { -1i8 } else { 0i8 };
        IndicatorValue::Signal(signal)
    }

    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let rsi_value = self.rsi.update_bar(o, h, l, c, v);
        let cci_value = self.cci.update_bar(h, l, c, v);
        let macd_value = self.macd.update_bar(o, h, l, c, v);
        let price = c;

        if self.prices.len() >= 512 {
            self.prices.remove(0);
        }
        self.prices.push(price);

        if self.rsi_values.len() >= 512 {
            self.rsi_values.remove(0);
        }
        self.rsi_values.push(rsi_value);

        if self.cci_values.len() >= 512 {
            self.cci_values.remove(0);
        }
        self.cci_values.push(cci_value);

        if self.macd_values.len() >= 512 {
            self.macd_values.remove(0);
        }
        self.macd_values.push(macd_value);

        if self.prices.len() >= self.lookback * 2 {
            let len = self.prices.len();
            let price1 = self.prices[len - self.lookback];
            let price2 = self.prices[len - 1];

            let rsi1 = self.rsi_values[len - self.lookback];
            let rsi2 = self.rsi_values[len - 1];
            let cci1 = self.cci_values[len - self.lookback];
            let cci2 = self.cci_values[len - 1];
            let macd1 = self.macd_values[len - self.lookback];
            let macd2 = self.macd_values[len - 1];

            // Count bullish and bearish signals
            let mut bullish_count = 0;
            let mut bearish_count = 0;

            // RSI divergence
            if price2 < price1 && rsi2 > rsi1 { bullish_count += 1; }
            else if price2 > price1 && rsi2 < rsi1 { bearish_count += 1; }

            // CCI divergence
            if price2 < price1 && cci2 > cci1 { bullish_count += 1; }
            else if price2 > price1 && cci2 < cci1 { bearish_count += 1; }

            // MACD divergence
            if price2 < price1 && macd2 > macd1 { bullish_count += 1; }
            else if price2 > price1 && macd2 < macd1 { bearish_count += 1; }

            // Aggregate signal (stronger if multiple indicators confirm)
            if bullish_count > bearish_count {
                self.value = (bullish_count as f64) / 3.0;
            } else if bearish_count > bullish_count {
                self.value = -(bearish_count as f64) / 3.0;
            } else {
                self.value = 0.0;
            }
        }

        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_divergence_creation() {
        let div = MultiDivergence::new(14, 10);
        assert!(!div.is_ready());
        assert_eq!(div.value().main(), 0.0);
    }

    #[test]
    fn test_multi_divergence_warmup() {
        let mut div = MultiDivergence::new(14, 10);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            div.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(div.is_ready());
    }

    #[test]
    fn test_multi_divergence_values_range() {
        let mut div = MultiDivergence::new(14, 10);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = div.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= -1.0 && value <= 1.0, "Value should be in [-1, 1]");
        }
    }

    #[test]
    fn test_multi_divergence_reset() {
        let mut div = MultiDivergence::new(14, 10);
        for i in 0..60 {
            div.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        div.reset();
        assert!(!div.is_ready());
        assert_eq!(div.value().main(), 0.0);
    }
}
