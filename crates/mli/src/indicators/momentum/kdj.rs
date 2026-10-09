// KDJ oscillator: %K, %D (SMA of %K), %J = 3*D - 2*K

use crate::engine::contract_engine::SmootherSlot;
use crate::indicators::momentum::stochastics::Stochastics;

#[derive(Debug, Clone)]
pub struct Kdj {
    stoch: Stochastics,
    d_ma: SmootherSlot,
    k: f64,
    d: f64,
    j: f64,
}

impl Kdj {
    /// Default ctor — outer %D smoothed with SMA (the classic KDJ kernel).
    pub fn new(k_period: usize, d_period: usize) -> Self {
        Self::from_smoother(k_period, d_period, SmootherId::Sma)
    }

    /// Build from a narrow `SmootherId` for the OUTER %D smoother + the K/D periods.
    /// The inner stochastic keeps its own default %D smoothing. Legacy bridge; the contract
    /// path goes through `KdjConfig`.
    pub fn from_smoother(k_period: usize, d_period: usize, d_smoother: SmootherId) -> Self {
        Self {
            stoch: Stochastics::new(k_period.max(1), d_period.max(1)),
            d_ma: SmootherSlot::new(d_smoother, d_period.max(1)),
            k: 0.0,
            d: 0.0,
            j: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.stoch.reset();
        self.d_ma.reset();
        self.k = 0.0;
        self.d = 0.0;
        self.j = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.d_ma.is_ready()
    }
    /// Feed the resolved input lanes — `[high, low, close]` (the factory resolves the fixed
    /// HLC slice). The inner stochastic is still bar-driven (un-converted); feed it the
    /// resolved h/l/c as a synthetic bar. Knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64, f64) {
        let (h, l, c) = (lanes[0], lanes[1], lanes[2]);
        let (k_raw, _d_ignore) = self.stoch.feed(&[h, l, c]);
        self.k = k_raw;
        self.d = self.d_ma.feed(self.k);
        self.j = 3.0 * self.d - 2.0 * self.k;
        (self.k, self.d, self.j)
    }

    pub fn k(&self) -> f64 {
        self.k
    }

    pub fn d(&self) -> f64 {
        self.d
    }

    pub fn j(&self) -> f64 {
        self.j
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, UpdateComplexity,
};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::SmootherId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderOutput, RenderSpec};

/// Dual-mode contract config for [`Kdj`] — %K/%D/%J over a stochastic.
///
/// `k_period` sizes the inner stochastic window; `d_period` is the outer %D smoother period;
/// `d_smoother` is the configurable MA choice (default: follow(Sma)).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct KdjConfig {
    /// %K lookback / inner stochastic window.
    pub k_period: Param<usize>,
    /// Outer %D smoothing period.
    pub d_period: Param<usize>,
    /// Outer %D smoother choice. Default: follow(Sma).
    #[slot]
    pub d_smoother: Param<SmootherChoice>,
}

impl Indicator for Kdj {
    const ID: IndicatorId = IndicatorId::Kdj;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Stochastic over the raw range slices (h/l/c).
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low, OhlcvField::Close]));
    /// O(1) over the inner stochastic's %K; the inner stochastic is a fixed `Port`, the
    /// outer %D smoother is the configurable `SLOTS` member.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Stoch, &[IndicatorOutputId::StochK])],
    };
    const SLOTS: &'static [Slot] = KdjConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::KdjK),
        Output::percent(IndicatorOutputId::KdjD),
        Output::percent(IndicatorOutputId::KdjJ),
    ];
    type Config = KdjConfig;
    type Runtime = Kdj;

    fn create(cfg: KdjConfig) -> Kdj {
        let k_period = cfg.k_period.resolved().max(1);
        let d_period = cfg.d_period.resolved().max(1);
        Kdj {
            stoch: Stochastics::new(k_period, d_period),
            d_ma: cfg.d_smoother.resolved().build(d_period),
            k: 0.0,
            d: 0.0,
            j: 0.0,
        }
    }

    fn slot_members(cfg: &KdjConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for KdjConfig {
    fn defaults() -> Self {
        KdjConfig {
            k_period: Param::Solo(14),
            d_period: Param::Solo(3),
            d_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // k_period/d_period: Class A → auto range(2,4048,1).
        // d_smoother: #[slot] SmootherChoice → left Solo (deferred wave).
        Self::machine_defaults_auto()
    }
}


impl Render for Kdj {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(IndicatorOutputId::KdjK, "K", Color::hex(0x2196F3), 2.0))
            .output(RenderOutput::line(IndicatorOutputId::KdjD, "D", Color::hex(0xFF9800), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::KdjJ, "J", Color::hex(0x9C27B0), 1.0))
            .bounds(0.0, 100.0)
            .reference_line(ReferenceLine::new(80.0, Color::hex(0x9E9E9E)))
            .reference_line(ReferenceLine::new(20.0, Color::hex(0x9E9E9E)))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kdj_creation() {
        let kdj = Kdj::new(9, 3);
        assert!(!kdj.is_ready());
        assert_eq!(kdj.k(), 0.0);
        assert_eq!(kdj.d(), 0.0);
        assert_eq!(kdj.j(), 0.0);
    }

    #[test]
    fn test_kdj_with_d_smoother() {
        let mut kdj = Kdj::from_smoother(9, 3, SmootherId::Ema);
        for i in 1..=25 {
            let p = 100.0 + i as f64 * 0.5;
            let (k, d, j) = kdj.feed(&[p + 1.0, p - 0.5, p + 0.3]);
            assert!(k.is_finite() && d.is_finite() && j.is_finite());
        }
        assert!(kdj.is_ready());
    }

    #[test]
    fn test_kdj_basic_calculation() {
        let mut kdj = Kdj::new(9, 3);

        for i in 1..=20 {
            let price = 100.0 + i as f64;
            let (k, d, j) = kdj.feed(&[price + 2.0, price - 1.0, price + 1.0]);

            if kdj.is_ready() {
                assert!(k >= 0.0 && k <= 100.0);
                assert!(d >= 0.0 && d <= 100.0);
                // J can be outside 0-100 range
                assert!(j.is_finite());
            }
        }
    }

    #[test]
    fn test_kdj_uptrend() {
        let mut kdj = Kdj::new(9, 3);

        for i in 1..=25 {
            let price = 100.0 + i as f64;
            kdj.feed(&[price + 1.0, price - 0.5, price + 0.5]);
        }

        if kdj.is_ready() {
            // In uptrend, K should be high
            assert!(kdj.k() > 50.0, "K in uptrend should be > 50, got {}", kdj.k());
        }
    }

    #[test]
    fn test_kdj_j_formula() {
        let mut kdj = Kdj::new(9, 3);

        for i in 1..=20 {
            let price = 100.0 + i as f64;
            kdj.feed(&[price + 2.0, price - 1.0, price + 1.0]);
        }

        // J = 3*D - 2*K
        let expected_j = 3.0 * kdj.d() - 2.0 * kdj.k();
        assert!((kdj.j() - expected_j).abs() < 1e-10);
    }

    #[test]
    fn test_kdj_reset() {
        let mut kdj = Kdj::new(9, 3);

        for i in 1..=20 {
            let price = 100.0 + i as f64;
            kdj.feed(&[price + 1.0, price - 1.0, price]);
        }

        kdj.reset();
        assert!(!kdj.is_ready());
        assert_eq!(kdj.k(), 0.0);
        assert_eq!(kdj.d(), 0.0);
        assert_eq!(kdj.j(), 0.0);
    }

    #[test]
    fn test_kdj_contract_create() {
        let cfg = <<Kdj as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.d_period.resolved(), 3);
        let mut kdj = <Kdj as Indicator>::create(cfg);
        for i in 1..=25 {
            let price = 100.0 + i as f64;
            kdj.feed(&[price + 1.0, price - 0.5, price + 0.5]);
        }
        assert!(kdj.is_ready());
    }

    /// The factory resolves the fixed HLC lanes and feeds the three scalars; KDJ (with its
    /// inner stochastic) runs end-to-end and stays in range.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Kdj(<<Kdj as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: price, high: price + 1.0, low: price - 0.5, close: price + 0.5, volume: 1000.0,
            });
        }
        let k = f.primary();
        assert!(k >= 0.0 && k <= 100.0, "factory KDJ %K in [0,100], got {}", k);
    }
}
