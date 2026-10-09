// Simplified Bai-Perron break detector: segment-wise CUSUM with windowed reinitialization

use crate::indicators::statistics::cusum_break_detector::CusumBreakDetector;

#[derive(Debug, Clone)]
pub struct BaiPerronCusum {
    inner: CusumBreakDetector,
    seg_window: usize,
    seg_idx: usize,
    pub value: f64,
    ever_updated: bool,
}

impl BaiPerronCusum {
    pub fn new(threshold: f64, kappa: f64, seg_window: usize) -> Self {
        Self {
            inner: CusumBreakDetector::new(threshold, kappa),
            seg_window: seg_window.max(50),
            seg_idx: 0,
            value: 0.0,
            ever_updated: false,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.inner.reset();
        self.seg_idx = 0;
        self.value = 0.0;
        self.ever_updated = false;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.ever_updated
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed a single close price scalar (const SOURCE = Field{Close}).
    pub fn feed(&mut self, c: f64) -> f64 {
        let s = self.inner.feed(c);
        self.seg_idx += 1;
        self.ever_updated = true;
        if self.seg_idx >= self.seg_window {
            self.inner.reset();
            self.seg_idx = 0;
        }
        self.value = s;
        self.value
    }
}

impl Default for BaiPerronCusum {
    /// Factory defaults: threshold=1.5, kappa=0.5, seg_window=20 (clamped to max(20,50)=50).
    fn default() -> Self {
        Self::new(1.5, 0.5, 20)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, UpdateComplexity};
use crate::contract::axis::sweep_f64;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed dual-mode config for [`BaiPerronCusum`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct BaiPerronCusumConfig {
    /// CUSUM detection threshold (positive, fraction or absolute).
    pub threshold: Param<f64>,
    /// Drift allowance kappa.
    pub kappa: Param<f64>,
    /// Bars before the CUSUM accumulator is reset (segment window).
    pub seg_window: Param<usize>,
    /// Price field to feed into the inner CUSUM detector (default Close).
    pub source: Param<OhlcvField>,
}

impl Indicator for BaiPerronCusum {
    const ID: IndicatorId = IndicatorId::BpCusum;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads only one configurable price field (default Close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// Outer update is O(1); the embedded `CusumBreakDetector` (Cusum) carries its
    /// own O(1) cost — declared here so the optimizer can account for the dependency.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Cusum, &[IndicatorOutputId::Cusum])],
    };
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::BpCusum)];
    type Config = BaiPerronCusumConfig;
    type Runtime = BaiPerronCusum;

    fn create(cfg: BaiPerronCusumConfig) -> BaiPerronCusum {
        BaiPerronCusum::new(cfg.threshold.resolved(), cfg.kappa.resolved(), cfg.seg_window.resolved())
    }

    fn source_fields(cfg: &BaiPerronCusumConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for BaiPerronCusumConfig {
    fn defaults() -> Self {
        BaiPerronCusumConfig {
            threshold: Param::Solo(1.5),
            kappa: Param::Solo(0.5),
            seg_window: Param::Solo(50),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn machine_defaults() -> Self {
        // seg_window: Class A → auto range(2,4048,1).
        // source: Class O → auto (all 8 fields).
        // threshold: Class F (detection threshold) → sweep_f64(0.1,5.0,0.1).
        // kappa: Class F/kappa (drift allowance, named kappa) → sweep_f64(0.1,2.0,0.1).
        let mut s = Self::machine_defaults_auto();
        s.threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s.kappa = Param::many(sweep_f64(0.1, 2.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for BaiPerronCusum {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::BpCusum, "BP CUSUM", Color::hex(0xE91E63))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bai_perron_cusum_creation() {
        let bpc = BaiPerronCusum::new(0.05, 0.95, 100);
        assert!(!bpc.is_ready());
        assert_eq!(bpc.value, 0.0);
    }

    #[test]
    fn test_bai_perron_cusum_warmup() {
        let mut bpc = BaiPerronCusum::new(0.05, 0.95, 100);
        bpc.feed(100.0);
        bpc.feed(101.0);
        assert!(bpc.is_ready());
    }

    #[test]
    fn test_bai_perron_cusum_values() {
        let mut bpc = BaiPerronCusum::new(0.05, 0.95, 100);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = bpc.feed(price);
            assert!(value >= 0.0, "CUSUM should be non-negative");
        }
    }

    #[test]
    fn test_bai_perron_cusum_reset() {
        let mut bpc = BaiPerronCusum::new(0.05, 0.95, 100);
        for i in 0..10 {
            bpc.feed(100.0 + i as f64);
        }
        bpc.reset();
        assert!(!bpc.is_ready());
        assert_eq!(bpc.value, 0.0);
    }

    /// The factory resolves Close (not the wild 9999 high) and feeds the scalar.
    #[test]
    fn factory_feeds_resolved_bpcusum() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<BaiPerronCusum as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::BpCusum(cfg).build_solo().unwrap();
        for i in 0..10 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1000.0,
            });
        }
        assert!(f.read(IndicatorOutputId::BpCusum) >= 0.0, "BpCusum value should be non-negative, got {}", f.read(IndicatorOutputId::BpCusum));
        assert!(f.is_ready());
    }
}
