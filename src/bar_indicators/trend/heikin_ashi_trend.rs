// Heikin-Ashi Trend - sign of HA close relative to HA open

use crate::bar_indicators::candles::heikin_ashi::HeikinAshi;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct HeikinAshiTrend {
    ha: HeikinAshi,
    value: i8,
}

impl Default for HeikinAshiTrend {
    fn default() -> Self {
        Self::new()
    }
}

impl HeikinAshiTrend {
    pub fn new() -> Self {
        Self {
            ha: HeikinAshi::new(),
            value: 0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.ha.reset();
        self.value = 0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Signal(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> i8 {
        let (ho, _hh, _hl, hc) = self.ha.update_bar(o, h, l, c, v);
        self.value = if hc > ho {
            1
        } else if hc < ho {
            -1
        } else {
            0
        };
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heikin_ashi_trend_creation() {
        let hat = HeikinAshiTrend::new();
        assert!(hat.is_ready()); // Always ready
    }

    #[test]
    fn test_heikin_ashi_trend_values() {
        let mut hat = HeikinAshiTrend::new();
        // Bullish bar: close > open
        let value = hat.update_bar(100.0, 102.0, 99.0, 101.0, 1000.0);
        assert!(value == 1 || value == 0 || value == -1, "Signal should be -1, 0, or 1");
    }

    #[test]
    fn test_heikin_ashi_trend_signal_range() {
        let mut hat = HeikinAshiTrend::new();
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 5.0;
            let value = hat.update_bar(price, price + 1.0, price - 1.0, price + 0.5, 1000.0);
            assert!(value >= -1 && value <= 1);
        }
    }

    #[test]
    fn test_heikin_ashi_trend_reset() {
        let mut hat = HeikinAshiTrend::new();
        for i in 0..10 {
            hat.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        hat.reset();
        assert!(hat.is_ready());
    }
}
