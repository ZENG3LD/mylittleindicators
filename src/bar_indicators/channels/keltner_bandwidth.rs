// Keltner Bandwidth: (upper - lower) / middle

use crate::bar_indicators::average::moving_average::MovingAverageType;
use crate::bar_indicators::channels::keltner_channel::{KeltnerChannel, KeltnerMode};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct KeltnerBandwidth {
    kc: KeltnerChannel,
    value: f64,
}

impl KeltnerBandwidth {
    pub fn new(period: usize, mult: f64) -> Self {
        Self::with_ma_types(period, mult, MovingAverageType::SMA, MovingAverageType::RMA)
    }

    /// Create with explicit center MA type and ATR smoothing MA type.
    ///
    /// Defaults used by `new`: center=`SMA`, atr=`RMA` (Wilder).
    pub fn with_ma_types(
        period: usize,
        mult: f64,
        center_ma_type: MovingAverageType,
        atr_ma_type: MovingAverageType,
    ) -> Self {
        Self {
            kc: KeltnerChannel::new(
                period.max(2),
                mult.max(0.1),
                KeltnerMode::Classic,
                center_ma_type,
                atr_ma_type,
            ),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.kc.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.kc.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let (upper, middle, lower) = self.kc.update_bar(o, h, l, c, v);
        let width = upper - lower;
        self.value = if middle.abs() > 1e-12 {
            width / middle.abs()
        } else {
            0.0
        };
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_ma_types_non_default() {
        let mut kb = KeltnerBandwidth::with_ma_types(20, 2.0, MovingAverageType::EMA, MovingAverageType::EMA);
        assert!(!kb.is_ready());
        for i in 0..30 {
            let p = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let v = kb.update_bar(p, p + 1.0, p - 1.0, p, 1000.0);
            assert!(v >= 0.0);
        }
        assert!(kb.is_ready());
    }

    #[test]
    fn test_keltner_bandwidth_creation() {
        let kb = KeltnerBandwidth::new(20, 2.0);
        assert!(!kb.is_ready());
        assert_eq!(kb.value().main(), 0.0);
    }

    #[test]
    fn test_keltner_bandwidth_warmup() {
        let mut kb = KeltnerBandwidth::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(kb.is_ready());
    }

    #[test]
    fn test_keltner_bandwidth_positive() {
        let mut kb = KeltnerBandwidth::new(20, 2.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kb.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value >= 0.0, "Bandwidth should be non-negative");
        }
    }

    #[test]
    fn test_keltner_bandwidth_reset() {
        let mut kb = KeltnerBandwidth::new(20, 2.0);
        for i in 0..25 {
            kb.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        kb.reset();
        assert!(!kb.is_ready());
        assert_eq!(kb.value().main(), 0.0);
    }
}
