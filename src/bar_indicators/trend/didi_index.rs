// Didi Index (Odir Aguiar): relationship of three EMAs

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct DidiIndex {
    ma_type: MovingAverageType,
    short_period: usize,
    mid_period: usize,
    long_period: usize,
    ema_short: MovingAverageProvider,
    ema_mid: MovingAverageProvider,
    ema_long: MovingAverageProvider,
    short_ratio: f64,  // Short/Mid ratio
    long_ratio: f64,   // Long/Mid ratio
}

impl DidiIndex {
    pub fn new(short_p: usize, mid_p: usize, long_p: usize) -> Self {
        Self::new_with_ma_type(short_p, mid_p, long_p, MovingAverageType::EMA)
    }

    pub fn new_default(short_p: usize, mid_p: usize, long_p: usize) -> Self {
        Self::new_with_ma_type(short_p, mid_p, long_p, MovingAverageType::EMA)
    }

    pub fn new_with_ma_type(short_p: usize, mid_p: usize, long_p: usize, ma_type: MovingAverageType) -> Self {
        let short = short_p.max(1);
        let mid = mid_p.max(2);
        let long = long_p.max(3);
        Self {
            ma_type,
            short_period: short,
            mid_period: mid,
            long_period: long,
            ema_short: MovingAverageProvider::new(ma_type, short),
            ema_mid: MovingAverageProvider::new(ma_type, mid),
            ema_long: MovingAverageProvider::new(ma_type, long),
            short_ratio: 0.0,
            long_ratio: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.ema_short = MovingAverageProvider::new(self.ma_type, self.short_period);
        self.ema_mid = MovingAverageProvider::new(self.ma_type, self.mid_period);
        self.ema_long = MovingAverageProvider::new(self.ma_type, self.long_period);
        self.short_ratio = 0.0;
        self.long_ratio = 0.0;
    }

    pub fn set_ma_type(&mut self, ma_type: MovingAverageType) {
        self.ma_type = ma_type;
        self.reset();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ema_long.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Double(self.short_ratio, self.long_ratio)
    }

    /// Returns the short ratio (Short/Mid)
    pub fn short_ratio(&self) -> f64 {
        self.short_ratio
    }

    /// Returns the long ratio (Long/Mid)
    pub fn long_ratio(&self) -> f64 {
        self.long_ratio
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        let s = self.ema_short.update_bar(0.0, 0.0, 0.0, c, 0.0);
        let m = self.ema_mid.update_bar(0.0, 0.0, 0.0, c, 0.0);
        let l = self.ema_long.update_bar(0.0, 0.0, 0.0, c, 0.0);
        // Classic Didi Index: ratios relative to mid line
        if m.abs() > 1e-12 {
            self.short_ratio = s / m;
            self.long_ratio = l / m;
        }
        self.short_ratio - self.long_ratio // Return spread for compatibility
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_didi_index_creation() {
        let didi = DidiIndex::new(3, 8, 20);
        assert!(!didi.is_ready());
        assert_eq!(didi.short_ratio(), 0.0);
        assert_eq!(didi.long_ratio(), 0.0);
    }

    #[test]
    fn test_didi_index_warmup() {
        let mut didi = DidiIndex::new(3, 8, 20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            didi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(didi.is_ready());
    }

    #[test]
    fn test_didi_index_values_finite() {
        let mut didi = DidiIndex::new(3, 8, 20);
        for i in 0..30 {
            let price = 100.0 + i as f64;
            let value = didi.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_didi_index_reset() {
        let mut didi = DidiIndex::new(3, 8, 20);
        for i in 0..30 {
            didi.update_bar(100.0 + i as f64, 105.0, 95.0, 101.0, 1000.0);
        }
        didi.reset();
        assert!(!didi.is_ready());
        assert_eq!(didi.short_ratio(), 0.0);
        assert_eq!(didi.long_ratio(), 0.0);
    }
}
