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


    /// Feed ONE pre-extracted scalar (the resolved price field).
    pub fn feed(&mut self, c: f64) -> f64 {
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

impl AlphaBetaGammaFilter {
    /// Estimated position (filtered price).
    pub fn pos(&self) -> f64 {
        self.pos
    }

    /// Estimated velocity (price change per bar).
    pub fn vel(&self) -> f64 {
        self.vel
    }

    /// Estimated acceleration (per bar squared).
    pub fn acc(&self) -> f64 {
        self.acc
    }
}

impl Default for AlphaBetaGammaFilter {
    /// Factory default: period=20.
    fn default() -> Self {
        Self::new(20)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::engine::stream_kind::StreamKind;
use crate::engine::ohlcv_field::OhlcvField;

/// Own config for [`AlphaBetaGammaFilter`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AbgfilterConfig {
    pub period: Param<usize>,
}

impl Indicator for AlphaBetaGammaFilter {
    const ID: IndicatorId = IndicatorId::Abgfilter;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::AbgfilterPos),
        Output::centered(IndicatorOutputId::AbgfilterVel),
        Output::centered(IndicatorOutputId::AbgfilterAcc),
    ];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = AbgfilterConfig;
    type Runtime = AlphaBetaGammaFilter;

    fn create(cfg: AbgfilterConfig) -> AlphaBetaGammaFilter {
        AlphaBetaGammaFilter::new(cfg.period.resolved())
    }

    fn source_fields(_cfg: &AbgfilterConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for AbgfilterConfig {
    fn defaults() -> Self {
        AbgfilterConfig { period: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // period: Class A usize — auto sets range(2,4048,1), which is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for AlphaBetaGammaFilter {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::AbgfilterPos, "ABG Filter", Color::hex(0x00BCD4))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alpha_beta_gamma_filter_creation() {
        let filter = AlphaBetaGammaFilter::new(14);
        assert!(!filter.is_ready());
        {
            let (p, v, a) = (filter.pos(), filter.vel(), filter.acc());
            assert_eq!(p, 0.0);
            assert_eq!(v, 0.0);
            assert_eq!(a, 0.0);
        }
    }

    #[test]
    fn test_alpha_beta_gamma_filter_warmup() {
        let mut filter = AlphaBetaGammaFilter::new(10);
        for i in 1..=20 {
            filter.feed(100.0 + i as f64);
        }
        assert!(filter.is_ready(), "Filter should be ready after 20 bars with period 10");
    }

    #[test]
    fn test_tracks_constant_velocity() {
        let mut filter = AlphaBetaGammaFilter::new(5);
        // Price increases by 1 each bar
        for i in 0..50 {
            filter.feed(i as f64);
        }
        {
            let pos = filter.pos();
            let vel = filter.vel();
            assert!((pos - 49.0).abs() < 2.0, "Position should track price, got {}", pos);
            assert!(vel > 0.5, "Velocity should be positive on uptrend, got {}", vel);
        }
    }

    #[test]
    fn test_finite_values() {
        let mut filter = AlphaBetaGammaFilter::new(10);
        for i in 1..=100 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            filter.feed(price);
            {
                let p = filter.pos();
                let v = filter.vel();
                let a = filter.acc();
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
            filter.feed(100.0 + i as f64);
        }
        assert!(filter.is_ready());
        filter.reset();
        assert!(!filter.is_ready());
        {
            let (p, v, a) = (filter.pos(), filter.vel(), filter.acc());
            assert_eq!(p, 0.0);
            assert_eq!(v, 0.0);
            assert_eq!(a, 0.0);
        }
    }

    /// Factory resolves close (not the wild 9999 high) and feeds the scalar.
    #[test]
    fn factory_feeds_resolved_abgfilter() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::{MarketSample, Param};
        let mut cfg = <<AlphaBetaGammaFilter as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        cfg.period = Param::Solo(20);
        let mut f = IndicatorOrder::Abgfilter(cfg).build_solo().unwrap();
        for i in 0..30 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.read(IndicatorOutputId::AbgfilterPos).is_finite(), "ABG position should be finite");
        assert!(f.read(IndicatorOutputId::AbgfilterPos) > 50.0, "ABG should track close, got {}", f.read(IndicatorOutputId::AbgfilterPos));
    }
}
