// Kalman Trend Regime: classifies trend regime by velocity z-score

use crate::indicators::kalman::basic_kalman_filter::BasicKalmanFilter;
use crate::engine::ohlcv_field::OhlcvField;

#[derive(Debug, Clone)]
pub struct KalmanTrendRegime {
    kf: BasicKalmanFilter,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: i8,
}

impl KalmanTrendRegime {
    pub fn new(dt: f64, process_noise: f64, measurement_noise: f64, z_window: usize) -> Self {
        let w = z_window.max(20);
        Self {
            kf: BasicKalmanFilter::new(dt, process_noise, measurement_noise),
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.kf.reset();
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    #[inline]
    pub fn value(&self) -> f64 {
        (self.value) as f64
    }

    /// Feed ONE pre-extracted scalar (the resolved price field). Knows no transport.
    pub fn feed(&mut self, c: f64) -> i8 {
        let est = self.kf.update(c);
        let vel = est.velocity;
        self.buf[self.idx] = vel;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut m = 0.0;
            for &x in &self.buf {
                m += x;
            }
            m /= self.window as f64;
            let mut s = 0.0;
            for &x in &self.buf {
                let d = x - m;
                s += d * d;
            }
            s = (s / (self.window as f64)).sqrt().max(1e-9);
            let z = (vel - m) / s;
            self.value = if z > 1.0 {
                1
            } else if z < -1.0 {
                -1
            } else {
                0
            };
        } else {
            self.value = 0;
        }
        self.value
    }
}

impl Default for KalmanTrendRegime {
    /// Factory default: dt=1.0, process_noise=0.01, measurement_noise=0.1, z_window=20.
    fn default() -> Self {
        Self::new(1.0, 0.01, 0.1, 20)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Color, Cost, Family, HistogramStyle, Indicator, Output, Param, Render, RenderOutput,
    RenderSpec, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`KalmanTrendRegime`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct KalmanTrendRegimeConfig {
    pub dt: Param<f64>,
    pub process_noise: Param<f64>,
    pub measurement_noise: Param<f64>,
    pub z_window: Param<usize>,
    pub source: Param<OhlcvField>,
}

impl Indicator for KalmanTrendRegime {
    const ID: IndicatorId = IndicatorId::Kregime;
    /// No family — a regime classifier (ternary signal: +1/0/-1 trend state), not a
    /// pluggable MA or oscillator family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(window): each bar rescans the buffer for mean + std. One period-deep Vec.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::Kregime)];
    type Config = KalmanTrendRegimeConfig;
    type Runtime = KalmanTrendRegime;

    fn create(cfg: KalmanTrendRegimeConfig) -> KalmanTrendRegime {
        KalmanTrendRegime::new(cfg.dt.resolved(), cfg.process_noise.resolved(), cfg.measurement_noise.resolved(), cfg.z_window.resolved())
    }

    fn source_fields(cfg: &KalmanTrendRegimeConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for KalmanTrendRegimeConfig {
    fn defaults() -> Self {
        KalmanTrendRegimeConfig {
            dt: Param::Solo(1.0),
            process_noise: Param::Solo(0.01),
            measurement_noise: Param::Solo(0.1),
            z_window: Param::Solo(20),
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
        // z_window: Class A usize — auto sets range(2,4048,1), correct.
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


impl Render for KalmanTrendRegime {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(
                IndicatorOutputId::Kregime,
                "Kalman Regime",
                Color::hex(0x2196F3),
            ))
            .bounds(-1.0, 1.0)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kalman_trend_regime_creation() {
        let ktr = KalmanTrendRegime::new(1.0, 0.1, 1.0, 20);
        assert!(!ktr.is_ready());
        assert_eq!(ktr.value, 0);
    }

    #[test]
    fn test_kalman_trend_regime_warmup() {
        let mut ktr = KalmanTrendRegime::new(1.0, 0.1, 1.0, 20);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ktr.feed(price);
        }
        assert!(ktr.is_ready());
    }

    #[test]
    fn test_kalman_trend_regime_values_range() {
        let mut ktr = KalmanTrendRegime::new(1.0, 0.1, 1.0, 20);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ktr.feed(price);
            assert!(value >= -1 && value <= 1);
        }
    }

    #[test]
    fn test_kalman_trend_regime_reset() {
        let mut ktr = KalmanTrendRegime::new(1.0, 0.1, 1.0, 20);
        for i in 0..30 {
            ktr.feed(100.0 + i as f64);
        }
        ktr.reset();
        assert!(!ktr.is_ready());
        assert_eq!(ktr.value, 0);
    }

    /// Factory resolves close (not the wild 9999 open) and feeds the scalar.
    #[test]
    fn factory_feeds_resolved_kregime() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Kregime(<<KalmanTrendRegime as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 1..=40 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        // Signal(-1/0/1) — main() coerces to f64
        let v = f.read(IndicatorOutputId::Kregime);
        assert!(v >= -1.0 && v <= 1.0, "regime signal should be in [-1,1], got {v}");
    }
}
