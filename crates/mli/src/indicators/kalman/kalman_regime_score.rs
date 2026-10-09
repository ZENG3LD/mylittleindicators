// Continuous Kalman regime score based on velocity z-score with sigmoid mapping

use crate::indicators::kalman::kalman_trend_slope::KalmanTrendSlope;

#[derive(Debug, Clone)]
pub struct KalmanRegimeScore {
    inner: KalmanTrendSlope,
    pub value: f64,
}

impl KalmanRegimeScore {
    pub fn new(
        dt: f64,
        process_noise: f64,
        measurement_noise: f64,
        window: usize,
        decay: f64,
    ) -> Self {
        let _ = decay;
        Self {
            inner: KalmanTrendSlope::new(dt, process_noise, measurement_noise, window),
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.inner.is_ready()
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn feed(&mut self, c: f64) -> f64 {
        let (_s, z) = self.inner.feed(c);
        self.value = 0.5 * (z.tanh() + 1.0);
        self.value
    }
}

impl Default for KalmanRegimeScore {
    fn default() -> Self {
        Self::new(1.0, 0.01, 0.1, 20, 0.95)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, Port, SourceAxis, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};
use crate::contract::sweep_f64;
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`KalmanRegimeScore`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct KalmanRegimeScoreConfig {
    pub dt: Param<f64>,
    pub process_noise: Param<f64>,
    pub measurement_noise: Param<f64>,
    pub window: Param<usize>,
    pub decay: Param<f64>,
    pub source: Param<OhlcvField>,
}

impl Indicator for KalmanRegimeScore {
    const ID: IndicatorId = IndicatorId::Kscr;
    /// Kalman-based regime classifier — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Single configurable price field (default: Close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// Outer is O(1) after the inner slope z-score is computed. Inner KalmanTrendSlope
    /// cost is charged via the Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Kslope, &[IndicatorOutputId::KslopeSlope])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Kscr)];

    type Config = KalmanRegimeScoreConfig;
    type Runtime = KalmanRegimeScore;

    fn create(cfg: KalmanRegimeScoreConfig) -> KalmanRegimeScore {
        KalmanRegimeScore::new(
            cfg.dt.resolved(),
            cfg.process_noise.resolved(),
            cfg.measurement_noise.resolved(),
            cfg.window.resolved(),
            cfg.decay.resolved(),
        )
    }

    fn source_fields(cfg: &KalmanRegimeScoreConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for KalmanRegimeScoreConfig {
    fn defaults() -> Self {
        KalmanRegimeScoreConfig {
            dt: Param::Solo(1.0),
            process_noise: Param::Solo(0.01),
            measurement_noise: Param::Solo(0.1),
            window: Param::Solo(20),
            decay: Param::Solo(0.95),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // dt: PIN — always 1.0 (one bar). auto leaves f64 Solo; confirmed not touched.
        // process_noise: Class H — 5 decade-spaced values.
        s.process_noise = Param::many(vec![0.0001, 0.001, 0.01, 0.1, 1.0]);
        // measurement_noise: Class H — 5 decade-spaced values.
        s.measurement_noise = Param::many(vec![0.0001, 0.001, 0.01, 0.1, 1.0]);
        // window: Class A usize — auto sets range(2,4048,1), correct.
        // decay: Class E kalman sub-case — sweep_f64(0.80, 0.99, 0.01).
        s.decay = Param::many(sweep_f64(0.80, 0.99, 0.01));
        // source: Class O — auto sets all 8 OhlcvField variants.
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for KalmanRegimeScore {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Kscr, "Kalman Score", Color::hex(0xFF9800))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

// Transitional bridge: kalman_regime_composite still calls update_bar; delegates to feed.
impl KalmanRegimeScore {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_regime_score_creation() {
        let krs = KalmanRegimeScore::new(1.0, 0.1, 1.0, 20, 0.94);
        assert!(!krs.is_ready());
        assert_eq!(krs.value, 0.0);
    }

    #[test]
    fn test_kalman_regime_score_warmup() {
        let mut krs = KalmanRegimeScore::new(1.0, 0.1, 1.0, 10, 0.94);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            krs.feed(price);
        }
        assert!(krs.is_ready());
    }

    #[test]
    fn test_kalman_regime_score_values_range() {
        let mut krs = KalmanRegimeScore::new(1.0, 0.1, 1.0, 10, 0.94);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = krs.feed(price);
            assert!(value >= 0.0 && value <= 1.0);
        }
    }

    #[test]
    fn test_kalman_regime_score_reset() {
        let mut krs = KalmanRegimeScore::new(1.0, 0.1, 1.0, 10, 0.94);
        for i in 0..20 {
            krs.feed(100.0 + i as f64);
        }
        krs.reset();
        assert!(!krs.is_ready());
        assert_eq!(krs.value, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_kscr() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Kscr(<<KalmanRegimeScore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        let v = f.read(IndicatorOutputId::Kscr);
        assert!(v.is_finite());
        assert!(v >= 0.0 && v <= 1.0);
    }
}
