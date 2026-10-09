// Conditional Entropy H(Y|X) proxy using discrete bins for r_t (Y) and r_{t-1} (X)

#[derive(Debug, Clone)]
pub struct ConditionalEntropy {
    window: usize,
    bins: usize,
    clip_abs: f64,
    rx: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    value: f64,
}

impl ConditionalEntropy {
    pub fn new(window: usize, bins: usize, clip_abs: f64) -> Self {
        let w = window.max(20);
        let b = bins.max(4);
        Self {
            window: w,
            bins: b,
            clip_abs: clip_abs.max(1e-6),
            rx: vec![0.0; w],
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
        self.rx.fill(0.0);
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    #[inline]
    fn bin(&self, r: f64) -> usize {
        let rr = r.max(-self.clip_abs).min(self.clip_abs);
        let x = (rr + self.clip_abs) / (2.0 * self.clip_abs);
        (x * self.bins as f64)
            .floor()
            .clamp(0.0, (self.bins - 1) as f64) as usize
    }

    /// Feed the close scalar (resolved by the factory; `const SOURCE = Field{Close}`).
    pub fn feed(&mut self, close: f64) -> f64 {
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            self.rx[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if self.idx == 0 {
                self.filled = true;
            }
            if self.filled {
                self.value = self.compute();
            }
        }
        self.last_close = Some(close);
        self.value
    }

    fn compute(&self) -> f64 {
        let n = self.window;
        let b = self.bins;
        let mut joint = vec![0usize; b * b];
        let mut px = vec![0usize; b];
        for t in 1..n {
            let y = self.bin(self.rx[(self.idx + t) % n]);
            let x = self.bin(self.rx[(self.idx + t - 1) % n]);
            joint[y * b + x] += 1;
            px[x] += 1;
        }
        let total = (n - 1) as f64;
        let mut h = 0.0;
        for x in 0..b {
            if px[x] == 0 {
                continue;
            }
            let p_x = (px[x] as f64) / total;
            let mut h_y_given_x = 0.0;
            for y in 0..b {
                let c = joint[y * b + x] as f64;
                if c > 0.0 {
                    let p_yx = c / (px[x] as f64);
                    h_y_given_x -= p_yx * p_yx.ln();
                }
            }
            h += p_x * h_y_given_x;
        }
        h.max(0.0)
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

}

impl Default for ConditionalEntropy {
    /// Factory default (Conden arm): window=100, bins=8, clip_abs=0.05.
    fn default() -> Self {
        Self::new(100, 8, 0.05)
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

/// Typed config for [`ConditionalEntropy`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct CondenConfig {
    pub window: Param<usize>,
    pub bins: Param<usize>,
    pub clip_abs: Param<f64>,
}

impl Indicator for ConditionalEntropy {
    const ID: IndicatorId = IndicatorId::Conden;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed close — no configurable source.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Conden)];
    type Config = CondenConfig;
    type Runtime = ConditionalEntropy;

    fn create(cfg: CondenConfig) -> ConditionalEntropy {
        ConditionalEntropy::new(cfg.window.resolved(), cfg.bins.resolved(), cfg.clip_abs.resolved())
    }
}

impl crate::contract::Config for CondenConfig {
    fn defaults() -> Self {
        CondenConfig {
            window: Param::Solo(100),
            bins: Param::Solo(8),
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


impl Render for ConditionalEntropy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Conden, "Conditional Entropy", Color::hex(0x9C27B0))
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
    fn test_conditional_entropy_creation() {
        let ce = ConditionalEntropy::new(30, 8, 0.05);
        assert!(!ce.is_ready());
        assert_eq!(ce.value(), 0.0);
    }

    #[test]
    fn test_conditional_entropy_warmup() {
        let mut ce = ConditionalEntropy::new(20, 8, 0.05);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ce.feed(price);
        }
        assert!(ce.is_ready());
    }

    #[test]
    fn test_conditional_entropy_values_finite() {
        let mut ce = ConditionalEntropy::new(20, 8, 0.05);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ce.feed(price);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_conditional_entropy_reset() {
        let mut ce = ConditionalEntropy::new(20, 8, 0.05);
        for i in 0..30 {
            ce.feed(100.0 + i as f64);
        }
        ce.reset();
        assert!(!ce.is_ready());
        assert_eq!(ce.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_conden() {
        
        let mut f = IndicatorOrder::Conden(<<ConditionalEntropy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..120 {
            let price = 100.0 + (i as f64 * 0.12).sin() * 7.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        let v = f.read(IndicatorOutputId::Conden);
        assert!(v.is_finite() && v >= 0.0, "conden must be finite >= 0: {v}");
    }
}
