// Information Gain: IG(Y;X)=H(Y)-H(Y|X) with discrete bins on r_t (Y) and r_{t-1} (X)

#[derive(Debug, Clone)]
pub struct InformationGain {
    window: usize,
    bins: usize,
    clip_abs: f64,
    vals: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    pub value: f64,
}

impl InformationGain {
    pub fn new(window: usize, bins: usize, clip_abs: f64) -> Self {
        let w = window.max(20);
        let b = bins.max(4);
        Self {
            window: w,
            bins: b,
            clip_abs: clip_abs.max(1e-6),
            vals: vec![0.0; w],
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
        self.vals.fill(0.0);
        self.last_close = None;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed the close scalar (resolved by the factory; `const SOURCE = Field{Close}`).
    pub fn feed(&mut self, c: f64) -> f64 {
        if let Some(prev) = self.last_close {
            let r = (c / prev).ln();
            self.vals[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if !self.filled && self.idx == 0 {
                self.filled = true;
            }
        }
        self.last_close = Some(c);
        if self.filled {
            self.value = self.compute_ig();
        }
        self.value
    }

    fn compute_ig(&self) -> f64 {
        let n = self.window;
        if n < 3 {
            return 0.0;
        }
        let mut hx = vec![0usize; self.bins];
        let mut hy = vec![0usize; self.bins];
        let mut hxy = vec![0usize; self.bins * self.bins];
        let clip = self.clip_abs;
        let min = -clip;
        let mut max = clip;
        if min == max {
            max = min + 1e-6;
        }
        let xi = |v: f64| -> usize {
            let t = ((v.min(max).max(min) - min) / (max - min) * (self.bins as f64)) as usize;
            t.min(self.bins - 1)
        };
        for i in 1..n {
            let y = self.vals[(self.idx + i) % n];
            let x = self.vals[(self.idx + i - 1) % n];
            let bx = xi(x);
            let by = xi(y);
            hx[bx] += 1;
            hy[by] += 1;
            hxy[by * self.bins + bx] += 1;
        }
        let total = (n - 1) as f64;
        let mut h_y = 0.0;
        for c in hy {
            if c > 0 {
                let p = c as f64 / total;
                h_y -= p * p.ln();
            }
        }
        let mut h_yx = 0.0;
        for by in 0..self.bins {
            let mut row_sum = 0usize;
            for bx in 0..self.bins {
                row_sum += hxy[by * self.bins + bx];
            }
            if row_sum > 0 {
                let denom = row_sum as f64;
                for bx in 0..self.bins {
                    let c = hxy[by * self.bins + bx];
                    if c > 0 {
                        let p = c as f64 / denom;
                        h_yx -= (denom / total) * p * p.ln();
                    }
                }
            }
        }
        (h_y - h_yx).max(0.0)
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

}

impl Default for InformationGain {
    /// Factory default (Infog arm): window=100, bins=8, clip_abs=0.05.
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

/// Typed config for [`InformationGain`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct InfogConfig {
    pub window: Param<usize>,
    pub bins: Param<usize>,
    pub clip_abs: Param<f64>,
}

impl Indicator for InformationGain {
    const ID: IndicatorId = IndicatorId::Infog;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed close — computes log-returns internally.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Infog)];
    type Config = InfogConfig;
    type Runtime = InformationGain;

    fn create(cfg: InfogConfig) -> InformationGain {
        InformationGain::new(cfg.window.resolved(), cfg.bins.resolved(), cfg.clip_abs.resolved())
    }
}

impl crate::contract::Config for InfogConfig {
    fn defaults() -> Self {
        InfogConfig {
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


impl Render for InformationGain {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Infog, "Info Gain", Color::hex(0x2196F3))
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
    fn test_information_gain_creation() {
        let ig = InformationGain::new(30, 8, 0.05);
        assert!(!ig.is_ready());
        assert_eq!(ig.value, 0.0);
    }

    #[test]
    fn test_information_gain_warmup() {
        let mut ig = InformationGain::new(20, 8, 0.05);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ig.feed(price);
        }
        assert!(ig.is_ready());
    }

    #[test]
    fn test_information_gain_values_finite() {
        let mut ig = InformationGain::new(20, 8, 0.05);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = ig.feed(price);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_information_gain_reset() {
        let mut ig = InformationGain::new(20, 8, 0.05);
        for i in 0..30 {
            ig.feed(100.0 + i as f64);
        }
        ig.reset();
        assert!(!ig.is_ready());
        assert_eq!(ig.value, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_infog() {
        
        let mut f = IndicatorOrder::Infog(<<InformationGain as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
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
        let v = f.read(IndicatorOutputId::Infog);
        assert!(v.is_finite() && v >= 0.0, "infog must be finite >= 0: {v}");
    }
}
