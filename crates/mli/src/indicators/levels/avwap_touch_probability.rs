// AVWAP touch-probability score over recent anchors

use crate::indicators::levels::anchored_vwap::{AnchoredVwap, AnchoredVwapParams};

#[derive(Debug, Clone)]
pub struct AvwapTouchProbability {
    anchors: Vec<AnchoredVwap>,
    window: usize,
    buf: Vec<f64>,
    idx: usize,
    filled: bool,
    threshold: f64,
    pub value: f64,
}

impl AvwapTouchProbability {
    pub fn new(
        params_list: Vec<AnchoredVwapParams>,
        prob_window: usize,
        touch_threshold: f64,
    ) -> Self {
        let mut anchors = Vec::new();
        for p in params_list {
            anchors.push(AnchoredVwap::new(p));
        }
        let w = prob_window.max(20);
        Self {
            anchors,
            window: w,
            buf: vec![0.0; w],
            idx: 0,
            filled: false,
            threshold: touch_threshold.max(0.0),
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
        self.filled
    }
    /// Feed `(ts, &[H,L,C,V])` — the `Fields +time` contract arm. ts canonical ms.
    pub fn feed(&mut self, ts_ms: i64, lanes: &[f64]) -> f64 {
        let h = lanes[0];
        let l = lanes[1];
        let c = lanes[2];
        let v = lanes[3];
        let mut touched = false;
        for a in &mut self.anchors {
            let vw = a.feed(ts_ms, &[h, l, c, v]);
            if (c - vw).abs() / vw.max(1e-9) <= self.threshold {
                touched = true;
            }
        }
        self.buf[self.idx] = if touched { 1.0 } else { 0.0 };
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut s = 0.0;
            for &x in &self.buf {
                s += x;
            }
            self.value = s / (self.window as f64);
        }
        self.value
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.value
    }

}

impl Default for AvwapTouchProbability {
    /// Factory default: `new(vec![AnchoredVwapParams::default()], 100, 0.001)`
    /// (prob_window = `unwrap_or(100)`, touch_threshold = `unwrap_or(0.001)`).
    fn default() -> Self {
        Self::new(vec![AnchoredVwapParams::default()], 100, 0.001)
    }
}

// ── contract ──────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Port, Render, RenderSpec, SourceAxis, Store,
    StoreKind, UpdateComplexity, sweep_f64,
};
use crate::engine::stream_kind::StreamKind;

/// Typed config for [`AvwapTouchProbability`] — rolling touch frequency over a window.
/// `AvwapAnchorMode` is single-valued (Monthly), so the anchor set is one monthly AVWAP.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct AvwapTprobConfig {
    pub window: Param<usize>,
    pub threshold: Param<f64>,
}

impl crate::contract::Config for AvwapTprobConfig {
    fn defaults() -> Self {
        AvwapTprobConfig {
            window: Param::Solo(100),
            threshold: Param::Solo(0.001),
        }
    }
    fn machine_defaults() -> Self {
        // window: Class A period — auto range(2,4048,1) is correct.
        // threshold: proximity fraction relative to vwap; default 0.001 (0.1%).
        //   Semantically a ratio/fraction (Class D): sweep_f64(0.0,1.0,0.05).
        //   However, useful range is much narrower (0.001–0.05). Classified as
        //   Class D fraction: sweep_f64(0.0,1.0,0.05). Ambiguous — see report.
        let mut s = Self::machine_defaults_auto();
        s.threshold = Param::many(sweep_f64(0.0, 1.0, 0.05));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}

impl Indicator for AvwapTouchProbability {
    const ID: IndicatorId = IndicatorId::AvwapTprob;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
        OhlcvField::Volume,
    ]));
    const NEEDS_VOLUME: bool = true;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::AvwapTprob)];
    /// Per-bar O(1) ring update + an O(window) mean over the touch buffer (Linear); inner
    /// AVWAP via a Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Avwap, &[IndicatorOutputId::Avwap])],
    };
    type Config = AvwapTprobConfig;
    type Runtime = AvwapTouchProbability;

    fn create(cfg: AvwapTprobConfig) -> AvwapTouchProbability {
        AvwapTouchProbability::new(
            vec![AnchoredVwapParams::default()],
            cfg.window.resolved(),
            cfg.threshold.resolved(),
        )
    }
}


impl Render for AvwapTouchProbability {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(
                IndicatorOutputId::AvwapTprob,
                "AVWAP Touch Probability",
                Color::hex(0x9C27B0),
            )
            .bounds(0.0, 1.0)
            .precision(3)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_avwap_touch_probability_creation() {
        let params = vec![AnchoredVwapParams::default()];
        let atp = AvwapTouchProbability::new(params, 30, 0.01);
        assert!(!atp.is_ready());
        assert_eq!(atp.value, 0.0);
    }

    #[test]
    fn test_avwap_touch_probability_warmup() {
        let params = vec![AnchoredVwapParams::default()];
        let mut atp = AvwapTouchProbability::new(params, 30, 0.01);
        for i in 0..40 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            atp.feed(1_700_000_000, &[price + 1.0, price - 1.0, price, 1000.0]);
        }
        assert!(atp.is_ready());
    }

    #[test]
    fn test_avwap_touch_probability_range() {
        let params = vec![AnchoredVwapParams::default()];
        let mut atp = AvwapTouchProbability::new(params, 30, 0.01);
        for i in 0..50 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = atp.feed(1_700_000_000, &[price + 1.0, price - 1.0, price, 1000.0]);
            assert!(value >= 0.0 && value <= 1.0, "Probability should be in [0, 1]");
        }
    }

    #[test]
    fn test_avwap_touch_probability_reset() {
        let params = vec![AnchoredVwapParams::default()];
        let mut atp = AvwapTouchProbability::new(params, 30, 0.01);
        for i in 0..40 {
            atp.feed(1_700_000_000, &[101.0, 99.0, 100.0, 1000.0]);
            let _ = i;
        }
        atp.reset();
        assert!(!atp.is_ready());
        assert_eq!(atp.value, 0.0);
    }
}
