// TRIMA Bands: TRIMA +/- k * std

use crate::indicators::average::trima::Trima;
use crate::engine::ohlcv_field::OhlcvField;
#[derive(Debug, Clone)]
pub struct TrimaBands {
    trima: Trima,
    window: usize,
    k: f64,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl Default for TrimaBands {
    fn default() -> Self {
        Self::with_source(14, 2.0, OhlcvField::Close)
    }
}

impl TrimaBands {
    pub fn new(period: usize, k: f64) -> Self {
        Self {
            trima: Trima::new(period.max(2)),
            window: period.clamp(2, 512),
            k: if k > 0.0 { k } else { 2.0 },
            buf: Vec::with_capacity(period.clamp(2, 512)),
            idx: 0,
            filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }

    pub fn with_source(period: usize, k: f64, _source: OhlcvField) -> Self {
        // `_source` is config-only (factory resolves it via `const SOURCE`); runtime keeps none.
        Self {
            trima: Trima::new(period.max(2)),
            window: period.clamp(2, 512),
            k: if k > 0.0 { k } else { 2.0 },
            buf: Vec::with_capacity(period.clamp(2, 512)),
            idx: 0,
            filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.trima.reset();
        self.buf.clear();
        self.idx = 0;
        self.filled = false;
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.trima.is_ready()
    }

    #[inline]
    pub fn value_tuple(&self) -> (f64, f64, f64) {
        (self.upper, self.middle, self.lower)
    }

    pub fn upper(&self) -> f64 { self.upper }
    pub fn middle(&self) -> f64 { self.middle }
    pub fn lower(&self) -> f64 { self.lower }
    /// Feed a scalar — the resolved price (const SOURCE = Field{Close}).
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
        self.middle = self.trima.feed(price);
        if self.is_ready() {
            let mean = self.buf.iter().sum::<f64>() / (self.window as f64);
            let var = self
                .buf
                .iter()
                .map(|&x| {
                    let d = x - mean;
                    d * d
                })
                .sum::<f64>()
                / (self.window as f64);
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
    Cost, Family, Indicator, Output, Port, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`TrimaBands`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct TrimaBandsConfig {
    pub period: Param<usize>,
    pub k: Param<f64>,
    pub source: Param<OhlcvField>,
}

impl Indicator for TrimaBands {
    const ID: IndicatorId = IndicatorId::Trimabands;
    /// Channel family — TRIMA-centered std-dev band channel.
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(period) per bar: price buffer rescan. The embedded TRIMA charges via Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)], // buf for std dev
        inner: &[Port::new(IndicatorId::Trima, &[IndicatorOutputId::Trima])],
    };
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::TrimabandsUpper),
        Output::price(IndicatorOutputId::TrimabandsMiddle),
        Output::price(IndicatorOutputId::TrimabandsLower),
    ];
    type Config = TrimaBandsConfig;
    type Runtime = TrimaBands;

    fn create(cfg: TrimaBandsConfig) -> TrimaBands {
        TrimaBands::with_source(cfg.period.resolved(), cfg.k.resolved(), cfg.source.resolved())
    }

    fn source_fields(cfg: &TrimaBandsConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }
}

impl crate::contract::Config for TrimaBandsConfig {
    fn defaults() -> Self {
        TrimaBandsConfig {
            period: Param::Solo(14),
            k: Param::Solo(2.0),
            source: Param::Solo(OhlcvField::Close),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1), source→all 8
        s.k = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for TrimaBands {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::TrimabandsUpper, "Upper", Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::TrimabandsMiddle, "Middle", Color::hex(0x9E9E9E), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::TrimabandsLower, "Lower", Color::hex(0x4CAF50), 1.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests_contract {
    use super::*;

    #[test]
    fn factory_feeds_resolved_trimabands() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<TrimaBands as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Trimabands(cfg).build_solo().unwrap();
        for i in 1..=25usize {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 0.0,
                close: price,
                volume: 1000.0,
            });
        }
        let v = f.primary();
        assert!(v.is_finite(), "trimabands middle should be finite, got {v}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trima_bands_creation() {
        let tb = TrimaBands::new(20, 2.0);
        assert!(!tb.is_ready());
        assert_eq!(tb.upper, 0.0);
        assert_eq!(tb.lower, 0.0);
    }

    #[test]
    fn test_trima_bands_warmup() {
        let mut tb = TrimaBands::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            tb.feed(price);
        }
        assert!(tb.is_ready());
    }

    #[test]
    fn test_trima_bands_values() {
        let mut tb = TrimaBands::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            tb.feed(price);
        }
        assert!(tb.upper >= tb.middle);
        assert!(tb.middle >= tb.lower);
    }

    #[test]
    fn test_trima_bands_reset() {
        let mut tb = TrimaBands::new(20, 2.0);
        for i in 0..25 {
            tb.feed(100.0 + i as f64);
        }
        tb.reset();
        assert!(!tb.is_ready());
        assert_eq!(tb.upper, 0.0);
        assert_eq!(tb.lower, 0.0);
    }
}
