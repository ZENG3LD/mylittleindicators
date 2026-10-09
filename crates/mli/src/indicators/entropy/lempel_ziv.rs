// Lempel–Ziv complexity (binaryized returns sign sequence, rolling window)

#[derive(Debug, Clone)]
pub struct LempelZivComplexity {
    window: usize,
    bits: Vec<u8>,
    idx: usize,
    filled: bool,
    last_close: Option<f64>,
    pub value: f64,
}

impl LempelZivComplexity {
    pub fn new(window: usize) -> Self {
        let w = window.max(32);
        Self {
            window: w,
            bits: vec![0; w],
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
        self.bits.fill(0);
        self.last_close = None;
        self.value = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    /// Feed the close scalar (resolved by the factory; `const SOURCE = Field{Close}`).
    pub fn feed(&mut self, c: f64) -> f64 {
        if let Some(p) = self.last_close {
            let r = c - p;
            self.bits[self.idx] = if r >= 0.0 { 1 } else { 0 };
            self.idx = (self.idx + 1) % self.window;
            if !self.filled && self.idx == 0 {
                self.filled = true;
            }
        }
        self.last_close = Some(c);
        if self.filled {
            self.value = self.compute_lz();
        }
        self.value
    }

    fn compute_lz(&self) -> f64 {
        // LZ76 complexity: count number of distinct substrings encountered in parsing
        let n = self.window;
        let mut i = 0;
        let mut c = 1;
        let mut l = 1;
        let mut k = 1;
        let mut k_max = 1;
        let s = &self.bits;
        while i + l <= n {
            if i + k >= n || s[i + k] != s[k - 1] {
                if k > k_max {
                    k_max = k;
                }
                i += 1;
                if i == k {
                    c += 1;
                    k += k_max;
                    if k >= n {
                        break;
                    }
                    i = 0;
                    l = 1;
                    k_max = 1;
                    continue;
                }
                k = 1;
                l = 1;
            } else {
                k += 1;
                l += 1;
            }
        }
        // normalize by n/log n
        let n_f = n as f64;
        let norm = n_f / n_f.ln().max(1.0001);
        c as f64 / norm
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

    pub fn window(&self) -> usize {
        self.window
    }

}

impl Default for LempelZivComplexity {
    /// Factory default (Lz arm): window=64.
    fn default() -> Self {
        Self::new(64)
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

/// Typed config for [`LempelZivComplexity`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct LzConfig {
    pub window: Param<usize>,
}

impl Indicator for LempelZivComplexity {
    const ID: IndicatorId = IndicatorId::Lz;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed close — binarizes price change sign internally.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(n) per bar — LZ76 parsing scan.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Lz)];
    type Config = LzConfig;
    type Runtime = LempelZivComplexity;

    fn create(cfg: LzConfig) -> LempelZivComplexity {
        LempelZivComplexity::new(cfg.window.resolved())
    }
}

impl crate::contract::Config for LzConfig {
    fn defaults() -> Self {
        LzConfig { window: Param::Solo(64) }
    }
    fn machine_defaults() -> Self {
        // window: Class A period — auto handles range(2,4048,1); no other axes
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for LempelZivComplexity {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Lz, "Lempel-Ziv", Color::hex(0x9C27B0))
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
    fn test_lempel_ziv_creation() {
        let lz = LempelZivComplexity::new(64);
        assert!(!lz.is_ready());
        assert_eq!(lz.value(), 0.0);
        assert_eq!(lz.window(), 64);
    }

    #[test]
    fn test_lempel_ziv_warmup() {
        let mut lz = LempelZivComplexity::new(32);
        for i in 0..33 {
            lz.feed(100.0 + i as f64);
        }
        assert!(lz.is_ready());
    }

    #[test]
    fn test_lempel_ziv_finite() {
        let mut lz = LempelZivComplexity::new(32);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            let value = lz.feed(price);
            assert!(value.is_finite(), "LZ complexity should be finite");
        }
    }

    #[test]
    fn test_lempel_ziv_reset() {
        let mut lz = LempelZivComplexity::new(32);
        for i in 0..50 {
            lz.feed(100.0 + i as f64);
        }
        lz.reset();
        assert!(!lz.is_ready());
        assert_eq!(lz.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_lz() {
        
        let mut f = IndicatorOrder::Lz(<<LempelZivComplexity as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..80 {
            let price = 100.0 + (i as f64 * 0.12).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        let v = f.read(IndicatorOutputId::Lz);
        assert!(v.is_finite() && v >= 0.0, "lz must be finite >= 0: {v}");
    }
}
