// Multi-anchor AVWAP reversion score: distance to nearest AVWAP among multiple anchors (z-normalized)

use crate::indicators::levels::anchored_vwap::{AnchoredVwap, AnchoredVwapParams};

#[derive(Debug, Clone)]
pub struct AvwapMultiAnchorReversion {
    anchors: Vec<AnchoredVwap>,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    pub value: f64,
}

impl AvwapMultiAnchorReversion {
    pub fn new(params_list: Vec<AnchoredVwapParams>, z_window: usize) -> Self {
        let mut anchors = Vec::new();
        for p in params_list {
            anchors.push(AnchoredVwap::new(p));
        }
        let w = z_window.max(20);
        Self {
            anchors,
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            value: 0.0,
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        for a in &mut self.anchors {
            a.reset();
        }
        self.buf.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.value = 0.0;
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.filled && !self.anchors.is_empty()
    }

    /// Feed `(ts, &[H,L,C,V])` — the `Fields +time` contract arm. ts canonical ms.
    pub fn feed(&mut self, ts_ms: i64, lanes: &[f64]) -> f64 {
        let h = lanes[0];
        let l = lanes[1];
        let c = lanes[2];
        let v = lanes[3];
        let mut best = std::f64::INFINITY;
        let mut any = false;
        for a in &mut self.anchors {
            let vw = a.feed(ts_ms, &[h, l, c, v]);
            let dist = (c - vw).abs();
            if dist < best {
                best = dist;
                any = true;
            }
        }
        let d = if any { best } else { 0.0 };
        self.buf[self.idx] = d;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let n = self.window;
            let mut mean = 0.0;
            for i in 0..n {
                mean += self.buf[i];
            }
            mean /= n as f64;
            let mut var = 0.0;
            for i in 0..n {
                let dd = self.buf[i] - mean;
                var += dd * dd;
            }
            let std = (var / (n as f64)).sqrt().max(1e-9);
            self.value = -(d - mean) / std;
        } // negative for reversion (more negative -> further from AVWAP)
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

}

impl Default for AvwapMultiAnchorReversion {
    /// Factory default: `new(vec![AnchoredVwapParams::default()], 100)`
    /// (z_window = `unwrap_or(100)`, single monthly-anchored AVWAP).
    fn default() -> Self {
        Self::new(vec![AnchoredVwapParams::default()], 100)
    }
}

// ── contract ──────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Port, Render, RenderSpec, SourceAxis, Store,
    StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`AvwapMultiAnchorReversion`] — z-window over a monthly-anchored AVWAP.
/// `AvwapAnchorMode` is single-valued (Monthly), so the anchor set is one monthly AVWAP.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AvwapMrevConfig {
    pub z_window: Param<usize>,
}

impl crate::contract::Config for AvwapMrevConfig {
    fn defaults() -> Self {
        AvwapMrevConfig { z_window: Param::Solo(100) }
    }
    fn machine_defaults() -> Self {
        // z_window: Class A period — auto range(2,4048,1) is correct.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}

impl Indicator for AvwapMultiAnchorReversion {
    const ID: IndicatorId = IndicatorId::AvwapMrev;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const NEEDS_VOLUME: bool = true;
    const OUTPUTS: &'static [Output] = &[Output::centered(IndicatorOutputId::AvwapMrev)];
    /// Per-bar z-normalization rescans the window buffer (Linear); inner AVWAP via a Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Avwap, &[IndicatorOutputId::Avwap])],
    };
    type Config = AvwapMrevConfig;
    type Runtime = AvwapMultiAnchorReversion;

    fn create(cfg: AvwapMrevConfig) -> AvwapMultiAnchorReversion {
        AvwapMultiAnchorReversion::new(vec![AnchoredVwapParams::default()], cfg.z_window.resolved())
    }
}


impl Render for AvwapMultiAnchorReversion {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(
                IndicatorOutputId::AvwapMrev,
                "AVWAP Multi-Anchor Reversion",
                Color::hex(0x9C27B0),
            )
            .zero_baseline()
            .precision(3)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avwap_multi_anchor_reversion_creation() {
        let params = vec![AnchoredVwapParams::default()];
        let mar = AvwapMultiAnchorReversion::new(params, 30);
        assert!(!mar.is_ready());
        assert_eq!(mar.value, 0.0);
    }

    #[test]
    fn test_avwap_multi_anchor_reversion_warmup() {
        let params = vec![AnchoredVwapParams::default()];
        let mut mar = AvwapMultiAnchorReversion::new(params, 30);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            mar.feed(1_700_000_000, &[price + 1.0, price - 1.0, price, 1000.0]);
        }
        assert!(mar.is_ready());
    }

    #[test]
    fn test_avwap_multi_anchor_reversion_values() {
        let params = vec![AnchoredVwapParams::default()];
        let mut mar = AvwapMultiAnchorReversion::new(params, 30);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = mar.feed(1_700_000_000, &[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(value.is_finite(), "Value should be finite");
        }
    }

    #[test]
    fn test_avwap_multi_anchor_reversion_reset() {
        let params = vec![AnchoredVwapParams::default()];
        let mut mar = AvwapMultiAnchorReversion::new(params, 30);
        for i in 0..40 {
            mar.feed(1_700_000_000, &[101.0, 99.0, 100.0, 1000.0]);
            let _ = i;
        }
        mar.reset();
        assert!(!mar.is_ready());
        assert_eq!(mar.value, 0.0);
    }
}
