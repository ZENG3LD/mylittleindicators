// VhfMa: модифицированный VHF с MA в знаменателе (аналог Nautilus, но на ArrayVec и с нашими MA)
// (c) 2024

use arrayvec::ArrayVec;
use crate::bar_indicators::average::moving_average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct VhfMa {
    period: usize,
    buffer: ArrayVec<f64, 512>,
    filled: bool,
    value: f64,
    ma: MovingAverageProvider,
    prev_close: Option<f64>,
}

impl VhfMa {
    pub fn new(period: usize, ma_type: MovingAverageType) -> Self {
        Self {
            period,
            buffer: ArrayVec::new(),
            filled: false,
            value: 0.0,
            ma: MovingAverageProvider::new(ma_type, period),
            prev_close: None,
        }
    }
    /// Модифицированный VHF: знаменатель — MA от abs разностей между close
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> f64 {
        if self.buffer.len() < self.period {
            self.buffer.push(close);
        } else {
            self.buffer.remove(0);
            self.buffer.push(close);
            self.filled = true;
        }
        // Обновляем MA по модулю разности close
        if let Some(prev) = self.prev_close {
            let abs_diff = (close - prev).abs();
            self.ma.update_bar(0.0, 0.0, 0.0, abs_diff, 0.0);
        }
        self.prev_close = Some(close);
        if self.buffer.len() < self.period || !self.filled {
            self.value = 0.0;
            return self.value;
        }
        let max = self.buffer.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let min = self.buffer.iter().copied().fold(f64::INFINITY, f64::min);
        let ma_val = self.ma.value().main();
        if ma_val.abs() < 1e-12 {
            self.value = 0.0;
        } else {
            self.value = (max - min) / (self.period as f64 * ma_val);
        }
        self.value
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.filled && self.ma.is_ready()
    }
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.filled = false;
        self.value = 0.0;
        self.ma.reset();
        self.prev_close = None;
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vhf_ma_creation() {
        let vhf = VhfMa::new(14, MovingAverageType::EMA);
        assert!(!vhf.is_ready());
        assert_eq!(vhf.value().main(), 0.0);
        assert_eq!(vhf.period(), 14);
    }

    #[test]
    fn test_vhf_ma_trending() {
        let mut vhf = VhfMa::new(14, MovingAverageType::EMA);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            vhf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vhf.is_ready());
        assert!(vhf.value().main() > 0.0, "VHF-MA should be > 0 in trending market, got {}", vhf.value().main());
    }

    #[test]
    fn test_vhf_ma_finite_values() {
        let mut vhf = VhfMa::new(14, MovingAverageType::SMA);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = vhf.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "VHF-MA should always be finite");
            assert!(value >= 0.0, "VHF-MA should be non-negative, got {}", value);
        }
    }

    #[test]
    fn test_vhf_ma_reset() {
        let mut vhf = VhfMa::new(14, MovingAverageType::EMA);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            vhf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vhf.is_ready());
        vhf.reset();
        assert!(!vhf.is_ready());
        assert_eq!(vhf.value().main(), 0.0);
    }
}






















