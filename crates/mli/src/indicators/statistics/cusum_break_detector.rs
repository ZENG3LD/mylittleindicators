// Simplified CUSUM break detector: accumulate returns deviations; emit score of break intensity

#[derive(Debug, Clone)]
pub struct CusumBreakDetector {
    threshold: f64,
    kappa: f64,
    pos: f64,
    neg: f64,
    last_close: Option<f64>,
    pub value: f64,
}

impl CusumBreakDetector {
    pub fn new(threshold: f64, kappa: f64) -> Self {
        Self {
            threshold,
            kappa,
            pos: 0.0,
            neg: 0.0,
            last_close: None,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.pos = 0.0;
        self.neg = 0.0;
        self.last_close = None;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.last_close.is_some()
    }

    /// Feed one pre-extracted close price (resolved by the factory from `const SOURCE`).
    pub fn feed(&mut self, c: f64) -> f64 {
        if let Some(prev) = self.last_close {
            let r = (c / prev).ln();
            self.pos = (self.kappa * (self.pos + r)).max(0.0);
            self.neg = (self.kappa * (self.neg - r)).max(0.0);
            let hit_pos = (self.pos - self.threshold).max(0.0);
            let hit_neg = (self.neg - self.threshold).max(0.0);
            self.value = hit_pos.max(hit_neg);
            if hit_pos > 0.0 {
                self.pos = 0.0;
            }
            if hit_neg > 0.0 {
                self.neg = 0.0;
            }
        }
        self.last_close = Some(c);
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }
}

impl Default for CusumBreakDetector {
    /// Factory defaults: threshold=0.05, kappa=0.95.
    fn default() -> Self {
        Self::new(0.05, 0.95)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, SourceAxis, Store, StoreKind,
    UpdateComplexity,
};
use crate::contract::axis::sweep_f64;
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`CusumBreakDetector`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct CusumBreakDetectorConfig {
    /// Threshold the accumulated CUSUM score must exceed to register a break.
    pub threshold: Param<f64>,
    /// Decay factor applied to the running sums each bar (0 < kappa ≤ 1).
    pub kappa: Param<f64>,
}

impl Indicator for CusumBreakDetector {
    const ID: IndicatorId = IndicatorId::Cusum;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Cusum)];
    /// O(1): single log + clamp + max per bar, pure scalar state.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::fixed(StoreKind::Scalar, 4)]);

    type Config = CusumBreakDetectorConfig;
    type Runtime = CusumBreakDetector;

    fn create(cfg: CusumBreakDetectorConfig) -> CusumBreakDetector {
        CusumBreakDetector::new(cfg.threshold.resolved(), cfg.kappa.resolved())
    }

    fn source_fields(_cfg: &CusumBreakDetectorConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for CusumBreakDetectorConfig {
    fn defaults() -> Self {
        CusumBreakDetectorConfig { threshold: Param::Solo(0.05), kappa: Param::Solo(0.95) }
    }
    fn machine_defaults() -> Self {
        // threshold: Class F (detection threshold) → sweep_f64(0.1,5.0,0.1).
        // kappa: Class F/kappa (named kappa, drift decay factor) → sweep_f64(0.1,2.0,0.1).
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


impl Render for CusumBreakDetector {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Cusum, "CUSUM", Color::hex(0xFF9800))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::indicator_id::IndicatorId;
    use crate::contract::Indicator;

    #[test]
    fn test_cusum_break_detector_creation() {
        let cbd = CusumBreakDetector::new(0.05, 0.95);
        assert!(!cbd.is_ready());
        assert_eq!(cbd.value, 0.0);
    }

    #[test]
    fn test_cusum_break_detector_warmup() {
        let mut cbd = CusumBreakDetector::new(0.05, 0.95);
        cbd.feed(100.0);
        assert!(cbd.is_ready());
    }

    #[test]
    fn test_cusum_break_detector_values() {
        let mut cbd = CusumBreakDetector::new(0.05, 0.95);
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = cbd.feed(price);
            assert!(value >= 0.0, "CUSUM should be non-negative");
        }
    }

    #[test]
    fn test_cusum_break_detector_reset() {
        let mut cbd = CusumBreakDetector::new(0.05, 0.95);
        for i in 0..10 {
            cbd.feed(100.0 + i as f64);
        }
        cbd.reset();
        assert!(!cbd.is_ready());
        assert_eq!(cbd.value, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_cusum() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Cusum(<<CusumBreakDetector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        let mut price = 100.0;
        for _ in 0..20 {
            price *= 1.01;
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: price, volume: 1.0 });
        }
        assert!(f.read(IndicatorOutputId::Cusum) >= 0.0);
    }

    #[test]
    fn contract_id_matches() {
        assert_eq!(<CusumBreakDetector as Indicator>::ID, IndicatorId::Cusum);
    }
}
