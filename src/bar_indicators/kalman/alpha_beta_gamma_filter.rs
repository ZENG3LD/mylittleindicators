// Alpha-Beta-Gamma filter — constant acceleration kinematic tracking filter.
//
// State vector: [position, velocity, acceleration]
// Given measurement z:
//   prediction:
//     x_pred = x + v*dt + 0.5*a*dt^2
//     v_pred = v + a*dt
//     a_pred = a
//   update:
//     residual = z - x_pred
//     x += alpha * residual
//     v += (beta / dt) * residual
//     a += (2*gamma / dt^2) * residual
//
// Where dt=1 (one bar per update), alpha, beta, gamma from 0 to 1.
//
// Output: Triple(position, velocity, acceleration)

use crate::bar_indicators::indicator_value::IndicatorValue;

#[derive(Debug, Clone)]
pub struct AlphaBetaGammaFilter {
    alpha: f64,
    beta: f64,
    gamma: f64,
    /// Estimated position.
    pos: f64,
    /// Estimated velocity (per bar).
    vel: f64,
    /// Estimated acceleration (per bar²).
    acc: f64,
    initialized: bool,
    count: usize,
    period: usize,
}

impl AlphaBetaGammaFilter {
    /// Create filter with given `period` — alpha/beta/gamma are derived from period.
    ///
    /// Standard derivation (critically-damped variant):
    ///   alpha = (2n - 1) / (n * (n + 1) / 2) for some n
    /// Simpler: alpha = 2/(n+1), beta = alpha^2/2, gamma = beta * alpha / 2
    pub fn new(period: usize) -> Self {
        let n = period.max(1) as f64;
        let alpha = 2.0 / (n + 1.0);
        let beta = alpha * alpha / 2.0;
        let gamma = beta * alpha / 2.0;
        Self::with_coefficients(period, alpha, beta, gamma)
    }

    /// Create filter with explicit alpha, beta, gamma coefficients.
    pub fn with_coefficients(period: usize, alpha: f64, beta: f64, gamma: f64) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
            beta: beta.clamp(0.0, 1.0),
            gamma: gamma.clamp(0.0, 1.0),
            pos: 0.0,
            vel: 0.0,
            acc: 0.0,
            initialized: false,
            count: 0,
            period,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.pos = 0.0;
        self.vel = 0.0;
        self.acc = 0.0;
        self.initialized = false;
        self.count = 0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.count >= self.period
    }

    #[inline]
    pub fn value(&self) -> IndicatorValue {
        IndicatorValue::Triple(self.pos, self.vel, self.acc)
    }

    pub fn update_bar(&mut self, _o: f64, _h: f64, _l: f64, c: f64, _v: f64) -> f64 {
        self.count += 1;

        if !self.initialized {
            // Initialize state with first observation
            self.pos = c;
            self.vel = 0.0;
            self.acc = 0.0;
            self.initialized = true;
            return self.pos;
        }

        // dt = 1 bar
        // Predict
        let pos_pred = self.pos + self.vel + 0.5 * self.acc;
        let vel_pred = self.vel + self.acc;
        let acc_pred = self.acc;

        // Residual (measurement - prediction)
        let residual = c - pos_pred;

        // Update
        self.pos = pos_pred + self.alpha * residual;
        self.vel = vel_pred + self.beta * residual;
        self.acc = acc_pred + self.gamma * residual;

        self.pos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alpha_beta_gamma_filter_creation() {
        let filter = AlphaBetaGammaFilter::new(14);
        assert!(!filter.is_ready());
        if let IndicatorValue::Triple(p, v, a) = filter.value() {
            assert_eq!(p, 0.0);
            assert_eq!(v, 0.0);
            assert_eq!(a, 0.0);
        }
    }

    #[test]
    fn test_alpha_beta_gamma_filter_warmup() {
        let mut filter = AlphaBetaGammaFilter::new(10);
        for i in 1..=20 {
            filter.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(filter.is_ready(), "Filter should be ready after 20 bars with period 10");
    }

    #[test]
    fn test_tracks_constant_velocity() {
        let mut filter = AlphaBetaGammaFilter::new(5);
        // Price increases by 1 each bar
        for i in 0..50 {
            filter.update_bar(0.0, 0.0, 0.0, i as f64, 0.0);
        }
        if let IndicatorValue::Triple(pos, vel, _acc) = filter.value() {
            assert!((pos - 49.0).abs() < 2.0, "Position should track price, got {}", pos);
            assert!(vel > 0.5, "Velocity should be positive on uptrend, got {}", vel);
        }
    }

    #[test]
    fn test_finite_values() {
        let mut filter = AlphaBetaGammaFilter::new(10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            filter.update_bar(0.0, 0.0, 0.0, price, 0.0);
            if let IndicatorValue::Triple(p, v, a) = filter.value() {
                assert!(p.is_finite());
                assert!(v.is_finite());
                assert!(a.is_finite());
            }
        }
    }

    #[test]
    fn test_reset_clears_state() {
        let mut filter = AlphaBetaGammaFilter::new(10);
        for i in 1..=20 {
            filter.update_bar(0.0, 0.0, 0.0, 100.0 + i as f64, 0.0);
        }
        assert!(filter.is_ready());
        filter.reset();
        assert!(!filter.is_ready());
        if let IndicatorValue::Triple(p, v, a) = filter.value() {
            assert_eq!(p, 0.0);
            assert_eq!(v, 0.0);
            assert_eq!(a, 0.0);
        }
    }
}
