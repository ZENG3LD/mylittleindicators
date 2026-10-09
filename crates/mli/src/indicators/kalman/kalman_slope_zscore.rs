// Kalman slope Z-score over rolling window

use crate::indicators::kalman::basic_kalman_filter::BasicKalmanFilter;
use crate::engine::ohlcv_field::OhlcvField;

#[derive(Debug, Clone)]
pub struct KalmanSlopeZscore {
    kf: BasicKalmanFilter,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl KalmanSlopeZscore {
    pub fn new(dt: f64, process_noise: f64, measurement_noise: f64, window: usize) -> Self {
        let w = window.max(20);
        Self {
            kf: BasicKalmanFilter::new(dt, process_noise, measurement_noise),
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.kf.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed ONE pre-extracted scalar (the resolved price field). Knows no transport.
    pub fn feed(&mut self, c: f64) -> f64 {
        let estimate = self.kf.update(c);
        let slope = estimate.velocity;
        self.buf[self.idx] = slope;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let n = self.window;
            let mut mean = 0.0;
            for i in 0..n {
                mean += self.buf[i];
            }
            mean /= n as f64;
            let mut var = 0.0;
            for i in 0..n {
                let d = self.buf[i] - mean;
                var += d * d;
            }
            let std = (var / (n as f64)).sqrt().max(1e-9);
            self.value = (slope - mean) / std;
        }
        self.value
    }
}

impl Default for KalmanSlopeZscore {
    /// Factory default: dt=1.0, process_noise=0.01, measurement_noise=0.1, window=20.
    fn default() -> Self {
        Self::new(1.0, 0.01, 0.1, 20)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, SourceAxis, Store,
    StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;
use crate::contract::ReferenceLine;

/// Typed config for [`KalmanSlopeZscore`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct KalmanSlopeZscoreConfig {
    pub dt: Param<f64>,
    pub process_noise: Param<f64>,
    pub measurement_noise: Param<f64>,
    pub window: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl Indicator for KalmanSlopeZscore {
    const ID: IndicatorId = IndicatorId::Kslopez;
    /// No family — a composite z-score detector (slope-velocity z-score), not a pluggable MA
    /// or oscillator family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(window): each bar rescans the buffer for mean + variance. One period-deep Vec.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::Kslopez)];
    type Config = KalmanSlopeZscoreConfig;
    type Runtime = KalmanSlopeZscore;

    fn create(cfg: KalmanSlopeZscoreConfig) -> KalmanSlopeZscore {
        KalmanSlopeZscore::new(cfg.dt.resolved(), cfg.process_noise.resolved(), cfg.measurement_noise.resolved(), cfg.window.resolved())
    }

    fn source_fields(cfg: &KalmanSlopeZscoreConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for KalmanSlopeZscoreConfig {
    fn defaults() -> Self {
        KalmanSlopeZscoreConfig {
            dt: Param::Solo(1.0),
            process_noise: Param::Solo(0.01),
            measurement_noise: Param::Solo(0.1),
            window: Param::Solo(20),
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


impl Render for KalmanSlopeZscore {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Kslopez, "Kalman Slope Z", Color::hex(0x9C27B0))
            .reference_line(ReferenceLine::new(0.0, Color::hex(0x9E9E9E)))
            .reference_line(ReferenceLine::new(2.0, Color::hex(0xFF5722)))
            .reference_line(ReferenceLine::new(-2.0, Color::hex(0x4CAF50)))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_slope_zscore_creation() {
        let ksz = KalmanSlopeZscore::new(1.0, 0.1, 1.0, 20);
        assert!(!ksz.is_ready());
        assert_eq!(ksz.value, 0.0);
    }

    #[test]
    fn test_kalman_slope_zscore_warmup() {
        let mut ksz = KalmanSlopeZscore::new(1.0, 0.1, 1.0, 20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ksz.feed(price);
        }
        assert!(ksz.is_ready());
    }

    #[test]
    fn test_kalman_slope_zscore_values_finite() {
        let mut ksz = KalmanSlopeZscore::new(1.0, 0.1, 1.0, 20);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ksz.feed(price);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_kalman_slope_zscore_reset() {
        let mut ksz = KalmanSlopeZscore::new(1.0, 0.1, 1.0, 20);
        for i in 0..30 {
            ksz.feed(100.0 + i as f64);
        }
        ksz.reset();
        assert!(!ksz.is_ready());
        assert_eq!(ksz.value, 0.0);
    }

    /// Factory resolves close (not the wild 9999 high/open) and feeds the scalar.
    #[test]
    fn factory_feeds_resolved_kslopez() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Kslopez(<<KalmanSlopeZscore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=40 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.read(IndicatorOutputId::Kslopez).is_finite(), "KalmanSlopeZscore should be finite");
    }
}
