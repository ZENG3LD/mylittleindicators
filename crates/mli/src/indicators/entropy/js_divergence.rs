// Rolling Jensen-Shannon divergence between two adjacent halves of a window

#[derive(Debug, Clone)]
pub struct JSDivergence {
    window: usize,
    bins: usize,
    clip_abs: f64,
    rets: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    value: f64,
}

impl JSDivergence {
    pub fn new(window: usize, bins: usize, clip_abs: f64) -> Self {
        let w = window.max(10);
        Self {
            window: w,
            bins: bins.max(8),
            clip_abs: clip_abs.max(1e-6),
            rets: vec![0.0; w],
            idx: 0,
            filled: false,
            last_close: None,
            value: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.last_close = None;
        self.rets.fill(0.0);
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    #[inline]
    fn bin_index(&self, r: f64) -> usize {
        let rr = r.max(-self.clip_abs).min(self.clip_abs);
        let x = (rr + self.clip_abs) / (2.0 * self.clip_abs);
        (x * self.bins as f64)
            .floor()
            .clamp(0.0, (self.bins - 1) as f64) as usize
    }

    #[inline]
    fn kl(p: &[f64], q: &[f64]) -> f64 {
        let mut s = 0.0;
        for i in 0..p.len() {
            if p[i] > 0.0 && q[i] > 0.0 {
                s += p[i] * (p[i] / q[i]).ln();
            }
        }
        s
    }

    /// Feed the close scalar (resolved by the factory; `const SOURCE = Field{Close}`).
    pub fn feed(&mut self, close: f64) -> f64 {
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            self.rets[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if self.idx == 0 {
                self.filled = true;
            }
            if self.filled {
                let half = self.window / 2;
                let mut p = vec![1e-12; self.bins];
                let mut q = vec![1e-12; self.bins];
                for i in 0..half {
                    let r = self.rets[(self.idx + i) % self.window];
                    p[self.bin_index(r)] += 1.0;
                }
                for i in half..self.window {
                    let r = self.rets[(self.idx + i) % self.window];
                    q[self.bin_index(r)] += 1.0;
                }
                let ps: f64 = p.iter().sum();
                let qs: f64 = q.iter().sum();
                for v in &mut p {
                    *v /= ps;
                }
                for v in &mut q {
                    *v /= qs;
                }
                let mut m = vec![0.0; self.bins];
                for i in 0..self.bins {
                    m[i] = 0.5 * (p[i] + q[i]);
                }
                let js = 0.5 * Self::kl(&p, &m) + 0.5 * Self::kl(&q, &m);
                self.value = js.max(0.0);
            }
        }
        self.last_close = Some(close);
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

}

impl Default for JSDivergence {
    /// Factory default (Jsd arm): window=200, bins=16, clip_abs=0.05.
    fn default() -> Self {
        Self::new(200, 16, 0.05)
    }
}

// ── Contract ──────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, StoreKind, Store, UpdateComplexity};
use crate::engine::indicator_id::IndicatorId;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

use crate::contract::Param;
use crate::contract::axis::sweep_f64;

/// Typed config for [`JSDivergence`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct JsdConfig {
    pub window: Param<usize>,
    pub bins: Param<usize>,
    pub clip_abs: Param<f64>,
}

impl Indicator for JSDivergence {
    const ID: IndicatorId = IndicatorId::Jsd;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Jsd)];
    type Config = JsdConfig;
    type Runtime = JSDivergence;

    fn create(cfg: JsdConfig) -> JSDivergence {
        JSDivergence::new(cfg.window.resolved(), cfg.bins.resolved(), cfg.clip_abs.resolved())
    }
}

impl crate::contract::Config for JsdConfig {
    fn defaults() -> Self {
        JsdConfig {
            window: Param::Solo(200),
            bins: Param::Solo(16),
            clip_abs: Param::Solo(0.05),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // bins: Class B (entropy histogram) — 4..=64 step 2, corrected from auto 2..=4048
        s.bins = Param::many((4usize..=64).step_by(2).collect());
        // clip_abs: Class F (entropy normalization) — sweep_f64(0.05, 0.5, 0.05)
        s.clip_abs = Param::many(sweep_f64(0.05, 0.5, 0.05));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for JSDivergence {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Jsd, "JS Divergence", Color::hex(0x009688))
            .bounds(0.0, 1.0)
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::market_sample::MarketSample;
    use crate::engine::contract_engine::IndicatorOrder;

    #[test]
    fn test_js_divergence_creation() {
        let jsd = JSDivergence::new(20, 10, 0.05);
        assert!(!jsd.is_ready());
        assert_eq!(jsd.value(), 0.0);
    }

    #[test]
    fn test_js_divergence_warmup() {
        let mut jsd = JSDivergence::new(15, 10, 0.05);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            jsd.feed(price);
        }
        assert!(jsd.is_ready());
    }

    #[test]
    fn test_js_divergence_values_finite() {
        let mut jsd = JSDivergence::new(15, 10, 0.05);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = jsd.feed(price);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_js_divergence_values_non_negative() {
        let mut jsd = JSDivergence::new(15, 10, 0.05);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = jsd.feed(price);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_js_divergence_reset() {
        let mut jsd = JSDivergence::new(15, 10, 0.05);
        for i in 0..25 {
            jsd.feed(100.0 + i as f64);
        }
        jsd.reset();
        assert!(!jsd.is_ready());
        assert_eq!(jsd.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_jsd() {
        
        let mut f = IndicatorOrder::Jsd(<<JSDivergence as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..220 {
            let price = 100.0 + (i as f64 * 0.12).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        let v = f.read(IndicatorOutputId::Jsd);
        assert!(v.is_finite() && v >= 0.0, "jsd must be finite >= 0: {v}");
    }
}
