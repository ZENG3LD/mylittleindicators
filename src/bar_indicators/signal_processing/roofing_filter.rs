// Roofing Filter (Ehlers) — high-pass filter followed by SuperSmoother.
//
// From John Ehlers "Cycle Analytics for Traders":
//   Step 1: 2-bar HP filter with cutoff = hp_period
//     alpha1 = (cos(sqrt(2)*pi/hp_period) + sin(sqrt(2)*pi/hp_period) - 1) / cos(sqrt(2)*pi/hp_period)
//     HP[i] = (1-alpha1/2)^2 * (price - 2*price[i-1] + price[i-2])
//             + 2*(1-alpha1) * HP[i-1] - (1-alpha1)^2 * HP[i-2]
//
//   Step 2: SuperSmoother (2-pole Butterworth low-pass) with cutoff = lp_period
//     b = exp(-sqrt(2)*pi/lp_period)
//     c2 = 2*b*cos(sqrt(2)*pi/lp_period)
//     c3 = -b^2
//     c1 = 1 - c2 - c3
//     SS[i] = c1 * (HP[i] + HP[i-1]) / 2 + c2 * SS[i-1] + c3 * SS[i-2]
//
// Output: Single(roofing_filter_value)

use std::f64::consts::PI;

#[derive(Debug, Clone)]
pub struct RoofingFilter {
    // HP filter coefficients
    hp_c0: f64,  // (1 - alpha1/2)^2
    hp_c1: f64,  // 2*(1 - alpha1)
    hp_c2: f64,  // (1 - alpha1)^2
    // SuperSmoother coefficients
    ss_c1: f64,
    ss_c2: f64,
    ss_c3: f64,
    // State
    prev_price: [f64; 2],
    prev_hp: [f64; 2],
    prev_ss: [f64; 2],
    value: f64,
    count: usize,
    ready: bool,
}

impl RoofingFilter {
    /// Create RoofingFilter with HP cutoff period and SuperSmoother LP period.
    /// Typical: hp_period=48, lp_period=10
    pub fn new_with_periods(hp_period: f64, lp_period: f64) -> Self {
        let hp_p = hp_period.max(2.0);
        let lp_p = lp_period.max(2.0);

        // HP filter alpha (Butterworth 2nd-order)
        let sq2 = 2.0_f64.sqrt();
        let alpha1_cos = (sq2 * PI / hp_p).cos();
        let alpha1_sin = (sq2 * PI / hp_p).sin();
        let alpha1 = (alpha1_cos + alpha1_sin - 1.0) / alpha1_cos;

        let hp_c0 = (1.0 - alpha1 / 2.0).powi(2);
        let hp_c1 = 2.0 * (1.0 - alpha1);
        let hp_c2 = (1.0 - alpha1).powi(2);

        // SuperSmoother coefficients
        let b = (-sq2 * PI / lp_p).exp();
        let ss_c2 = 2.0 * b * (sq2 * PI / lp_p).cos();
        let ss_c3 = -b * b;
        let ss_c1 = 1.0 - ss_c2 - ss_c3;

        Self {
            hp_c0,
            hp_c1,
            hp_c2,
            ss_c1,
            ss_c2,
            ss_c3,
            prev_price: [0.0; 2],
            prev_hp: [0.0; 2],
            prev_ss: [0.0; 2],
            value: 0.0,
            count: 0,
            ready: false,
        }
    }

    /// Backward-compatible constructor using alpha values.
    /// hp_alpha ≈ 2/hp_period, lp_alpha ≈ 2/lp_period.
    pub fn new(hp_alpha: f64, lp_alpha: f64) -> Self {
        let hp_a = hp_alpha.clamp(0.001, 1.0);
        let lp_a = lp_alpha.clamp(0.001, 1.0);
        let hp_period = 2.0 / hp_a;
        let lp_period = 2.0 / lp_a;
        Self::new_with_periods(hp_period, lp_period)
    }

    #[inline]
    pub fn reset(&mut self) {
        self.prev_price = [0.0; 2];
        self.prev_hp = [0.0; 2];
        self.prev_ss = [0.0; 2];
        self.value = 0.0;
        self.count = 0;
        self.ready = false;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ready
    }

    #[inline]
    pub fn value(&self) -> crate::bar_indicators::indicator_value::IndicatorValue {
        crate::bar_indicators::indicator_value::IndicatorValue::Single(self.value)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        self.count += 1;

        if self.count < 3 {
            // Seed state
            if self.count == 1 {
                self.prev_price[1] = c;
            } else {
                self.prev_price[0] = self.prev_price[1];
                self.prev_price[1] = c;
            }
            return self.value;
        }

        // Step 1: HP filter
        let hp = self.hp_c0 * (c - 2.0 * self.prev_price[1] + self.prev_price[0])
            + self.hp_c1 * self.prev_hp[1]
            - self.hp_c2 * self.prev_hp[0];

        // Step 2: SuperSmoother on HP output
        let ss = self.ss_c1 * (hp + self.prev_hp[1]) / 2.0
            + self.ss_c2 * self.prev_ss[1]
            + self.ss_c3 * self.prev_ss[0];

        self.value = ss;
        self.ready = true;

        // Shift state
        self.prev_price[0] = self.prev_price[1];
        self.prev_price[1] = c;
        self.prev_hp[0] = self.prev_hp[1];
        self.prev_hp[1] = hp;
        self.prev_ss[0] = self.prev_ss[1];
        self.prev_ss[1] = ss;

        self.value
    }

    pub fn hp_alpha(&self) -> f64 {
        // Return approximate alpha for backward compat
        2.0 / (2.0 * PI / (PI / (2.0_f64.sqrt() * (self.hp_c0.sqrt().acos()))))
    }

    pub fn lp_alpha(&self) -> f64 {
        2.0 / ((-self.ss_c3).sqrt().ln() / (-2.0_f64.sqrt() * PI).recip()).abs().max(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roofing_filter_creation() {
        let rf = RoofingFilter::new_with_periods(48.0, 10.0);
        assert!(!rf.is_ready());
        assert_eq!(rf.value().main(), 0.0);
    }

    #[test]
    fn test_roofing_filter_compat_constructor() {
        let rf = RoofingFilter::new(0.5, 0.2);
        assert!(!rf.is_ready());
    }

    #[test]
    fn test_roofing_filter_ready_after_bars() {
        let mut rf = RoofingFilter::new_with_periods(48.0, 10.0);
        rf.update_bar(100.0, 101.0, 99.0, 100.0, 1000.0);
        assert!(!rf.is_ready()); // needs 3 bars minimum
        rf.update_bar(101.0, 102.0, 100.0, 101.0, 1000.0);
        rf.update_bar(102.0, 103.0, 101.0, 102.0, 1000.0);
        assert!(rf.is_ready());
    }

    #[test]
    fn test_roofing_filter_finite() {
        let mut rf = RoofingFilter::new_with_periods(48.0, 10.0);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = rf.update_bar(price, price + 1.0, price - 1.0, price, 1000.0);
            assert!(value.is_finite(), "RoofingFilter should always be finite");
        }
    }

    #[test]
    fn test_roofing_filter_zero_mean_cycles() {
        let mut rf = RoofingFilter::new_with_periods(48.0, 10.0);
        let mut sum = 0.0;
        let mut count = 0;
        // Pure sine wave should average near 0 after warmup
        for i in 1..=200 {
            let price = 100.0 + (i as f64 * 2.0 * std::f64::consts::PI / 20.0).sin() * 5.0;
            let v = rf.update_bar(price, price, price, price, 1000.0);
            if rf.is_ready() && i > 50 {
                sum += v;
                count += 1;
            }
        }
        let avg = sum / count as f64;
        // The roofing filter removes DC offset; sine average should be close to 0
        assert!(avg.abs() < 2.0, "Roofing filter of sine should have near-zero mean, got {}", avg);
    }

    #[test]
    fn test_roofing_filter_reset() {
        let mut rf = RoofingFilter::new_with_periods(48.0, 10.0);
        for i in 1..=50 {
            rf.update_bar(100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64, 1000.0);
        }
        rf.reset();
        assert!(!rf.is_ready());
        assert_eq!(rf.value().main(), 0.0);
    }
}
