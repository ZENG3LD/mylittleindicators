// FVG reversion probability within horizon H bars after detection

use crate::indicators::structure::FvgEventDetector as FvgDetector;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
struct PendingGap {
    upper: f64,
    lower: f64,
    remain: usize,
}

#[derive(Debug, Clone)]
pub struct FvgReversionProbability {
    det: FvgDetector,
    horizon: usize,
    active: Vec<PendingGap>,
    total: usize,
    hits: usize,
    pub current_value: f64,
    buffer: VecDeque<(f64, f64, f64, f64)>,  // (open, high, low, close)
}

impl FvgReversionProbability {
    pub fn new(horizon: usize) -> Self {
        Self {
            det: FvgDetector::new(),
            horizon: horizon.clamp(1, 50),
            active: Vec::new(),
            total: 0,
            hits: 0,
            current_value: 0.0,
            buffer: VecDeque::with_capacity(4),
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.det.reset();
        self.active.clear();
        self.total = 0;
        self.hits = 0;
        self.current_value = 0.0;
        self.buffer.clear();
    }
    #[inline]
    pub fn is_ready(&self) -> bool {
        self.buffer.len() >= 4
    }

    /// Feed resolved OHLC lanes: `[open, high, low, close]`.
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let open  = lanes[0];
        let high  = lanes[1];
        let low   = lanes[2];
        let close = lanes[3];

        // Maintain 4-bar ring buffer (3 for triplet + 1 for next_close)
        if self.buffer.len() >= 4 {
            self.buffer.pop_front();
        }
        self.buffer.push_back((open, high, low, close));

        // Need minimum 4 bars
        if self.buffer.len() < 4 {
            return 0.0;
        }

        // Extract triplet (bars 0-2) + next_close (bar 3)
        let (o0, h0, l0, c0) = self.buffer[0];
        let (o1, h1, l1, c1) = self.buffer[1];
        let (o2, h2, l2, c2) = self.buffer[2];
        let (_o3, _h3, _l3, c3) = self.buffer[3];  // next_close

        // Call update_triplet_and_progress
        self.update_triplet_and_progress(
            o0, h0, l0, c0,
            o1, h1, l1, c1,
            o2, h2, l2, c2,
            c3,  // next_close (from bar 3)
        )
    }

    /// Get current indicator value
    pub fn value(&self) -> f64 {
        self.current_value
    }
    pub fn update_triplet_and_progress(
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
        next_close: f64,
    ) -> f64 {
        // detect
        let (bull, bear) = self
            .det
            .update_triplet(o0, h0, l0, c0, o1, h1, l1, c1, o2, h2, l2, c2);
        if bull {
            // gap between h0 and l1
            let upper = l1;
            let lower = h0.max(h2);
            self.active.push(PendingGap {
                upper,
                lower,
                remain: self.horizon,
            });
            self.total += 1;
        } else if bear {
            // gap between h1 and l0 (approx)
            let lower = h1;
            let upper = l0.min(h2);
            self.active.push(PendingGap {
                upper,
                lower,
                remain: self.horizon,
            });
            self.total += 1;
        }
        // progress one bar with next_close to check fill
        let mut still: Vec<PendingGap> = Vec::with_capacity(self.active.len());
        for mut g in self.active.drain(..) {
            if next_close <= g.upper && next_close >= g.lower {
                self.hits += 1;
            } else if g.remain > 1 {
                g.remain -= 1;
                still.push(g);
            }
        }
        self.active = still;
        self.current_value = if self.total > 0 {
            (self.hits as f64) / (self.total as f64)
        } else {
            0.0
        };
        self.current_value
    }
}

impl Default for FvgReversionProbability {
    fn default() -> Self {
        Self::new(10)
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

/// Typed dual-mode contract config for [`FvgReversionProbability`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FvgrevConfig {
    /// How many bars after detection to wait for the gap to fill (1..=50).
    pub horizon: Param<usize>,
}

impl Indicator for FvgReversionProbability {
    const ID: IndicatorId = IndicatorId::Fvgrev;
    /// Not a pluggable family member — a statistical reversion-probability scorer.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads all four price fields (open/high/low/close) from the bar for the
    /// 4-bar ring buffer (triplet + next_close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// O(active_gaps) per bar (drains the active list); the active list is bounded
    /// by horizon so effectively O(horizon). Two heap stores: the 4-bar ring buffer
    /// (Deque 4) and the active-gaps vector (horizon-bounded Vec).
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[
            Store::fixed(StoreKind::Deque, 4),
            Store::window(StoreKind::Vec),
        ],
        inner: &[Port::new(IndicatorId::Fvg, &[IndicatorOutputId::Fvg])],
    };
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::Fvgrev)];
    type Config = FvgrevConfig;
    type Runtime = FvgReversionProbability;

    fn create(cfg: FvgrevConfig) -> FvgReversionProbability {
        FvgReversionProbability::new(cfg.horizon.resolved())
    }
}

impl crate::contract::Config for FvgrevConfig {
    fn defaults() -> Self {
        FvgrevConfig { horizon: Param::Solo(10) }
    }
    fn machine_defaults() -> Self {
        // horizon: forward bars — runtime clamps to 50, sweep the full 1..=50 range.
        let mut s = Self::machine_defaults_auto();
        s.horizon = Param::range(1, 50, 1);
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for FvgReversionProbability {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(
                IndicatorOutputId::Fvgrev,
                "FVG Rev",
                Color::hex(0x009688),
            ))
            .bounds(-1.0, 1.0)
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fvg_reversion_probability_creation() {
        let frp = FvgReversionProbability::new(10);
        assert!(!frp.is_ready()); // needs 10 FVGs
        assert_eq!(frp.current_value, 0.0);
    }

    #[test]
    fn test_fvg_reversion_probability_update() {
        let mut frp = FvgReversionProbability::new(5);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            // Create a bullish FVG scenario
            frp.update_triplet_and_progress(
                price, price + 1.0, price - 1.0, price,
                price + 5.0, price + 6.0, price + 4.0, price + 5.5,
                price + 2.0, price + 3.0, price + 1.0, price + 2.5,
                price + 4.5, // next close
            );
        }
        assert!(frp.current_value >= 0.0 && frp.current_value <= 1.0, "Probability should be in [0, 1]");
    }

    #[test]
    fn test_fvg_reversion_probability_reset() {
        let mut frp = FvgReversionProbability::new(5);
        for i in 0..15 {
            let price = 100.0 + i as f64;
            frp.update_triplet_and_progress(
                price, price + 1.0, price - 1.0, price,
                price + 5.0, price + 6.0, price + 4.0, price + 5.5,
                price + 2.0, price + 3.0, price + 1.0, price + 2.5,
                price + 4.5,
            );
        }
        frp.reset();
        assert!(!frp.is_ready());
        assert_eq!(frp.current_value, 0.0);
    }

    /// Factory resolves Open/High/Low/Close lanes; feeds overlapping bars (no FVG),
    /// so probability stays 0.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Fvgrev(<<FvgReversionProbability as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        // Overlapping bars: no FVG detected, probability stays 0
        for i in 0..20 {
            let p = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: p, high: p + 0.5, low: p - 0.5, close: p, volume: 9999.0,
            });
        }
        assert_eq!(f.primary(), 0.0, "no FVG detected → probability 0.0");
    }
}
