// CUSUM filter on returns: emits event when cumulative sum crosses +/- threshold

#[derive(Debug, Clone)]
pub struct CusumFilter {
    threshold: f64,
    last_close: Option<f64>,
    pos_sum: f64,
    neg_sum: f64,
    pub event: i8, // -1, 0, +1
}

impl CusumFilter {
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold: threshold.abs().max(1e-12),
            last_close: None,
            pos_sum: 0.0,
            neg_sum: 0.0,
            event: 0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.last_close = None;
        self.pos_sum = 0.0;
        self.neg_sum = 0.0;
        self.event = 0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.last_close.is_some()
    }

    /// Feed one pre-extracted close price.
    pub fn feed(&mut self, close: f64) -> i8 {
        self.event = 0;
        if let Some(prev) = self.last_close {
            let r = (close / prev) - 1.0;
            self.pos_sum = (self.pos_sum + r).max(0.0);
            self.neg_sum = (self.neg_sum + r).min(0.0);
            if self.pos_sum > self.threshold {
                self.event = 1;
                self.pos_sum = 0.0;
                self.neg_sum = 0.0;
            }
            if self.neg_sum < -self.threshold {
                self.event = -1;
                self.pos_sum = 0.0;
                self.neg_sum = 0.0;
            }
        }
        self.last_close = Some(close);
        self.event
    }

    #[inline]
    pub fn value(&self) -> f64 {
        (self.event) as f64
    }

    pub fn threshold(&self) -> f64 {
        self.threshold
    }
}

impl Default for CusumFilter {
    /// Factory defaults: threshold=0.01.
    fn default() -> Self {
        Self::new(0.01)
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

/// Typed dual-mode config for [`CusumFilter`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct CusumConfig {
    /// Return threshold (absolute); event fires when cumulative sum exceeds ±threshold.
    pub threshold: Param<f64>,
}

impl Indicator for CusumFilter {
    const ID: IndicatorId = IndicatorId::StCusum;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::StCusum)];
    /// O(1): single add + clamp per bar.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[Store::fixed(StoreKind::Scalar, 2)]);

    type Config = CusumConfig;
    type Runtime = CusumFilter;

    fn create(cfg: CusumConfig) -> CusumFilter {
        CusumFilter::new(cfg.threshold.resolved())
    }

    fn source_fields(_cfg: &CusumConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for CusumConfig {
    fn defaults() -> Self {
        CusumConfig { threshold: Param::Solo(0.01) }
    }
    fn machine_defaults() -> Self {
        // threshold: Class F (return threshold for cumulative sum filter) → sweep_f64(0.1,5.0,0.1).
        let mut s = Self::machine_defaults_auto();
        s.threshold = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for CusumFilter {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::StCusum, "CUSUM", Color::hex(0x2196F3))
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
    fn test_cusum_creation() {
        let cusum = CusumFilter::new(0.05);
        assert!(!cusum.is_ready());
        assert_eq!(cusum.event, 0);
        assert!((cusum.threshold() - 0.05).abs() < 1e-9);
    }

    #[test]
    fn test_cusum_positive_event() {
        let mut cusum = CusumFilter::new(0.05);
        // Feed rising prices to trigger positive event
        let mut price = 100.0;
        let mut triggered = false;
        for _ in 0..20 {
            price *= 1.02; // 2% increase each bar
            let event = cusum.feed(price);
            if event == 1 {
                triggered = true;
                break;
            }
        }
        assert!(triggered, "CUSUM should trigger positive event on rising prices");
    }

    #[test]
    fn test_cusum_negative_event() {
        let mut cusum = CusumFilter::new(0.05);
        let mut price = 100.0;
        let mut triggered = false;
        for _ in 0..20 {
            price *= 0.98; // 2% decrease each bar
            let event = cusum.feed(price);
            if event == -1 {
                triggered = true;
                break;
            }
        }
        assert!(triggered, "CUSUM should trigger negative event on falling prices");
    }

    #[test]
    fn test_cusum_reset() {
        let mut cusum = CusumFilter::new(0.05);
        cusum.feed(100.0);
        assert!(cusum.is_ready());
        cusum.reset();
        assert!(!cusum.is_ready());
        assert_eq!(cusum.event, 0);
    }

    #[test]
    fn factory_feeds_resolved_st_cusum() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::StCusum(<<CusumFilter as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        let mut price = 100.0;
        for _ in 0..50 {
            price *= 1.005;
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: price, volume: 1.0 });
        }
        // Signal fires or stays 0; main() returns the i8 as f64
        let v = f.read(IndicatorOutputId::StCusum);
        assert!(v == 0.0 || v == 1.0 || v == -1.0);
    }

    #[test]
    fn contract_id_matches() {
        assert_eq!(<CusumFilter as Indicator>::ID, IndicatorId::StCusum);
    }
}
