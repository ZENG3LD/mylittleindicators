// Smoothed Ultimate Oscillator: EMA of existing UltimateOscillator

use crate::bar_indicators::average::{MovingAverageProvider, MovingAverageType};
use crate::bar_indicators::momentum::ultimate_oscillator::UltimateOscillator;
use crate::bar_indicators::indicator_value::IndicatorValue;


#[derive(Clone)]
pub struct UltimateOscillatorSmooth {
    uo: UltimateOscillator,
    ema: MovingAverageProvider,
    value: f64,
}

impl UltimateOscillatorSmooth {
    pub fn new(p1: usize, p2: usize, p3: usize, smooth: usize) -> Self {
        let uo = UltimateOscillator::with_periods(p1, p2, p3);
        // ensure constructed; nothing else needed
        Self {
            uo,
            ema: MovingAverageProvider::new(MovingAverageType::EMA, smooth.max(1)),
            value: 50.0,
        }
    }
    pub fn reset(&mut self) {
        self.uo.reset();
        self.ema.reset();
        self.value = 50.0;
    }
    pub fn is_ready(&self) -> bool {
        self.ema.is_ready()
    }
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }
    pub fn update_bar(&mut self, o: f64, h: f64, l: f64, c: f64, v: f64) -> f64 {
        let u = self.uo.update_bar(o, h, l, c, v);
        self.value = self.ema.update_bar(0.0, 0.0, 0.0, u, 0.0);
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uo_smooth_creation() {
        let uo = UltimateOscillatorSmooth::new(7, 14, 28, 9);
        assert!(!uo.is_ready());
        assert_eq!(uo.value().main(), 50.0);
    }

    #[test]
    fn test_uo_smooth_uptrend() {
        let mut uo = UltimateOscillatorSmooth::new(7, 14, 28, 9);
        for i in 1..=50 {
            let price = 100.0 + i as f64 * 2.0;
            uo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(uo.is_ready());
        assert!(uo.value().main() > 50.0, "UO Smooth should be > 50 in uptrend, got {}", uo.value().main());
    }

    #[test]
    fn test_uo_smooth_downtrend() {
        let mut uo = UltimateOscillatorSmooth::new(7, 14, 28, 9);
        for i in 1..=50 {
            let price = 200.0 - i as f64 * 2.0;
            uo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(uo.is_ready());
        assert!(uo.value().main() < 50.0, "UO Smooth should be < 50 in downtrend, got {}", uo.value().main());
    }

    #[test]
    fn test_uo_smooth_finite() {
        let mut uo = UltimateOscillatorSmooth::new(7, 14, 28, 9);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 20.0;
            let value = uo.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "UO Smooth should always be finite");
        }
    }

    #[test]
    fn test_uo_smooth_reset() {
        let mut uo = UltimateOscillatorSmooth::new(7, 14, 28, 9);
        for i in 1..=50 {
            let price = 100.0 + i as f64;
            uo.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(uo.is_ready());
        uo.reset();
        assert!(!uo.is_ready());
        assert_eq!(uo.value().main(), 50.0);
    }
}
