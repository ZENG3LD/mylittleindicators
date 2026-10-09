// Williams VIX Fix (WVF)

use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct Wvf {
    lookback: usize,
    // Monotonic (decreasing-by-value) deque of (seq, close) — its front is the rolling
    // highest-close over the window, maintained in O(1) amortized (no per-bar rescan).
    // Only the CURRENT low is used by the formula, so no low history is kept.
    max_deque: VecDeque<(u64, f64)>,
    seq: u64,
    filled: bool,
    value: f64,
}

impl Wvf {
    pub fn new(lookback: usize) -> Self {
        let lb = lookback.clamp(2, 1024);
        Self {
            lookback: lb,
            max_deque: VecDeque::with_capacity(lb),
            seq: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.max_deque.clear();
        self.seq = 0;
        self.filled = false;
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
    /// Feed the resolved `[low, close]` lanes (in `SOURCE` order).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let l = lanes[0];
        let c = lanes[1];
        let s = self.seq;
        // Push the new close, dropping the dominated tail (anything <= c can never be max).
        while let Some(&(_, b)) = self.max_deque.back() {
            if b <= c {
                self.max_deque.pop_back();
            } else {
                break;
            }
        }
        self.max_deque.push_back((s, c));
        // Evict the front if it slid out of the window (window = the last `lookback` seqs).
        let window_start = (s + 1).saturating_sub(self.lookback as u64);
        while let Some(&(fs, _)) = self.max_deque.front() {
            if fs < window_start {
                self.max_deque.pop_front();
            } else {
                break;
            }
        }
        self.seq += 1;
        if self.seq >= self.lookback as u64 {
            self.filled = true;
        }
        if self.filled {
            let highest_close = self.max_deque.front().map_or(c, |&(_, v)| v);
            if highest_close.abs() > 1e-12 {
                self.value = (highest_close - l) / highest_close * 100.0;
            }
        }
        self.value
    }
}

// -- contract -----------------------------------------------------------------

use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed config for [`Wvf`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct WvfConfig {
    pub lookback: Param<usize>,
}

/// One bounded monotonic `VecDeque` ring for the rolling highest-close.
static WVF_STORES: &[Store] = &[Store::window(StoreKind::Deque)];

impl Indicator for Wvf {
    const ID: IndicatorId = IndicatorId::Wvf;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads Low and Close -- KlineSlice; field is not configurable.
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::Low, OhlcvField::Close]));
    /// O(1) amortized: a monotonic deque yields the rolling highest-close without a rescan.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, WVF_STORES);
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Wvf)];
    type Config = WvfConfig;
    type Runtime = Wvf;

    fn create(cfg: WvfConfig) -> Wvf {
        Wvf::new(cfg.lookback.resolved())
    }
}

impl crate::contract::Config for WvfConfig {
    fn defaults() -> Self {
        WvfConfig { lookback: Param::Solo(22) }
    }
    fn machine_defaults() -> Self {
        // lookback: Class A usize — auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
}


impl Render for Wvf {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Wvf, "Williams VixFix", Color::hex(0xF44336))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wvf_creation() {
        let wvf = Wvf::new(22);
        assert!(!wvf.is_ready());
        assert_eq!(wvf.value(), 0.0);
    }

    #[test]
    fn test_wvf_warmup() {
        let mut wvf = Wvf::new(22);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            wvf.feed(&[price - 1.0, price]);
        }
        assert!(wvf.is_ready());
    }

    #[test]
    fn test_wvf_non_negative() {
        let mut wvf = Wvf::new(22);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = wvf.feed(&[price - 1.0, price]);
            assert!(value >= 0.0, "WVF should be non-negative");
        }
    }

    #[test]
    fn test_wvf_matches_bruteforce_max() {
        // Locks the O(1) monotonic-deque against a brute-force rolling max over the window.
        let lb = 8;
        let mut wvf = Wvf::new(lb);
        let mut closes: Vec<f64> = Vec::new();
        for i in 0..40 {
            let c = 100.0 + (i as f64 * 0.7).sin() * 10.0;
            let l = c - 2.0;
            let v = wvf.feed(&[l, c]);
            closes.push(c);
            if wvf.is_ready() {
                let start = closes.len() - lb;
                let highest = closes[start..].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let expected = (highest - l) / highest * 100.0;
                assert!((v - expected).abs() < 1e-9, "WVF deque max desync at i={}: got {}, expected {}", i, v, expected);
            }
        }
    }

    #[test]
    fn test_wvf_reset() {
        let mut wvf = Wvf::new(22);
        for i in 0..25 {
            wvf.feed(&[99.0, 100.0 + i as f64]);
        }
        wvf.reset();
        assert!(!wvf.is_ready());
        assert_eq!(wvf.value(), 0.0);
    }
}
