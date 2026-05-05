// Volume Oscillator - difference between fast and slow volume averages

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct VolumeOscillator {
    fast: MovingAverageProvider,
    slow: MovingAverageProvider,
    value: f64,
}

impl VolumeOscillator {
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self {
            fast: MovingAverageProvider::new(MovingAverageType::EMA, fast_period.max(1)),
            slow: MovingAverageProvider::new(MovingAverageType::EMA, slow_period.max(2)),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.fast.reset();
        self.slow.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.fast.is_ready() && self.slow.is_ready()
    }
    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, _c: f64, v: f64) -> f64 {
        let f = self.fast.update_bar(0.0, 0.0, 0.0, v, 0.0);
        let s = self.slow.update_bar(0.0, 0.0, 0.0, v, 0.0);
        self.value = f - s;
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_oscillator_creation() {
        let vo = VolumeOscillator::new(5, 10);
        assert!(!vo.is_ready());
        assert_eq!(vo.value().main(), 0.0);
    }

    #[test]
    fn test_volume_oscillator_warmup() {
        let mut vo = VolumeOscillator::new(5, 10);
        for i in 0..15 {
            let volume = 1000.0 + (i as f64 * 0.1).sin() * 100.0;
            vo.update_bar(100.0, 101.0, 99.0, 100.0, volume);
        }
        assert!(vo.is_ready());
    }

    #[test]
    fn test_volume_oscillator_values() {
        let mut vo = VolumeOscillator::new(5, 10);
        for i in 0..20 {
            let volume = 1000.0 + i as f64 * 50.0;
            let value = vo.update_bar(100.0, 101.0, 99.0, 100.0, volume);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_volume_oscillator_reset() {
        let mut vo = VolumeOscillator::new(5, 10);
        for i in 0..20 {
            vo.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0 + i as f64 * 10.0);
        }
        vo.reset();
        assert!(!vo.is_ready());
        assert_eq!(vo.value().main(), 0.0);
    }
}
