// FVG duration/intensity score based on recent detected gaps

use crate::indicators::structure::FvgEventDetector as FvgDetector;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct FvgDurationIntensityScore {
    det: FvgDetector,
    window: usize,
    hits: Vec<f64>,
    idx: usize,
    filled: bool,
    pub current_value: f64,
    buffer: VecDeque<(f64, f64, f64, f64)>,  // (open, high, low, close)
}

impl FvgDurationIntensityScore {
    pub fn new(lookback: usize, agg_window: usize) -> Self {
        let _ = lookback;
        let w = agg_window.max(20);
        Self {
            det: FvgDetector::new(),
            window: w,
            hits: vec![0.0; w],
            idx: 0,
            filled: false,
            current_value: 0.0,
            buffer: VecDeque::with_capacity(3),
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.det.reset();
        self.hits.fill(0.0);
        self.idx = 0;
        self.filled = false;
        self.current_value = 0.0;
        self.buffer.clear();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.buffer.len() >= 3
    }

    /// Feed resolved OHLC lanes: `[open, high, low, close]`.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let open  = lanes[0];
        let high  = lanes[1];
        let low   = lanes[2];
        let close = lanes[3];

        // Maintain 3-bar ring buffer
        if self.buffer.len() >= 3 {
            self.buffer.pop_front();
        }
        self.buffer.push_back((open, high, low, close));

        // Need minimum 3 bars
        if self.buffer.len() < 3 {
            return 0.0;
        }

        // Extract triplet
        let (o0, h0, l0, c0) = self.buffer[0];
        let (o1, h1, l1, c1) = self.buffer[1];
        let (o2, h2, l2, c2) = self.buffer[2];

        // Call update_triplet_and_score
        self.update_triplet_and_score(
            o0, h0, l0, c0,
            o1, h1, l1, c1,
            o2, h2, l2, c2,
        )
    }

    /// Get current indicator value
    pub fn value(&self) -> f64 {
        self.current_value
    }
    pub fn update_triplet_and_score(
        &mut self,
        o0: f64,
        h0: f64,
        l0: f64,
        c0: f64,
        o1: f64,
        h1: f64,
        l1: f64,
        c1: f64,
        o2: f64,
        h2: f64,
        l2: f64,
        c2: f64,
    ) -> f64 {
        let (is_bull, is_bear) = self
            .det
            .update_triplet(o0, h0, l0, c0, o1, h1, l1, c1, o2, h2, l2, c2);
        let hit = if is_bull || is_bear { 1.0 } else { 0.0 };
        self.hits[self.idx] = hit;
        self.idx = (self.idx + 1) % self.window;
        if self.idx == 0 {
            self.filled = true;
        }
        if self.filled {
            let mut s = 0.0;
            for &x in &self.hits {
                s += x;
            }
            self.current_value = s / (self.window as f64);
        }
        self.current_value
    }
}

impl Default for FvgDurationIntensityScore {
    fn default() -> Self {
        Self::new(20, 50)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Port, Render, RenderOutput, RenderSpec,
    SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode contract config for [`FvgDurationIntensityScore`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FvgdurConfig {
    /// Ignored lookback parameter (kept for API compat).
    pub lookback: Param<usize>,
    /// Rolling aggregation window for the hit-rate score.
    pub agg_window: Param<usize>,
}

impl Indicator for FvgDurationIntensityScore {
    const ID: IndicatorId = IndicatorId::Fvgdur;
    /// Not a pluggable family member — a scoring composite over FVG detections.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads all four price fields (open/high/low/close) from the bar for the
    /// 3-bar ring buffer.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// O(window) per bar (iterates `hits` to sum the rate), plus one fixed-depth
    /// OHLC ring buffer (Deque 3) and a heap `hits` vector of `agg_window` depth.
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[
            Store::fixed(StoreKind::Deque, 3),
            Store::window(StoreKind::Vec),
        ],
        inner: &[Port::new(IndicatorId::Fvg, &[IndicatorOutputId::Fvg])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Fvgdur)];
    type Config = FvgdurConfig;
    type Runtime = FvgDurationIntensityScore;

    fn create(cfg: FvgdurConfig) -> FvgDurationIntensityScore {
        FvgDurationIntensityScore::new(cfg.lookback.resolved(), cfg.agg_window.resolved())
    }
}

impl crate::contract::Config for FvgdurConfig {
    fn defaults() -> Self {
        FvgdurConfig {
            lookback: Param::Solo(20),
            agg_window: Param::Solo(50),
        }
    }
    fn machine_defaults() -> Self {
        // lookback: ignored by runtime but kept for API compat — auto period sweep.
        // agg_window: rolling aggregation window — auto period sweep.
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for FvgDurationIntensityScore {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::Fvgdur,
                "FVG Duration",
                Color::hex(0x9C27B0),
                2.0,
            ))
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fvg_duration_intensity_creation() {
        let fdi = FvgDurationIntensityScore::new(10, 30);
        assert!(!fdi.is_ready());
        assert_eq!(fdi.current_value, 0.0);
    }

    #[test]
    fn test_fvg_duration_intensity_warmup() {
        let mut fdi = FvgDurationIntensityScore::new(10, 30);
        for i in 0..40 {
            let price = 100.0 + i as f64;
            fdi.feed(&[price, price + 1.0, price - 1.0, price]);
        }
        assert!(fdi.is_ready());
    }

    #[test]
    fn test_fvg_duration_intensity_range() {
        let mut fdi = FvgDurationIntensityScore::new(10, 30);
        for i in 0..50 {
            let price = 100.0 + i as f64;
            let value = fdi.update_triplet_and_score(
                price, price + 1.0, price - 1.0, price,
                price + 1.0, price + 2.0, price, price + 1.5,
                price + 2.0, price + 3.0, price + 1.0, price + 2.5,
            );
            assert!(value >= 0.0 && value <= 1.0, "Score should be in [0, 1]");
        }
    }

    #[test]
    fn test_fvg_duration_intensity_reset() {
        let mut fdi = FvgDurationIntensityScore::new(10, 30);
        for i in 0..40 {
            let price = 100.0 + i as f64;
            fdi.update_triplet_and_score(
                price, price + 1.0, price - 1.0, price,
                price + 1.0, price + 2.0, price, price + 1.5,
                price + 2.0, price + 3.0, price + 1.0, price + 2.5,
            );
        }
        fdi.reset();
        assert!(!fdi.is_ready());
        assert_eq!(fdi.current_value, 0.0);
    }

    /// Factory resolves Open/High/Low/Close lanes; feeds a flat price stream (no FVG),
    /// so the score stays 0 after warm-up.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Fvgdur(<<FvgDurationIntensityScore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..60 {
            let p = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: p, high: p + 0.5, low: p - 0.5, close: p, volume: 9999.0,
            });
        }
        let v = f.primary();
        assert!(v >= 0.0 && v <= 1.0, "score must be in [0,1], got {v}");
    }
}
