// Rolling KL divergence between two adjacent halves of a window of log-returns

#[derive(Debug, Clone)]
pub struct KLDivergence {
    window: usize,
    bins: usize,
    clip_abs: f64,
    rets: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    value: f64,
}

impl KLDivergence {
    pub fn new(window: usize, bins: usize, clip_abs: f64) -> Self {
        let w = window.max(10) | 1; // ensure odd? use |1 to make odd; but we will split into halves truncating
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
                // first half
                for i in 0..half {
                    let r = self.rets[(self.idx + i) % self.window];
                    p[self.bin_index(r)] += 1.0;
                }
                // second half
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
                let mut kl = 0.0;
                for b in 0..self.bins {
                    let pi = p[b];
                    let qi = q[b];
                    if pi > 0.0 && qi > 0.0 {
                        kl += pi * (pi / qi).ln();
                    }
                }
                self.value = kl.max(0.0);
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

impl Default for KLDivergence {
    /// Factory default (Kld arm): window=200, bins=16, clip_abs=0.05.
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

/// Typed config for [`KLDivergence`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct KldConfig {
    pub window: Param<usize>,
    pub bins: Param<usize>,
    pub clip_abs: Param<f64>,
}

impl Indicator for KLDivergence {
    const ID: IndicatorId = IndicatorId::Kld;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Kld)];
    type Config = KldConfig;
    type Runtime = KLDivergence;

    fn create(cfg: KldConfig) -> KLDivergence {
        KLDivergence::new(cfg.window.resolved(), cfg.bins.resolved(), cfg.clip_abs.resolved())
    }
}

impl crate::contract::Config for KldConfig {
    fn defaults() -> Self {
        KldConfig {
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


impl Render for KLDivergence {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Kld, "KL Divergence", Color::hex(0xFF9800))
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
    fn test_kl_divergence_creation() {
        let kld = KLDivergence::new(20, 10, 0.05);
        assert!(!kld.is_ready());
        assert_eq!(kld.value(), 0.0);
    }

    #[test]
    fn test_kl_divergence_warmup() {
        let mut kld = KLDivergence::new(15, 10, 0.05);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            kld.feed(price);
        }
        assert!(kld.is_ready());
    }

    #[test]
    fn test_kl_divergence_values_finite() {
        let mut kld = KLDivergence::new(15, 10, 0.05);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kld.feed(price);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_kl_divergence_values_non_negative() {
        let mut kld = KLDivergence::new(15, 10, 0.05);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = kld.feed(price);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_kl_divergence_reset() {
        let mut kld = KLDivergence::new(15, 10, 0.05);
        for i in 0..25 {
            kld.feed(100.0 + i as f64);
        }
        kld.reset();
        assert!(!kld.is_ready());
        assert_eq!(kld.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_kld() {
        
        let mut f = IndicatorOrder::Kld(<<KLDivergence as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
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
        let v = f.read(IndicatorOutputId::Kld);
        assert!(v.is_finite() && v >= 0.0, "kld must be finite >= 0: {v}");
    }
}
