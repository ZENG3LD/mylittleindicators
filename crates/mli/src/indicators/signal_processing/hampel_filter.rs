// Hampel Filter - robust outlier smoother using median and MAD

use crate::indicators::utils::math::percentile::median;

#[derive(Debug, Clone)]
pub struct HampelFilter {
    window: usize,
    k: f64,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    value: f64,
}

impl HampelFilter {
    pub fn new(window: usize, k: f64) -> Self {
        Self {
            window: window.clamp(3, 512),
            k: if k > 0.0 { k } else { 3.0 },
            buf: Vec::with_capacity(window.clamp(3, 512)),
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.buf.clear();
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
    pub fn feed(&mut self, c: f64) -> f64 {
        if self.buf.len() < self.window {
            self.buf.push(c);
            if self.buf.len() == self.window {
                self.filled = true;
            }
        } else {
            self.buf[self.idx] = c;
        }
        self.idx = (self.idx + 1) % self.window;
        if self.is_ready() {
            // Optimized: Use O(n) quickselect for median instead of O(n log n) full sort
            let mut v: Vec<f64> = self.buf.iter().copied().collect();
            let med = median(&mut v);

            // Optimized: Use O(n) quickselect for MAD instead of O(n log n) full sort
            let mut dev: Vec<f64> = self.buf.iter().map(|x| (x - med).abs()).collect();
            let mad = median(&mut dev);

            let sigma = 1.4826 * mad;
            let x = c;
            let z = (x - med) / sigma.max(1e-9);
            self.value = if z.abs() > self.k {
                med + self.k * sigma * z.signum()
            } else {
                x
            };
        }
        self.value
    }

    pub fn window(&self) -> usize {
        self.window
    }

    pub fn k(&self) -> f64 {
        self.k
    }

}

impl Default for HampelFilter {
    /// Factory default: window=25, k=3.0.
    fn default() -> Self {
        Self::new(25, 3.0)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::contract::axis::sweep_f64;
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`HampelFilter`]: rolling window size + sigma-multiplier k.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct HampelConfig {
    pub window: Param<usize>,
    pub k: Param<f64>,
}

impl Indicator for HampelFilter {
    const ID: IndicatorId = IndicatorId::Hampel;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(window): each bar computes median over the rolling window (quickselect).
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[crate::contract::Output::price(IndicatorOutputId::Hampel)];
    type Config = HampelConfig;
    type Runtime = HampelFilter;

    fn create(cfg: HampelConfig) -> HampelFilter {
        HampelFilter::new(cfg.window.resolved(), cfg.k.resolved())
    }

    fn source_fields(cfg: &HampelConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        let _ = cfg;
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for HampelConfig {
    fn defaults() -> Self {
        HampelConfig { window: Param::Solo(25), k: Param::Solo(3.0) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // window → range(2,4048,1)
        s.k = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier (sigma-multiplier)
        s
    }
}


impl Render for HampelFilter {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Hampel, "Hampel Filter", Color::hex(0x4CAF50))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hampel_creation() {
        let hf = HampelFilter::new(10, 3.0);
        assert!(!hf.is_ready());
        assert_eq!(hf.window(), 10);
        assert!((hf.k() - 3.0).abs() < 1e-9);
    }

    #[test]
    fn test_hampel_outlier_handling() {
        let mut hf = HampelFilter::new(10, 3.0);
        // Fill with normal values
        for i in 0..10 {
            hf.feed(100.0 + (i as f64 * 0.1));
        }
        assert!(hf.is_ready());
        // Add an outlier
        let outlier_result = hf.feed(200.0);
        // The filter should clamp the outlier
        assert!(outlier_result < 200.0, "Hampel should clamp outliers");
    }

    #[test]
    fn test_hampel_finite() {
        let mut hf = HampelFilter::new(10, 3.0);
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.5).sin() * 10.0;
            let value = hf.feed(price);
            assert!(value.is_finite(), "Hampel should always be finite");
        }
    }

    #[test]
    fn factory_feeds_resolved_hampel() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<HampelFilter as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Hampel(cfg).build_solo().unwrap();
        for i in 1..=30 {
            let price = 100.0 + (i as f64 * 0.1);
            f.feed(0, MarketSample::Bar {
                open: price, high: 9999.0, low: 0.0, close: price, volume: 1.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite());
    }

    #[test]
    fn test_hampel_reset() {
        let mut hf = HampelFilter::new(10, 3.0);
        for i in 1..=20 {
            hf.feed(100.0 + i as f64);
        }
        assert!(hf.is_ready());
        hf.reset();
        assert!(!hf.is_ready());
    }
}
