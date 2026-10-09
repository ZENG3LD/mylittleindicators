// Kalman Trend Slope and Z-Score using BasicKalmanFilter velocity/acceleration

use crate::indicators::kalman::basic_kalman_filter::BasicKalmanFilter;
use crate::engine::ohlcv_field::OhlcvField;

#[derive(Debug, Clone)]
pub struct KalmanTrendSlope {
    kf: BasicKalmanFilter,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub slope: f64,
    pub slope_z: f64,
}

impl KalmanTrendSlope {
    pub fn new(dt: f64, process_noise: f64, measurement_noise: f64, window: usize) -> Self {
        Self {
            kf: BasicKalmanFilter::new(dt, process_noise, measurement_noise),
            window: window.max(10),
            buf: vec![0.0; window.max(10)],
            idx: 0,
            filled: false,
            slope: 0.0,
            slope_z: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.kf.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.slope = 0.0;
        self.slope_z = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.window >= 10
    }

    /// Feed ONE pre-extracted scalar (the resolved price field). Knows no transport.
    pub fn feed(&mut self, c: f64) -> (f64, f64) {
        let res = self.kf.update(c);
        self.slope = res.velocity;
        // ring buffer for z-score
        self.buf[self.idx] = self.slope;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        let n = if self.filled {
            self.window
        } else {
            self.idx.max(1)
        };
        let mut sum = 0.0;
        let mut sumsq = 0.0;
        for i in 0..n {
            let v = self.buf[i];
            sum += v;
            sumsq += v * v;
        }
        let mean = sum / n as f64;
        let var = (sumsq / n as f64) - mean * mean;
        let sd = var.max(1e-12).sqrt();
        self.slope_z = (self.slope - mean) / sd;
        (self.slope, self.slope_z)
    }
}

impl KalmanTrendSlope {
    /// Kalman velocity — slope of the filtered price series.
    pub fn slope(&self) -> f64 {
        self.slope
    }

    /// Z-score of the slope within its rolling window.
    pub fn slope_z(&self) -> f64 {
        self.slope_z
    }
}

impl Default for KalmanTrendSlope {
    /// Factory default: dt=1.0, process_noise=0.01, measurement_noise=0.1, window=20.
    fn default() -> Self {
        Self::new(1.0, 0.01, 0.1, 20)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderOutput, RenderSpec, SourceAxis,
    Store, StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`KalmanTrendSlope`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct KalmanTrendSlopeConfig {
    pub dt: Param<f64>,
    pub process_noise: Param<f64>,
    pub measurement_noise: Param<f64>,
    pub window: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl Indicator for KalmanTrendSlope {
    const ID: IndicatorId = IndicatorId::Kslope;
    /// No family — a Kalman-velocity tracker (slope + z-score pair), not a pluggable MA
    /// or oscillator family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(window): each bar scans the buffer for mean + variance. One period-deep Vec.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::KslopeSlope),
        Output::centered(IndicatorOutputId::KslopeSlopeZ),
    ];
    type Config = KalmanTrendSlopeConfig;
    type Runtime = KalmanTrendSlope;

    fn create(cfg: KalmanTrendSlopeConfig) -> KalmanTrendSlope {
        KalmanTrendSlope::new(cfg.dt.resolved(), cfg.process_noise.resolved(), cfg.measurement_noise.resolved(), cfg.window.resolved())
    }

    fn source_fields(cfg: &KalmanTrendSlopeConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for KalmanTrendSlopeConfig {
    fn defaults() -> Self {
        KalmanTrendSlopeConfig {
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


impl Render for KalmanTrendSlope {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::KslopeSlope,
                "Kalman Slope",
                Color::hex(0x009688),
                2.0,
            ))
            .output(RenderOutput::line(
                IndicatorOutputId::KslopeSlopeZ,
                "Kalman Slope Z",
                Color::hex(0x9C27B0),
                1.0,
            ).hidden())
            .zero_baseline()
            .precision(4)
            .build()
    }
}

// Transitional bridge: un-contracted composites still call update_bar; delegates to feed.
impl KalmanTrendSlope {
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_trend_slope_creation() {
        let kts = KalmanTrendSlope::new(1.0, 0.1, 1.0, 20);
        assert!(!kts.is_ready());
        assert_eq!(kts.slope, 0.0);
        assert_eq!(kts.slope_z, 0.0);
    }

    #[test]
    fn test_kalman_trend_slope_warmup() {
        let mut kts = KalmanTrendSlope::new(1.0, 0.1, 1.0, 10);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kts.feed(price);
        }
        assert!(kts.is_ready());
    }

    #[test]
    fn test_kalman_trend_slope_values_finite() {
        let mut kts = KalmanTrendSlope::new(1.0, 0.1, 1.0, 10);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let (slope, slope_z) = kts.feed(price);
            assert!(slope.is_finite());
            assert!(slope_z.is_finite());
        }
    }

    #[test]
    fn test_kalman_trend_slope_reset() {
        let mut kts = KalmanTrendSlope::new(1.0, 0.1, 1.0, 10);
        for i in 0..20 {
            kts.feed(100.0 + i as f64);
        }
        kts.reset();
        assert!(!kts.is_ready());
        assert_eq!(kts.slope, 0.0);
        assert_eq!(kts.slope_z, 0.0);
    }

    /// Factory resolves close (not the wild 9999 open) and feeds the scalar.
    #[test]
    fn factory_feeds_resolved_kslope() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Kslope(<<KalmanTrendSlope as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.read(IndicatorOutputId::KslopeSlope).is_finite(), "KalmanTrendSlope slope should be finite");
    }
}
