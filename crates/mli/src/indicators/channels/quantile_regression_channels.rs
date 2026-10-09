// Quantile Regression Channels - robust channel via quantile lines
// Placeholder: compute rolling median and MAD-based bands as proxy

use crate::indicators::utils::math::percentile::median;
#[derive(Debug, Clone)]
pub struct QuantileRegressionChannels {
    window: usize,
    k: f64,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl Default for QuantileRegressionChannels {
    fn default() -> Self {
        Self::new(14, 2.0)
    }
}

impl QuantileRegressionChannels {
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

    /// Feed one pre-extracted scalar price (close) — the contract feed path.
    pub fn feed(&mut self, c: f64) -> (f64, f64, f64) {
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
            let mut v: Vec<f64> = self.buf.iter().copied().collect();
            let mid = median(&mut v);
            self.middle = mid;
            let mut dev: Vec<f64> = self.buf.iter().map(|x| (x - mid).abs()).collect();
            let mad = median(&mut dev);
            let sigma = 1.4826 * mad;
            self.upper = mid + self.k * sigma;
            self.lower = mid - self.k * sigma;
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

/// Typed dual-mode config for [`QuantileRegressionChannels`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct QrchanConfig {
    pub period: Param<usize>,
    /// MAD-sigma multiplier for the bands (1.4826 * MAD * k).
    pub k: Param<f64>,
}

impl Indicator for QuantileRegressionChannels {
    const ID: IndicatorId = IndicatorId::Qrchan;
    /// Channel family — robust median/MAD regression channel.
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Close only (fixed) — the algorithm operates only on close prices.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::Field { default: crate::engine::ohlcv_field::OhlcvField::Close });
    /// O(period) per bar: median + MAD quickselect. One period-deep Vec.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::QrchanUpper),
        Output::price(IndicatorOutputId::QrchanMiddle),
        Output::price(IndicatorOutputId::QrchanLower),
    ];
    type Config = QrchanConfig;
    type Runtime = QuantileRegressionChannels;

    fn create(cfg: QrchanConfig) -> QuantileRegressionChannels {
        QuantileRegressionChannels::new(cfg.period.resolved(), cfg.k.resolved())
    }
}

impl crate::contract::Config for QrchanConfig {
    fn defaults() -> Self {
        QrchanConfig {
            period: Param::Solo(14),
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


impl Render for QuantileRegressionChannels {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::QrchanUpper, "Upper", Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::QrchanMiddle, "Middle", Color::hex(0x9E9E9E), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::QrchanLower, "Lower", Color::hex(0x4CAF50), 1.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantile_regression_channels_creation() {
        let qrc = QuantileRegressionChannels::new(20, 2.0);
        assert!(!qrc.is_ready());
        assert_eq!(qrc.upper, 0.0);
        assert_eq!(qrc.lower, 0.0);
    }

    #[test]
    fn test_quantile_regression_channels_warmup() {
        let mut qrc = QuantileRegressionChannels::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            qrc.feed(price);
        }
        assert!(qrc.is_ready());
    }

    #[test]
    fn test_quantile_regression_channels_values() {
        let mut qrc = QuantileRegressionChannels::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            qrc.feed(price);
        }
        assert!(qrc.upper >= qrc.middle);
        assert!(qrc.middle >= qrc.lower);
    }

    #[test]
    fn test_quantile_regression_channels_reset() {
        let mut qrc = QuantileRegressionChannels::new(20, 2.0);
        for i in 0..25 {
            qrc.feed(100.0 + i as f64);
        }
        qrc.reset();
        assert!(!qrc.is_ready());
        assert_eq!(qrc.upper, 0.0);
        assert_eq!(qrc.lower, 0.0);
    }

    /// Factory resolves close via default Source; wild high/low are ignored.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<QuantileRegressionChannels as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Qrchan(cfg).build_solo().unwrap();
        for i in 0..20usize {
            let close = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: 9999.0, low: 9999.0, close, volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        assert!(f.primary().is_finite(), "upper should be finite, got {}", f.primary());
    }
}
