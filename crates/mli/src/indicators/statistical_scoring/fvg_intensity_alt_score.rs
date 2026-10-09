// Alternative FVG intensity score: exponential weighting of recent FVG gaps magnitude

use crate::indicators::structure::FvgEventDetector as FvgDetector;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct FvgIntensityAltScore {
    det: FvgDetector,
    alpha: f64,
    pub current_value: f64,
    buffer: VecDeque<(f64, f64, f64, f64)>,  // (open, high, low, close)
}

impl FvgIntensityAltScore {
    pub fn new(alpha: f64) -> Self {
        Self {
            det: FvgDetector::new(),
            alpha: alpha.clamp(0.0, 1.0),
            current_value: 0.0,
            buffer: VecDeque::with_capacity(3),
        }
    }
    #[inline]
    pub fn reset(&mut self) {
        self.det.reset();
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

        // Need minimum 3 bars for triplet
        if self.buffer.len() < 3 {
            return 0.0;
        }

        // Extract triplet (bar0 = oldest, bar2 = newest)
        let (o0, h0, l0, c0) = self.buffer[0];
        let (o1, h1, l1, c1) = self.buffer[1];
        let (o2, h2, l2, c2) = self.buffer[2];

        // Call update_triplet
        self.update_triplet(
            o0, h0, l0, c0,
            o1, h1, l1, c1,
            o2, h2, l2, c2,
        )
    }

    /// Get current indicator value
    pub fn value(&self) -> f64 {
        self.current_value
    }
    pub fn update_triplet(
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
        let (bull, bear) = self
            .det
            .update_triplet(o0, h0, l0, c0, o1, h1, l1, c1, o2, h2, l2, c2);
        let gap = if bull {
            (l1 - h0).max(l1 - h2).max(0.0)
        } else if bear {
            (l0 - h1).min(l2 - h1).abs().max(0.0)
        } else {
            0.0
        };
        self.current_value = self.alpha * gap + (1.0 - self.alpha) * self.current_value;
        self.current_value
    }
}

impl Default for FvgIntensityAltScore {
    fn default() -> Self {
        Self::new(0.1)
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

/// Typed dual-mode contract config for [`FvgIntensityAltScore`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FvgaltConfig {
    /// EMA smoothing factor for gap magnitude (clamped 0..1).
    pub alpha: Param<f64>,
}

impl Indicator for FvgIntensityAltScore {
    const ID: IndicatorId = IndicatorId::Fvgalt;
    /// Not a pluggable family member — an exponentially-weighted FVG gap magnitude scorer.
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
    /// O(1) per bar: EMA-style running scalar + fixed-depth 3-bar ring buffer.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[Store::fixed(StoreKind::Deque, 3)],
        inner: &[Port::new(IndicatorId::Fvg, &[IndicatorOutputId::Fvg])],
    };
    const OUTPUTS: &'static [Output] = &[Output::magnitude(IndicatorOutputId::Fvgalt)];
    type Config = FvgaltConfig;
    type Runtime = FvgIntensityAltScore;

    fn create(cfg: FvgaltConfig) -> FvgIntensityAltScore {
        FvgIntensityAltScore::new(cfg.alpha.resolved())
    }
}

impl crate::contract::Config for FvgaltConfig {
    fn defaults() -> Self {
        FvgaltConfig { alpha: Param::Solo(0.1) }
    }
    fn machine_defaults() -> Self {
        use crate::contract::sweep_f64;
        // alpha: EMA smoothing factor — Class E (alpha/decay), 0.01..=0.99 step 0.01.
        let mut s = Self::machine_defaults_auto();
        s.alpha = Param::many(sweep_f64(0.01, 0.99, 0.01));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for FvgIntensityAltScore {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(
                IndicatorOutputId::Fvgalt,
                "FVG Alt",
                Color::hex(0xFF9800),
                2.0,
            ))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fvg_intensity_alt_creation() {
        let fia = FvgIntensityAltScore::new(0.1);
        assert!(!fia.is_ready()); // Not ready until 3 bars
        assert_eq!(fia.current_value, 0.0);
    }

    #[test]
    fn test_fvg_intensity_alt_update() {
        let mut fia = FvgIntensityAltScore::new(0.1);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            fia.update_triplet(
                price, price + 1.0, price - 1.0, price,
                price + 1.0, price + 2.0, price, price + 1.5,
                price + 2.0, price + 3.0, price + 1.0, price + 2.5,
            );
        }
        assert!(fia.current_value >= 0.0);
    }

    #[test]
    fn test_fvg_intensity_alt_non_negative() {
        let mut fia = FvgIntensityAltScore::new(0.2);
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let value = fia.update_triplet(
                price, price + 1.0, price - 1.0, price,
                price + 1.0, price + 2.0, price, price + 1.5,
                price + 2.0, price + 3.0, price + 1.0, price + 2.5,
            );
            assert!(value >= 0.0, "Value should be non-negative");
        }
    }

    #[test]
    fn test_fvg_intensity_alt_reset() {
        let mut fia = FvgIntensityAltScore::new(0.1);
        fia.update_triplet(
            100.0, 101.0, 99.0, 100.0,
            101.0, 102.0, 100.0, 101.5,
            102.0, 103.0, 101.0, 102.5,
        );
        fia.reset();
        assert_eq!(fia.current_value, 0.0);
    }

    /// Factory resolves Open/High/Low/Close lanes; feeds overlapping bars (no FVG),
    /// so EMA-weighted gap stays at 0.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Fvgalt(<<FvgIntensityAltScore as crate::contract::Indicator>::Config as crate::contract::Config>::defaults())
            .build_solo()
            .unwrap();
        // Overlapping bars: no gap, current_value stays 0
        for i in 0..10 {
            let p = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: p, high: p + 0.5, low: p - 0.5, close: p, volume: 9999.0,
            });
        }
        assert_eq!(f.primary(), 0.0, "no FVG in overlapping bars → score 0.0");
    }
}
