// High-performance Vertical Horizontal Filter (VHF)
// (c) 2024

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Clone)]
pub struct Vhf {
    period: usize,
    buffer: ArrayVec<f64, 512>,
    filled: bool,
    value: f64,
}

impl Vhf {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            buffer: ArrayVec::new(),
            filled: false,
            value: 0.0,
        }
    }
    /// Обновить VHF новым баром (используется close)
    pub fn update_bar(&mut self, _open: f64, _high: f64, _low: f64, close: f64, _volume: f64) -> f64 {
        if self.buffer.len() < self.period {
            self.buffer.push(close);
        } else {
            self.buffer.remove(0);
            self.buffer.push(close);
            self.filled = true;
        }
        if self.buffer.len() < self.period {
            self.value = 0.0;
            return self.value;
        }
        let (min, max) = self.buffer.iter().copied()
            .fold((f64::INFINITY, f64::NEG_INFINITY),
                  |(min, max), val| (min.min(val), max.max(val)));
        let mut sum = 0.0;
        for i in 1..self.period {
            sum += (self.buffer[i] - self.buffer[i - 1]).abs();
        }
        self.value = if sum.abs() < 1e-12 { 0.0 } else { (max - min) / sum };
        self.value
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn reset(&mut self) {
        self.buffer.clear();
        self.filled = false;
        self.value = 0.0;
    }

    pub fn period(&self) -> usize {
        self.period
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vhf_creation() {
        let vhf = Vhf::new(14);
        assert!(!vhf.is_ready());
        assert_eq!(vhf.value().main(), 0.0);
        assert_eq!(vhf.period(), 14);
    }

    #[test]
    fn test_vhf_trending() {
        let mut vhf = Vhf::new(14);
        for i in 1..=30 {
            let price = 100.0 + i as f64 * 2.0;
            vhf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(vhf.is_ready());
        assert!(vhf.value().main() > 0.0, "VHF should be > 0 in trending market, got {}", vhf.value().main());
    }

    #[test]
    fn test_vhf_finite_values() {
        let mut vhf = Vhf::new(14);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = vhf.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "VHF should always be finite");
            assert!(value >= 0.0, "VHF should be non-negative, got {}", value);
        }
    }

    #[test]
    fn test_vhf_reset() {
        let mut vhf = Vhf::new(14);
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
