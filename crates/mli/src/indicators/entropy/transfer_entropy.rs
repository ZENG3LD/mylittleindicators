// Simplified Transfer Entropy proxy: TE(X->Y) with lag using binned returns and joint frequencies
//
// Self-contained: computes returns from close and volume internally
// X = volume returns, Y = price returns
// Measures information flow from volume to price

#[derive(Debug, Clone)]
pub struct TransferEntropy {
    window: usize,
    lag: usize,
    bins: usize,
    clip_abs: f64,
    rx: Vec<f64>,
    ry: Vec<f64>,
    idx: usize,
    filled: bool,
    // 3D joint: p(y_t, y_{t-1}, x_{t-1}) flattened as [bins^3]
    joint: Vec<usize>,
    count: usize,
    value: f64,
    // For computing returns internally
    prev_close: f64,
    prev_volume: f64,
    has_prev: bool,
}

impl TransferEntropy {
    pub fn new(window: usize, lag: usize, bins: usize, clip_abs: f64) -> Self {
        let w = window.max(3);
        let l = lag.max(1).min(w - 2);
        let b = bins.max(2);
        Self {
            window: w,
            lag: l,
            bins: b,
            clip_abs: clip_abs.max(1e-6),
            rx: vec![0.0; w + 2],
            ry: vec![0.0; w + 2],
            idx: 0,
            filled: false,
            joint: vec![0usize; b * b * b],
            count: 0,
            value: 0.0,
            prev_close: 0.0,
            prev_volume: 0.0,
            has_prev: false,
        }
    }

    #[inline]
    fn bin(&self, r: f64) -> usize {
        let rr = r.max(-self.clip_abs).min(self.clip_abs);
        let x = (rr + self.clip_abs) / (2.0 * self.clip_abs);
        (x * self.bins as f64)
            .floor()
            .clamp(0.0, (self.bins - 1) as f64) as usize
    }

    #[inline]
    fn idx3(&self, y: usize, y1: usize, x1: usize) -> usize {
        (y * self.bins + y1) * self.bins + x1
    }

    #[inline]
    pub fn reset(&mut self) {
        self.rx.fill(0.0);
        self.ry.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.joint.fill(0);
        self.count = 0;
        self.value = 0.0;
        self.prev_close = 0.0;
        self.prev_volume = 0.0;
        self.has_prev = false;
    }

    /// Feed resolved lanes `[close, volume]` (factory resolves `KlineSlice([Close, Volume])`).
    /// X = volume return, Y = price return — measures info flow from volume to price.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let close = lanes[0];
        let volume = lanes[1];
        if !self.has_prev {
            self.prev_close = close;
            self.prev_volume = volume.max(1.0);
            self.has_prev = true;
            return self.value;
        }

        // Compute log returns
        let price_return = if self.prev_close > 0.0 {
            (close / self.prev_close).ln()
        } else {
            0.0
        };
        let volume_return = if self.prev_volume > 0.0 && volume > 0.0 {
            (volume / self.prev_volume).ln()
        } else {
            0.0
        };

        self.prev_close = close;
        self.prev_volume = volume.max(1.0);

        // Use volume as X (source), price as Y (target)
        // Transfer entropy measures: does past volume help predict future price?
        self.update_returns(volume_return, price_return)
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && self.count >= (self.window - self.lag - 1)
    }

    pub fn update_returns(&mut self, r_x: f64, r_y: f64) -> f64 {
        self.rx[self.idx] = r_x;
        self.ry[self.idx] = r_y;
        let curr = self.idx;
        self.idx = (self.idx + 1) % self.rx.len();
        if self.idx == 0 {
            self.filled = true;
        }
        if !self.filled {
            return self.value;
        }

        let by = self.bin(self.ry[curr]);
        let by1 = self.bin(self.ry[(curr + self.ry.len() - 1) % self.ry.len()]);
        let bx1 = self.bin(self.rx[(curr + self.rx.len() - 1 - self.lag) % self.rx.len()]);
        let ins_idx = self.idx3(by, by1, bx1);
        self.joint[ins_idx] += 1;
        self.count += 1;
        if self.count > (self.window - self.lag - 1) {
            let old = (curr + self.rx.len() - (self.window - self.lag - 1)) % self.rx.len();
            let oy = self.bin(self.ry[old]);
            let oy1 = self.bin(self.ry[(old + self.ry.len() - 1) % self.ry.len()]);
            let ox1 = self.bin(self.rx[(old + self.rx.len() - 1 - self.lag) % self.rx.len()]);
            let oi = self.idx3(oy, oy1, ox1);
            if self.joint[oi] > 0 {
                self.joint[oi] -= 1;
            }
            self.count -= 1;
        }
        // TE proxy: sum p(y,y1,x1) log [ p(y|y1,x1) / p(y|y1) ]
        let total = (self.count as f64).max(1.0);
        let mut py_y1x1 = vec![0.0; self.bins * self.bins];
        let mut py_y1 = vec![0.0; self.bins * self.bins];
        for y in 0..self.bins {
            for y1 in 0..self.bins {
                let mut s = 0.0;
                for x1 in 0..self.bins {
                    s += self.joint[self.idx3(y, y1, x1)] as f64;
                }
                py_y1[y * self.bins + y1] += s;
                for x1 in 0..self.bins {
                    py_y1x1[y1 * self.bins + x1] += self.joint[self.idx3(y, y1, x1)] as f64;
                }
            }
        }
        let mut te = 0.0;
        for y in 0..self.bins {
            for y1 in 0..self.bins {
                for x1 in 0..self.bins {
                    let pyyx = (self.joint[self.idx3(y, y1, x1)] as f64) / total;
                    if pyyx <= 0.0 {
                        continue;
                    }
                    let py_given_y1x1 = pyyx / (py_y1x1[y1 * self.bins + x1] / total).max(1e-12);
                    let py_given_y1 = pyyx / (py_y1[y * self.bins + y1] / total).max(1e-12);
                    if py_given_y1x1 > 0.0 && py_given_y1 > 0.0 {
                        te += pyyx * (py_given_y1x1 / py_given_y1).ln();
                    }
                }
            }
        }
        self.value = te.max(0.0);
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

}

impl Default for TransferEntropy {
    /// Factory default (Te arm): window=50, lag=1, bins=8, clip_abs=0.15.
    fn default() -> Self {
        Self::new(50, 1, 8, 0.15)
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

/// Typed config for [`TransferEntropy`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct TeConfig {
    pub window: Param<usize>,
    pub lag: Param<usize>,
    pub bins: Param<usize>,
    pub clip_abs: Param<f64>,
}

impl Indicator for TransferEntropy {
    const ID: IndicatorId = IndicatorId::Te;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed Close + Volume lanes — transfer entropy from volume to price.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[OhlcvField::Close, OhlcvField::Volume]));
    const NEEDS_VOLUME: bool = true;
    /// O(bins³) per bar for the TE sum; two ring-buffer Vec stores.
    const COST: Cost = Cost::new(
        UpdateComplexity::Quadratic,
        &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
    );
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Te)];
    type Config = TeConfig;
    type Runtime = TransferEntropy;

    fn create(cfg: TeConfig) -> TransferEntropy {
        TransferEntropy::new(
            cfg.window.resolved(),
            cfg.lag.resolved(),
            cfg.bins.resolved(),
            cfg.clip_abs.resolved(),
        )
    }
}

impl crate::contract::Config for TeConfig {
    fn defaults() -> Self {
        TeConfig {
            window: Param::Solo(50),
            lag: Param::Solo(1),
            bins: Param::Solo(8),
            clip_abs: Param::Solo(0.15),
        }
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // lag: Class A.lag — 1..=50, corrected from auto 2..=4048
        s.lag = Param::range(1, 50, 1);
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


impl Render for TransferEntropy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Te, "Transfer Entropy", Color::hex(0xE91E63))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transfer_entropy_creation() {
        let te = TransferEntropy::new(30, 1, 4, 0.05);
        assert!(!te.is_ready());
        assert_eq!(te.value(), 0.0);
    }

    #[test]
    fn test_transfer_entropy_warmup() {
        let mut te = TransferEntropy::new(20, 1, 4, 0.05);
        for i in 0..40 {
            let r_x = (i as f64 * 0.1).sin() * 0.01;
            let r_y = (i as f64 * 0.15).sin() * 0.01;
            te.update_returns(r_x, r_y);
        }
        assert!(te.is_ready());
    }

    #[test]
    fn test_transfer_entropy_values_finite() {
        let mut te = TransferEntropy::new(20, 1, 4, 0.05);
        for i in 0..50 {
            let r_x = (i as f64 * 0.1).sin() * 0.01;
            let r_y = (i as f64 * 0.15).sin() * 0.01;
            let value = te.update_returns(r_x, r_y);
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_transfer_entropy_values_non_negative() {
        let mut te = TransferEntropy::new(20, 1, 4, 0.05);
        for i in 0..50 {
            let r_x = (i as f64 * 0.1).sin() * 0.01;
            let r_y = (i as f64 * 0.15).sin() * 0.01;
            let value = te.update_returns(r_x, r_y);
            assert!(value >= 0.0);
        }
    }

    #[test]
    fn test_transfer_entropy_reset() {
        let mut te = TransferEntropy::new(20, 1, 4, 0.05);
        for i in 0..40 {
            let r_x = (i as f64 * 0.1).sin() * 0.01;
            let r_y = (i as f64 * 0.15).sin() * 0.01;
            te.update_returns(r_x, r_y);
        }
        te.reset();
        assert!(!te.is_ready());
        assert_eq!(te.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_te() {
        use crate::contract::market_sample::MarketSample;
        use crate::engine::contract_engine::IndicatorOrder;
        
        let mut f = IndicatorOrder::Te(<<TransferEntropy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        let mut price = 100.0_f64;
        for i in 0..80 {
            price += (i as f64 * 0.1).sin() * 0.5;
            let vol = 1000.0 + (i as f64 * 0.07).cos() * 200.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: vol,
            });
        }
        let v = f.read(IndicatorOutputId::Te);
        assert!(v.is_finite() && v >= 0.0, "te must be finite >= 0: {v}");
    }
}
