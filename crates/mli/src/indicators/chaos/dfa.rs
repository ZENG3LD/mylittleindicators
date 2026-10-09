// Detrended Fluctuation Analysis (DFA) proxy: slope of log(F(n)) vs log(n) for few scales

#[derive(Debug, Clone)]
pub struct Dfa {
    windows: [usize; 4],
    buffers: Vec<Vec<f64>>, // per scale demeaned cum-sum
    idx: usize,
    filled: bool,
    pub alpha: f64,
}

impl Default for Dfa {
    fn default() -> Self {
        Self::new([16, 32, 64, 128])
    }
}

impl Dfa {
    pub fn new(scales: [usize; 4]) -> Self {
        let mut bufs = Vec::with_capacity(4);
        for &w in &scales {
            bufs.push(vec![0.0; w]);
        }
        Self {
            windows: scales,
            buffers: bufs,
            idx: 0,
            filled: false,
            alpha: 0.0,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        for b in &mut self.buffers {
            b.fill(0.0);
        }
        self.idx = 0;
        self.filled = false;
        self.alpha = 0.0;
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn value(&self) -> f64 {
        self.alpha
    }

    pub fn feed(&mut self, c: f64) -> f64 {
        // maintain cumulative sum per scale and compute RMS detrended via simple linear fit proxy
        for (si, w) in self.windows.iter().enumerate() {
            let buf = &mut self.buffers[si];
            let n = *w;
            buf[self.idx % n] = c;
            if self.idx >= n {
                // demean
                let mean: f64 = buf[..n].iter().sum::<f64>() / n as f64;
                let mut sxy = 0.0;
                let mut sx = 0.0;
                let mut sy = 0.0;
                let mut sxx = 0.0;
                for (i, &bv) in buf[..n].iter().enumerate() {
                    let x = i as f64;
                    let y = bv - mean;
                    sx += x;
                    sy += y;
                    sxy += x * y;
                    sxx += x * x;
                }
                let denom = n as f64 * sxx - sx * sx;
                let (a, beta) = if denom.abs() > 1e-12 {
                    let beta = (n as f64 * sxy - sx * sy) / denom;
                    let a = (sy - beta * sx) / n as f64;
                    (a, beta)
                } else {
                    (0.0, 0.0)
                };
                // RMS residual
                let mut rss = 0.0;
                for (i, &bv) in buf[..n].iter().enumerate() {
                    let x = i as f64;
                    let fit = a + beta * x;
                    let r = (bv - mean) - fit;
                    rss += r * r;
                }
                let f = (rss / n as f64).sqrt();
                // store back f in first cell as cheap cache
                buf[0] = f;
            }
        }
        self.idx += 1;
        if !self.filled {
            self.filled = self.windows.iter().all(|&w| self.idx >= w);
        }
        if self.filled {
            // compute slope between first two non-zero Fs on log-log grid
            let mut xs = Vec::new();
            let mut ys = Vec::new();
            for (si, &w) in self.windows.iter().enumerate() {
                let f = self.buffers[si][0];
                if f > 0.0 {
                    xs.push((w as f64).ln());
                    ys.push(f.ln());
                }
            }
            if xs.len() >= 2 {
                let n = xs.len() as f64;
                let mut sx = 0.0;
                let mut sy = 0.0;
                let mut sxy = 0.0;
                let mut sxx = 0.0;
                for i in 0..xs.len() {
                    sx += xs[i];
                    sy += ys[i];
                    sxy += xs[i] * ys[i];
                    sxx += xs[i] * xs[i];
                }
                let denom = n * sxx - sx * sx;
                self.alpha = if denom.abs() > 1e-9 {
                    (n * sxy - sx * sy) / denom
                } else {
                    0.5
                };
            }
        }
        self.alpha
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, ReferenceLine, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Config for DFA: the four scale windows used in the log-log regression.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DfaConfig {
    pub scales: Param<[usize; 4]>,
}

impl Indicator for Dfa {
    const ID: IndicatorId = IndicatorId::Dfa;
    /// Statistical chaos detector — not a pluggable family member.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Single configurable close field.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    /// O(max_window) per bar (rescans each scale buffer for linear fit). Four
    /// Vec buffers (one per scale, sized to each scale window).
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
        ],
    );
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Dfa)];

    type Config = DfaConfig;
    type Runtime = Dfa;

    fn create(cfg: DfaConfig) -> Dfa {
        Dfa::new(cfg.scales.resolved())
    }
}

impl crate::contract::Config for DfaConfig {
    fn defaults() -> Self {
        DfaConfig { scales: Param::Solo([16, 32, 64, 128]) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto();
        // scales: Class S fixed-array → 4 canonical DFA presets
        s.scales = Param::many(vec![
            [4usize, 8, 16, 32],
            [8usize, 16, 32, 64],
            [16usize, 32, 64, 128],
            [32usize, 64, 128, 256],
        ]);
        s
    }
}


impl Render for Dfa {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Dfa, "DFA", Color::hex(0x009688))
            .reference_line(ReferenceLine::new(0.5, Color::hex(0x9E9E9E)).with_label("Random Walk"))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dfa_creation() {
        let ind = Dfa::new([8, 16, 32, 64]);
        assert!(!ind.is_ready());
        assert_eq!(ind.alpha, 0.0);
    }

    #[test]
    fn test_dfa_warmup() {
        let mut ind = Dfa::new([8, 16, 32, 64]);
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.feed(price);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_dfa_values_finite() {
        let mut ind = Dfa::new([8, 16, 32, 64]);
        for i in 0..100 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let alpha = ind.feed(price);
            assert!(alpha.is_finite());
        }
    }

    #[test]
    fn test_dfa_reset() {
        let mut ind = Dfa::new([8, 16, 32, 64]);
        for i in 0..70 {
            ind.feed(100.0 + i as f64);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.alpha, 0.0);
    }

    #[test]
    fn factory_feeds_resolved_dfa() {
        use crate::contract::MarketSample;
        use crate::engine::contract_engine::IndicatorOrder;
        let mut f = IndicatorOrder::Dfa(<<Dfa as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..70 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            f.feed(0, MarketSample::Bar {
                open: 9999.0, // not used — SOURCE is Close
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.read(IndicatorOutputId::Dfa).is_finite());
    }
}
