// Half-Life of Mean Reversion (Ornstein–Uhlenbeck proxy) from AR(1) phi: hl = -ln(2)/ln(phi)

#[derive(Debug, Clone)]
pub struct HalfLifeMr {
    window: usize,
    vals: Vec<f64>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    pub half_life: f64,
}

impl HalfLifeMr {
    pub fn new(window: usize) -> Self {
        let w = window.max(20);
        Self {
            window: w,
            vals: vec![0.0; w],
            idx: 0,
            filled: false,
            last_close: None,
            half_life: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.idx = 0;
        self.filled = false;
        self.vals.fill(0.0);
        self.last_close = None;
        self.half_life = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.half_life
    }

    /// Feed one pre-extracted close price.
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
            self.half_life = self.compute_hl();
        }
        self.half_life
    }

    fn compute_hl(&self) -> f64 {
        let n = self.window;
        let mut sx = 0.0;
        let mut sy = 0.0;
        let mut sxx = 0.0;
        let mut sxy = 0.0;
        let mut count = 0.0;
        for i in 1..n {
            let y = self.vals[(self.idx + i) % n];
            let x = self.vals[(self.idx + i - 1) % n];
            sx += x;
            sy += y;
            sxx += x * x;
            sxy += x * y;
            count += 1.0;
        }
        let denom = count * sxx - sx * sx;
        let phi = if denom.abs() > 1e-12 {
            (count * sxy - sx * sy) / denom
        } else {
            0.0
        };
        if phi > 0.0 && phi < 1.0 {
            (-(2.0_f64).ln() / phi.ln()).max(0.0)
        } else {
            f64::INFINITY
        }
    }
}

impl Default for HalfLifeMr {
    /// Factory defaults: window=100 (clamped to max(100,20)=100).
    fn default() -> Self {
        Self::new(100)
    }
}

// ── Contract ─────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderSpec, SourceAxis, Store,
    StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Own config for [`HalfLifeMr`] — period-only, no smoother slot.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct HalfLifeMrConfig {
    pub period: Param<usize>,
}

impl Indicator for HalfLifeMr {
    const ID: IndicatorId = IndicatorId::HalfLifeMr;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const OUTPUTS: &'static [Output] = &[Output::count(IndicatorOutputId::HalfLifeMr)];
    /// O(n): single-pass AR(1) OLS over the return window.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);

    type Config = HalfLifeMrConfig;
    type Runtime = HalfLifeMr;

    fn create(cfg: HalfLifeMrConfig) -> HalfLifeMr {
        HalfLifeMr::new(cfg.period.resolved())
    }

    fn source_fields(_cfg: &HalfLifeMrConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [OhlcvField::Close].into_iter().collect()
    }
}

impl crate::contract::Config for HalfLifeMrConfig {
    fn defaults() -> Self {
        HalfLifeMrConfig { period: Param::Solo(100) }
    }
    fn machine_defaults() -> Self {
        // period: Class A → auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for HalfLifeMr {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::HalfLifeMr, "Half Life MR", Color::hex(0x00BCD4))
            .precision(2)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::indicator_id::IndicatorId;
    use crate::contract::Indicator;

    #[test]
    fn test_half_life_mr_creation() {
        let hlmr = HalfLifeMr::new(50);
        assert!(!hlmr.is_ready());
        assert_eq!(hlmr.half_life, 0.0);
    }

    #[test]
    fn test_half_life_mr_warmup() {
        let mut hlmr = HalfLifeMr::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            hlmr.feed(price);
        }
        assert!(hlmr.is_ready());
    }

    #[test]
    fn test_half_life_mr_values() {
        let mut hlmr = HalfLifeMr::new(50);
        for i in 0..60 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = hlmr.feed(price);
            assert!(value >= 0.0 || value.is_infinite(), "Half-life should be non-negative or inf");
        }
    }

    #[test]
    fn test_half_life_mr_reset() {
        let mut hlmr = HalfLifeMr::new(50);
        for i in 0..60 {
            hlmr.feed(100.0 + i as f64);
        }
        hlmr.reset();
        assert!(!hlmr.is_ready());
        assert_eq!(hlmr.half_life, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_half_life_mr() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::HalfLifeMr(<<HalfLifeMr as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..150 {
            let p = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar { open: 9999.0, high: 9999.0, low: 9999.0, close: p, volume: 1.0 });
        }
        assert!(f.read(IndicatorOutputId::HalfLifeMr) >= 0.0 || f.read(IndicatorOutputId::HalfLifeMr).is_infinite());
    }

    #[test]
    fn contract_id_matches() {
        assert_eq!(<HalfLifeMr as Indicator>::ID, IndicatorId::HalfLifeMr);
    }
}
