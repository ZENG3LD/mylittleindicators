// Williams Accumulation/Distribution (WAD)

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct WilliamsAd {
    prev_close: f64,
    initialized: bool,
    value: f64,
}

impl Default for WilliamsAd {
    fn default() -> Self {
        Self::new()
    }
}

impl WilliamsAd {
    pub fn new() -> Self {
        Self {
            prev_close: 0.0,
            initialized: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.prev_close = 0.0;
        self.initialized = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.initialized
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, c: f64, _v: f64) -> f64 {
        if !self.initialized {
            self.prev_close = c;
            self.initialized = true;
            return self.value;
        }
        let trh = h.max(self.prev_close);
        let trl = l.min(self.prev_close);
        let ad = if c > self.prev_close {
            c - trl
        } else if c < self.prev_close {
            c - trh
        } else {
            0.0
        };
        self.prev_close = c;
        self.value += ad;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_williams_ad_creation() {
        let wad = WilliamsAd::new();
        assert!(!wad.is_ready());
        assert_eq!(wad.value().main(), 0.0);
    }

    #[test]
    fn test_williams_ad_warmup() {
        let mut wad = WilliamsAd::new();
        for i in 0..10 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            wad.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(wad.is_ready());
    }

    #[test]
    fn test_williams_ad_values_finite() {
        let mut wad = WilliamsAd::new();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = wad.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_williams_ad_reset() {
        let mut wad = WilliamsAd::new();
        for i in 0..10 {
            wad.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        wad.reset();
        assert!(!wad.is_ready());
        assert_eq!(wad.value().main(), 0.0);
    }
}
