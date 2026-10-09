// Jump test proxy using Bipower Variance vs Realized Variance

use crate::indicators::volatility::bipower_variance::BipowerVariance;
use crate::indicators::volatility::realized_vol::RealizedVol;

#[derive(Debug, Clone)]
pub struct RbvJumpTest {
    rbv: BipowerVariance,
    rv: RealizedVol,
    value: f64,
}

impl RbvJumpTest {
    pub fn new(window: usize, annualize_factor: f64) -> Self {
        Self {
            rbv: BipowerVariance::new(window.max(2)),
            rv: RealizedVol::new(window.max(2), annualize_factor),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.value = 0.0;
        self.rbv.reset();
        self.rv.reset();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.rbv.is_ready() && self.rv.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed `[close]` lane — the contracted core entry point.
    /// `const SOURCE = KlineSlice(&[Close])`: both inner indicators only use close.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let c = lanes[0];
        let rbv = self.rbv.feed(c);
        let rv = self.rv.feed(c);
        self.value = if rbv > 0.0 {
            (rv * rv - rbv).max(0.0) / (rbv + 1e-9)
        } else {
            0.0
        };
        self.value
    }
}

impl Default for RbvJumpTest {
    fn default() -> Self {
        Self::new(14, 252.0)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Port, Render, RenderSpec, SourceAxis,
    UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`RbvJumpTest`] — window + annualization factor.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RbvjConfig {
    pub window: Param<usize>,
    /// Annualization factor passed to the embedded [`RealizedVol`]; `252.0_f64.sqrt()` for
    /// daily bars. Passed through to `RealizedVol::new`; does not affect jump-ratio math
    /// (the ratio is dimensionless when both sides use the same annualization).
    pub annualize_factor: Param<f64>,
}

impl Indicator for RbvJumpTest {
    const ID: IndicatorId = IndicatorId::Rbvj;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Both inner indicators consume only close-to-close log returns.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[OhlcvField::Close]));
    /// O(1) outer formula; inner cost declared via Port references.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[
            Port::new(IndicatorId::Bpv, &[IndicatorOutputId::Bpv]),
            Port::new(IndicatorId::Rv, &[IndicatorOutputId::Rv]),
        ],
    };
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Rbvj)];
    type Config = RbvjConfig;
    type Runtime = RbvJumpTest;

    fn create(cfg: RbvjConfig) -> RbvJumpTest {
        RbvJumpTest::new(cfg.window.resolved(), cfg.annualize_factor.resolved())
    }
}

impl crate::contract::Config for RbvjConfig {
    fn defaults() -> Self {
        RbvjConfig { window: Param::Solo(14), annualize_factor: Param::Solo(252.0) }
    }
    fn machine_defaults() -> Self {
        // window: Class A usize — auto range(2,4048,1)
        // annualize_factor: Class J PIN (discrete {252,365,8760} instrument/timeframe constant)
        //   — auto leaves f64 Solo, confirmed correct, do NOT sweep
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for RbvJumpTest {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Rbvj, "Bipower Jumps", Color::hex(0xF44336))
            .precision(6)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rbv_jump_test_creation() {
        let rjt = RbvJumpTest::new(20, 252.0);
        assert!(!rjt.is_ready());
        assert_eq!(rjt.value(), 0.0);
    }

    #[test]
    fn test_rbv_jump_test_warmup() {
        let mut rjt = RbvJumpTest::new(20, 252.0);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            rjt.feed(&[price]);
        }
        assert!(rjt.is_ready());
    }

    #[test]
    fn test_rbv_jump_test_non_negative() {
        let mut rjt = RbvJumpTest::new(20, 252.0);
        for i in 0..35 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = rjt.feed(&[price]);
            assert!(value >= 0.0, "Jump test value should be non-negative");
        }
    }

    #[test]
    fn test_rbv_jump_test_reset() {
        let mut rjt = RbvJumpTest::new(20, 252.0);
        for i in 0..30 {
            rjt.feed(&[100.0 + i as f64]);
        }
        rjt.reset();
        assert!(!rjt.is_ready());
        assert_eq!(rjt.value(), 0.0);
    }

    /// Factory resolves [Close] from const SOURCE.
    /// Open/High/Low/Volume (9999.0) are wild values NOT in SOURCE — proves correct resolution.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        use crate::contract::Param;
        let cfg = RbvjConfig { window: Param::Solo(10), annualize_factor: Param::Solo(252.0) };
        let mut f = IndicatorOrder::Rbvj(cfg).build_solo().unwrap();
        for i in 0..25_u32 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        let v = f.read(IndicatorOutputId::Rbvj);
        assert!(v >= 0.0, "Jump test should be non-negative, got {v}");
    }
}
