// Realized Bipower Variance (RBV) - jump-robust volatility measure

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
#[derive(Debug, Clone)]
pub struct BipowerVariance {
    window: usize,
    abs_ret: Vec<f64>,
    idx: usize,
    filled: bool,
    prev_close: f64,
    value: f64,
}

impl BipowerVariance {
    pub fn new(window: usize) -> Self {
        Self {
            window: window.max(2),
            abs_ret: vec![0.0; window.max(2)],
            idx: 0,
            filled: false,
            prev_close: 0.0,
            value: 0.0,
        }
    }
    pub fn reset(&mut self) {
        self.abs_ret.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.prev_close = 0.0;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Feed ONE resolved scalar (close) — close-to-close realized estimator.
    pub fn feed(&mut self, c: f64) -> f64 {
        if self.prev_close <= 0.0 {
            self.prev_close = c.max(1e-12);
            return self.value;
        }
        let r = (c / self.prev_close).ln();
        self.prev_close = c.max(1e-12);
        let ar = r.abs();
        let _old = self.abs_ret[self.idx];
        self.abs_ret[self.idx] = ar;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        // RBV ~ (pi/2) * sum(|r_t||r_{t-1}|) / (N-1)
        if self.idx > 0 || self.filled {
            let n = if self.filled { self.window } else { self.idx };
            let mut s = 0.0;
            let len = if self.filled { self.window } else { self.idx };
            for i in 1..len {
                s += self.abs_ret[i] * self.abs_ret[i - 1];
            }
            let denom = (n as f64 - 1.0).max(1.0);
            // Annualize and scale for better readability (252 trading days)
            self.value = std::f64::consts::PI * 0.5 * (s / denom) * 252.0 * 10000.0;
        }
        self.value
    }
}

// -- contract -----------------------------------------------------------------

use crate::contract::{Param, Render};
use crate::contract::{Color, RenderSpec};

/// Typed dual-mode config for [`BipowerVariance`] — period only.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct BpvConfig {
    pub period: Param<usize>,
}

/// Period-deep ring of |r| values -- the rescan buffer.
static BPV_STORES: &[Store] = &[Store::window(StoreKind::Vec)];

impl Indicator for BipowerVariance {
    const ID: IndicatorId = IndicatorId::Bpv;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed close-to-close returns -- reads close intrinsically, not a swept field.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[OhlcvField::Close]));
    /// Reads only close -> default `Field(Close)` SOURCE applies.
    /// O(period): feed rescans abs_ret[0..n] each bar to compute the
    /// bipower sum -- no incremental form for |r_t||r_{t-1}| products.
    const COST: Cost = Cost::new(UpdateComplexity::Linear, BPV_STORES);
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Bpv)];
    type Config = BpvConfig;
    type Runtime = BipowerVariance;

    fn create(cfg: BpvConfig) -> BipowerVariance {
        BipowerVariance::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for BpvConfig {
    fn defaults() -> Self {
        BpvConfig { period: Param::Solo(30) }
    }
    fn machine_defaults() -> Self {
        // period: Class A usize — auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for BipowerVariance {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Bpv, "Bipower Var", Color::hex(0x9C27B0))
            .precision(6)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bipower_variance_creation() {
        let bv = BipowerVariance::new(20);
        assert!(!bv.is_ready());
        assert_eq!(bv.value(), 0.0);
    }

    #[test]
    fn test_bipower_variance_warmup() {
        let mut bv = BipowerVariance::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            bv.feed(price);
        }
        assert!(bv.is_ready());
    }

    #[test]
    fn test_bipower_variance_positive() {
        let mut bv = BipowerVariance::new(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            bv.feed(price);
        }
        assert!(bv.value() >= 0.0);
    }

    #[test]
    fn test_bipower_variance_reset() {
        let mut bv = BipowerVariance::new(20);
        for i in 0..25 {
            bv.feed(100.0 + i as f64);
        }
        bv.reset();
        assert!(!bv.is_ready());
        assert_eq!(bv.value(), 0.0);
    }
}
