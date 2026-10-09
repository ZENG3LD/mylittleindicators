// Projection Bands - project slope and add std-based bands

#[derive(Debug, Clone)]
pub struct ProjectionBands {
    window: usize,
    k: f64,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl Default for ProjectionBands {
    fn default() -> Self {
        Self::new(50, 2.0)
    }
}

impl ProjectionBands {
    pub fn new(window: usize, k: f64) -> Self {
        Self {
            window: window.clamp(2, 512),
            k: if k > 0.0 { k } else { 2.0 },
            buf: Vec::with_capacity(window.clamp(2, 512)),
            idx: 0,
            filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    #[inline]
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper, self.middle, self.lower)
    }

    pub fn upper(&self) -> f64 { self.upper }
    pub fn middle(&self) -> f64 { self.middle }
    pub fn lower(&self) -> f64 { self.lower }

    /// Feed one pre-extracted scalar price — the contract feed path.
    pub fn feed(&mut self, price: f64) -> (f64, f64, f64) {
        if self.buf.len() < self.window {
            self.buf.push(price);
            if self.buf.len() == self.window {
                self.filled = true;
            }
        } else {
            self.buf[self.idx] = price;
        }
        self.idx = (self.idx + 1) % self.window;
        if self.is_ready() {
            let n = self.window as f64;
            let mean = self.buf.iter().sum::<f64>() / n;
            let mut sxx = 0.0;
            let mut sxy = 0.0;
            let mut sx = 0.0;
            for (i, &y) in self.buf.iter().enumerate() {
                let x = i as f64;
                sx += x;
                sxx += x * x;
                sxy += x * (y - mean);
            }
            let den = (n * sxx - sx * sx).max(1e-9);
            let slope = sxy / den;
            let a = mean;
            self.middle = a + slope * (n - 1.0);
            let var = self.buf.iter().map(|&p| (p - mean) * (p - mean)).sum::<f64>() / n.max(1.0);
            let sd = var.sqrt();
            self.upper = self.middle + self.k * sd;
            self.lower = self.middle - self.k * sd;
        }
        (self.upper, self.middle, self.lower)
    }
}

// ---- Indicator contract ----

use crate::contract::{Param, sweep_f64};
use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::contract::{
    Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`ProjectionBands`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct ProjectionBandsConfig {
    pub period: Param<usize>,
    pub k: Param<f64>,
}

impl Indicator for ProjectionBands {
    const ID: IndicatorId = IndicatorId::Projbands;
    /// Channel family — linear-regression projection band.
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::Field { default: crate::engine::ohlcv_field::OhlcvField::Close });
    /// O(period) per bar: LR slope rescan. One period-deep Vec buffer.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::ProjbandsUpper),
        Output::price(IndicatorOutputId::ProjbandsMiddle),
        Output::price(IndicatorOutputId::ProjbandsLower),
    ];
    type Config = ProjectionBandsConfig;
    type Runtime = ProjectionBands;

    fn create(cfg: ProjectionBandsConfig) -> ProjectionBands {
        ProjectionBands::new(cfg.period.resolved(), cfg.k.resolved())
    }
}

impl crate::contract::Config for ProjectionBandsConfig {
    /// Period 50, k 2.0.
    fn defaults() -> Self {
        ProjectionBandsConfig {
            period: Param::Solo(50),
            k: Param::Solo(2.0),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1)
        s.k = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for ProjectionBands {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::ProjbandsUpper, "Upper", Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::ProjbandsMiddle, "Middle", Color::hex(0x9E9E9E), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::ProjbandsLower, "Lower", Color::hex(0x4CAF50), 1.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projection_bands_creation() {
        let pb = ProjectionBands::new(20, 2.0);
        assert!(!pb.is_ready());
        assert_eq!(pb.upper, 0.0);
        assert_eq!(pb.lower, 0.0);
    }

    #[test]
    fn test_projection_bands_warmup() {
        let mut pb = ProjectionBands::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            pb.feed(price);
        }
        assert!(pb.is_ready());
    }

    #[test]
    fn test_projection_bands_values() {
        let mut pb = ProjectionBands::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            pb.feed(price);
        }
        assert!(pb.upper >= pb.middle);
        assert!(pb.middle >= pb.lower);
    }

    #[test]
    fn test_projection_bands_reset() {
        let mut pb = ProjectionBands::new(20, 2.0);
        for i in 0..25 {
            pb.feed(100.0 + i as f64);
        }
        pb.reset();
        assert!(!pb.is_ready());
        assert_eq!(pb.upper, 0.0);
        assert_eq!(pb.lower, 0.0);
    }

    /// Factory resolves configurable source (default close); wild high/low are ignored.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Projbands(<<ProjectionBands as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..55usize {
            let close = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 9999.0, close, volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite(), "upper should be finite, got {}", f.primary());
    }
}
