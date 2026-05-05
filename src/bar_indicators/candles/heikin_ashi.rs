use crate::bar_indicators::indicator_value::IndicatorValue;

/// Heikin-Ashi bar transformer
#[derive(Debug, Clone, Default)]
pub struct HeikinAshi {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    initialized: bool,
}

impl HeikinAshi {
    pub fn new() -> Self {
        Self::default()
    }

    /// Update with real OHLCV; returns HA (o,h,l,c)
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, _v: f64) -> (f64, f64, f64, f64) {
        let ha_close = (o + h + l + c) / 4.0;
        let ha_open = if !self.initialized {
            (o + c) / 2.0
        } else {
            (self.open + self.close) / 2.0
        };
        let ha_high = ha_close.max(ha_open).max(h);
        let ha_low = ha_close.min(ha_open).min(l);
        self.open = ha_open;
        self.high = ha_high;
        self.low = ha_low;
        self.close = ha_close;
        self.initialized = true;
        (self.open, self.high, self.low, self.close)
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Candle {
            open: self.open,
            high: self.high,
            low: self.low,
            close: self.close,
        }
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heikin_ashi_creation() {
        let ind = HeikinAshi::new();
        assert!(ind.is_ready());
        assert_eq!(ind.value(), IndicatorValue::Candle { open: 0.0, high: 0.0, low: 0.0, close: 0.0 });
    }

    #[test]
    fn test_heikin_ashi_update() {
        let mut ind = HeikinAshi::new();
        let (ha_o, ha_h, ha_l, ha_c) = ind.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0);
        assert!(ha_o.is_finite());
        assert!(ha_h.is_finite());
        assert!(ha_l.is_finite());
        assert!(ha_c.is_finite());
        assert!(ha_h >= ha_l);
    }

    #[test]
    fn test_heikin_ashi_multiple_updates() {
        let mut ind = HeikinAshi::new();
        for i in 0..10 {
            let price = 100.0 + i as f64;
            let (ha_o, ha_h, ha_l, ha_c) = ind.update_bar(price, price + 2.0, price - 2.0, price + 1.0, 1000.0);
            assert!(ha_h >= ha_l);
            assert!(ha_h >= ha_o && ha_h >= ha_c);
            assert!(ha_l <= ha_o && ha_l <= ha_c);
        }
    }

    #[test]
    fn test_heikin_ashi_reset() {
        let mut ind = HeikinAshi::new();
        ind.update_bar(100.0, 105.0, 95.0, 102.0, 1000.0);
        ind.reset();
        assert_eq!(ind.value(), IndicatorValue::Candle { open: 0.0, high: 0.0, low: 0.0, close: 0.0 });
    }
}
