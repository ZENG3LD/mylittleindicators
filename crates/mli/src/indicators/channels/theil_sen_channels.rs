// Theil–Sen Regression Channels — robust linear regression via median-of-slopes.
//
// Algorithm (O(N²) — intentional for correctness at window ≤ 256):
//   1. For all pairs (i, j) with i < j: slope_ij = (price[j] - price[i]) / (j - i)
//   2. slope = median of all slopes  (robust to outliers)
//   3. intercept = median of (price[i] - slope*i)  for all i
//   4. midline = intercept + slope*(N-1)  (end-of-window regression value)
//   5. spread = 1.4826 * MAD(residuals)  (robust σ estimate)
//   6. upper = midline + k*spread, lower = midline - k*spread
//
// Output: Channel3 { upper, middle, lower }

#[derive(Debug, Clone)]
pub struct TheilSenChannels {
    window: usize,
    k: f64,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl Default for TheilSenChannels {
    fn default() -> Self {
        Self::new(50, 2.0)
    }
}

impl TheilSenChannels {
    pub fn new(window: usize, k: f64) -> Self {
        Self {
            window: window.clamp(5, 10000),
            k: if k > 0.0 { k } else { 2.0 },
            buf: Vec::with_capacity(window.clamp(5, 10000)),
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
            let n = self.window;
            let mut slopes: Vec<f64> = Vec::with_capacity(n * n);
            for i in 0..n {
                for j in (i + 1)..n {
                    let dy = self.buf[j] - self.buf[i];
                    let dx = (j as f64 - i as f64).max(1e-9);
                    slopes.push(dy / dx);
                }
            }
            slopes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let slope = slopes[slopes.len() / 2];
            let mut intercepts: Vec<f64> = Vec::with_capacity(n);
            for i in 0..n {
                let x = i as f64;
                let y = self.buf[i];
                intercepts.push(y - slope * x);
            }
            intercepts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let a = intercepts[intercepts.len() / 2];
            let mid = a + slope * (n as f64 - 1.0);
            self.middle = mid;
            let mut dev: Vec<f64> = self
                .buf
                .iter()
                .enumerate()
                .map(|(i, &y)| (y - (a + slope * (i as f64))).abs())
                .collect();
            dev.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let mad = dev[dev.len() / 2];
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

/// Typed dual-mode config for [`TheilSenChannels`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct TheilsenConfig {
    pub period: Param<usize>,
    /// Robust sigma multiplier (1.4826 * MAD * k).
    pub k: Param<f64>,
}

impl Indicator for TheilSenChannels {
    const ID: IndicatorId = IndicatorId::Theilsenchan;
    /// Channel family — robust Theil-Sen regression channel.
    const FAMILY: &'static [Family] = &[Family::Channel];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Close only — Theil-Sen uses only the price series.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::Field { default: crate::engine::ohlcv_field::OhlcvField::Close });
    /// O(N²) per bar: all pairs of slopes + MAD spread. One period-deep Vec.
    const COST: Cost = Cost::new(UpdateComplexity::Quadratic, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::TheilsenchanUpper),
        Output::price(IndicatorOutputId::TheilsenchanMiddle),
        Output::price(IndicatorOutputId::TheilsenchanLower),
    ];
    type Config = TheilsenConfig;
    type Runtime = TheilSenChannels;

    fn create(cfg: TheilsenConfig) -> TheilSenChannels {
        TheilSenChannels::new(cfg.period.resolved(), cfg.k.resolved())
    }
}

impl crate::contract::Config for TheilsenConfig {
    fn defaults() -> Self {
        TheilsenConfig {
            period: Param::Solo(50),
            k: Param::Solo(2.0),
        }
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1); clamped 5..10000 in new()
        s.k = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for TheilSenChannels {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(IndicatorOutputId::TheilsenchanUpper, "Upper", Color::hex(0xF44336), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::TheilsenchanMiddle, "Middle", Color::hex(0x9E9E9E), 1.0))
            .output(RenderOutput::line(IndicatorOutputId::TheilsenchanLower, "Lower", Color::hex(0x4CAF50), 1.0))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theil_sen_channels_creation() {
        let tsc = TheilSenChannels::new(20, 2.0);
        assert!(!tsc.is_ready());
        assert_eq!(tsc.upper, 0.0);
        assert_eq!(tsc.lower, 0.0);
    }

    #[test]
    fn test_theil_sen_channels_warmup() {
        let mut tsc = TheilSenChannels::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            tsc.feed(price);
        }
        assert!(tsc.is_ready());
    }

    #[test]
    fn test_theil_sen_channels_values() {
        let mut tsc = TheilSenChannels::new(20, 2.0);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            tsc.feed(price);
        }
        assert!(tsc.upper >= tsc.middle);
        assert!(tsc.middle >= tsc.lower);
    }

    #[test]
    fn test_theil_sen_channels_reset() {
        let mut tsc = TheilSenChannels::new(20, 2.0);
        for i in 0..25 {
            tsc.feed(100.0 + i as f64);
        }
        tsc.reset();
        assert!(!tsc.is_ready());
        assert_eq!(tsc.upper, 0.0);
        assert_eq!(tsc.lower, 0.0);
    }

    /// Factory resolves close via default Source; wild high/low are ignored.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<TheilSenChannels as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Theilsenchan(cfg).build_solo().unwrap();
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
