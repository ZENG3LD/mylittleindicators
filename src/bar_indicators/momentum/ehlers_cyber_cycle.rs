// Ehlers Cyber Cycle - Isolates market cycle component from trend
//
// Formula:
// smooth = (price + 2*price[1] + 2*price[2] + price[3]) / 6
// cycle = (1 - 0.5*alpha)^2 * (smooth - 2*smooth[1] + smooth[2])
//         + 2*(1 - alpha) * cycle[1]
//         - (1 - alpha)^2 * cycle[2]

use arrayvec::ArrayVec;
use crate::bar_indicators::indicator_value::IndicatorValue;


#[derive(Clone)]
pub struct EhlersCyberCycle {
    alpha: f64,
    coeff1: f64,  // (1 - 0.5*alpha)^2
    coeff2: f64,  // 2*(1 - alpha)
    coeff3: f64,  // (1 - alpha)^2

    smooth_history: ArrayVec<f64, 3>,  // smooth[0..2]
    cycle_history: ArrayVec<f64, 2>,   // cycle[0..1]
    price_history: ArrayVec<f64, 4>,   // price[0..3]

    value: f64,
}

impl EhlersCyberCycle {
    pub fn new(alpha: f64) -> Self {
        let alpha = alpha.clamp(0.01, 0.99);
        let one_minus_alpha = 1.0 - alpha;
        let one_minus_half_alpha = 1.0 - 0.5 * alpha;

        let coeff1 = one_minus_half_alpha * one_minus_half_alpha;
        let coeff2 = 2.0 * one_minus_alpha;
        let coeff3 = one_minus_alpha * one_minus_alpha;

        Self {
            alpha,
            coeff1,
            coeff2,
            coeff3,
            smooth_history: ArrayVec::new(),
            cycle_history: ArrayVec::new(),
            price_history: ArrayVec::new(),
            value: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.smooth_history.clear();
        self.cycle_history.clear();
        self.price_history.clear();
        self.value = 0.0;
    }

    pub fn is_ready(&self) -> bool {
        self.price_history.len() >= 4
    }

    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Single(self.value)
    }

    pub fn update_bar(&mut self, _o: f64, h: f64, l: f64, _c: f64, _v: f64) -> f64 {
        // Use HL2 (high-low average) as price source
        let price = (h + l) / 2.0;

        // Add to price history
        if self.price_history.len() >= 4 {
            self.price_history.remove(0);
        }
        self.price_history.push(price);

        // Need at least 4 prices to calculate smooth
        if self.price_history.len() < 4 {
            return self.value;
        }

        // Calculate smooth: (price + 2*price[1] + 2*price[2] + price[3]) / 6
        let smooth = (self.price_history[3]
                    + 2.0 * self.price_history[2]
                    + 2.0 * self.price_history[1]
                    + self.price_history[0]) / 6.0;

        // Add to smooth history
        if self.smooth_history.len() >= 3 {
            self.smooth_history.remove(0);
        }
        self.smooth_history.push(smooth);

        // Need at least 3 smooth values to calculate cycle
        if self.smooth_history.len() < 3 {
            return self.value;
        }

        // Calculate cycle
        let smooth_diff = self.smooth_history[2] - 2.0 * self.smooth_history[1] + self.smooth_history[0];
        let mut cycle = self.coeff1 * smooth_diff;

        // Add previous cycle terms if available
        if !self.cycle_history.is_empty() {
            cycle += self.coeff2 * self.cycle_history[self.cycle_history.len() - 1];
        }

        if self.cycle_history.len() >= 2 {
            cycle -= self.coeff3 * self.cycle_history[self.cycle_history.len() - 2];
        }

        // Add to cycle history
        if self.cycle_history.len() >= 2 {
            self.cycle_history.remove(0);
        }
        self.cycle_history.push(cycle);

        self.value = cycle;
        self.value
    }

    pub fn alpha(&self) -> f64 {
        self.alpha
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cyber_cycle_creation() {
        let cc = EhlersCyberCycle::new(0.07);
        assert!(!cc.is_ready());
        assert_eq!(cc.value().main(), 0.0);
        assert!((cc.alpha() - 0.07).abs() < 1e-10);
    }

    #[test]
    fn test_cyber_cycle_alpha_clamped() {
        let cc1 = EhlersCyberCycle::new(0.0);
        assert!((cc1.alpha() - 0.01).abs() < 1e-10); // clamped to 0.01

        let cc2 = EhlersCyberCycle::new(1.5);
        assert!((cc2.alpha() - 0.99).abs() < 1e-10); // clamped to 0.99
    }

    #[test]
    fn test_cyber_cycle_basic() {
        let mut cc = EhlersCyberCycle::new(0.07);
        for i in 1..=30 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            cc.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
        }
        assert!(cc.is_ready());
        assert!(cc.value().main().is_finite());
    }

    #[test]
    fn test_cyber_cycle_reset() {
        let mut cc = EhlersCyberCycle::new(0.07);
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            cc.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
        }
        assert!(cc.is_ready());
        cc.reset();
        assert!(!cc.is_ready());
        assert_eq!(cc.value().main(), 0.0);
    }

    #[test]
    fn test_cyber_cycle_finite_values() {
        let mut cc = EhlersCyberCycle::new(0.07);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            let value = cc.update_bar(price, price + 2.0, price - 2.0, price, 1000.0);
            assert!(value.is_finite(), "Cyber Cycle should always be finite");
        }
    }
}
