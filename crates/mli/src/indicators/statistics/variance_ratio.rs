// Rolling Lo-MacKinlay Variance Ratio test (simple m-step version) on returns

#[derive(Debug, Clone)]
pub struct VarianceRatio {
    window: usize,
    m: usize,
    vals: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    ratio: f64,
}

impl VarianceRatio {
    pub fn new(window: usize, m: usize) -> Self {
        let w = window.max(20);
        let step = m.max(2).min(w / 2);
        Self {
            window: w,
            m: step,
            vals: vec![0.0; w],
            idx: 0,
            filled: false,
            last_close: None,
            ratio: 1.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.vals.fill(0.0);
        self.last_close = None;
        self.ratio = 1.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed one pre-extracted close price.
    pub fn feed(&mut self, close: f64) -> f64 {
        if let Some(prev) = self.last_close {
            let r = (close / prev).ln();
            self.vals[self.idx] = r;
            self.idx = (self.idx + 1) % self.window;
            if self.idx == 0 {
                self.filled = true;
            }
            if self.filled {
                self.ratio = self.compute_ratio();
            }
        }
        self.last_close = Some(close);
        self.ratio
    }

    fn compute_ratio(&self) -> f64 {
        let n = self.window;
        // variance of 1-step returns
        let mut mean = 0.0;
        for i in 0..n {
            mean += self.vals[i];
        }
        mean /= n as f64;
        let mut var1 = 0.0;
        for i in 0..n {
            let d = self.vals[i] - mean;
            var1 += d * d;
        }
        var1 /= n as f64;
        if var1 <= 1e-12 {
            return 1.0;
        }
        // variance of m-step cumulative returns (overlapping)
        let m = self.m;
        let mut sums = Vec::with_capacity(n - m + 1);
        let mut cur = 0.0;
        for i in 0..m {
            cur += self.vals[(self.idx + i) % n];
        }
        sums.push(cur);
        for i in m..n {
            cur += self.vals[(self.idx + i) % n] - self.vals[(self.idx + i - m) % n];
            sums.push(cur);
        }
        let count = sums.len();
        let mut smean = 0.0;
        for v in &sums {
            smean += *v;
        }
        smean /= count as f64;
        let mut varm = 0.0;
        for v in &sums {
            let d = *v - smean;
            varm += d * d;
        }
        varm /= count as f64;
        (varm / var1) / (m as f64)
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.ratio
    }
}

impl Default for VarianceRatio {
    /// Factory defaults: window=100 (clamped to max(100,20)=100), m=5.
    fn default() -> Self {
        Self::new(100, 5)
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
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`VarianceRatio`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct VrConfig {
    pub period: Param<usize>,
    pub m: Param<usize>,
}

impl Indicator for VarianceRatio {
    const ID: IndicatorId = IndicatorId::Vr;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::ratio(IndicatorOutputId::Vr)];
    /// O(n): single pass variance + overlapping m-step sums.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);

    type Config = VrConfig;
    type Runtime = VarianceRatio;

    fn create(cfg: VrConfig) -> VarianceRatio {
        VarianceRatio::new(cfg.period.resolved(), cfg.m.resolved())
    }

    fn source_fields(_cfg: &VrConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for VrConfig {
    fn defaults() -> Self {
        VrConfig { period: Param::Solo(100), m: Param::Solo(5) }
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1).
        // m: VR step horizon (multi-step return length) — classified as A.lag-like.
        //    Range range(2,50,1): minimum 2 (degenerate at 1), capped at 50 (beyond window/2
        //    is meaningless; practical range is 2..20). Corrected off auto 2..4048.
        let mut s = Self::machine_defaults_auto();
        s.m = Param::range(2, 50, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VarianceRatio {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vr, "Variance Ratio", Color::hex(0x009688))
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
    fn test_variance_ratio_creation() {
        let vr = VarianceRatio::new(50, 5);
        assert!(!vr.is_ready());
        assert_eq!(vr.value(), 1.0);
    }

    #[test]
    fn test_variance_ratio_warmup() {
        let mut vr = VarianceRatio::new(50, 5);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            vr.feed(price);
        }
        assert!(vr.is_ready());
    }

    #[test]
    fn test_variance_ratio_positive() {
        let mut vr = VarianceRatio::new(50, 5);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = vr.feed(price);
            assert!(value > 0.0, "VR should be positive");
        }
    }

    #[test]
    fn test_variance_ratio_reset() {
        let mut vr = VarianceRatio::new(50, 5);
        for i in 0..60 {
            vr.feed(100.0 + i as f64);
        }
        vr.reset();
        assert!(!vr.is_ready());
        assert_eq!(vr.value(), 1.0);
    }

    #[test]
    fn factory_feeds_resolved_vr() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Vr(<<VarianceRatio as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..150 {
            let p = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: p, volume: 1.0 });
        }
        assert!(f.read(IndicatorOutputId::Vr) > 0.0);
    }

    #[test]
    fn contract_id_matches() {
        assert_eq!(<VarianceRatio as Indicator>::ID, IndicatorId::Vr);
    }
}
