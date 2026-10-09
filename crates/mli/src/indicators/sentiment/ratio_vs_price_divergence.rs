//! RatioVsPriceDivergence — detects when long_ratio direction opposes price direction.

use std::collections::VecDeque;

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::long_short_ratio_consumer::LongShortRatioConsumer;
use crate::contract::render::Render;
use crate::contract::{Color, Cost, Family, Indicator, Output, Param, RenderSpec, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::core::types::LongShortRatio;
use crate::engine::stream_kind::StreamKind;

/// Measures divergence between the rolling trend of `long_ratio` and price.
///
/// Divergence is present when their directional changes have opposite signs.
/// The score is normalised to [0, 1] based on relative move sizes.
///
/// Outputs: `score`, `side`.
/// - `score` ∈ [0, 1]: strength of the divergence.
/// - `side`: `1.0` = bullish divergence (ratio ↑ while price ↓),
///           `-1.0` = bearish divergence (ratio ↓ while price ↑),
///           `0.0` = no divergence.
///
/// **Note:** the price-history path (`update_price`) is an inherent method that
/// the factory cannot route (the macro only delivers `LongShortRatio` samples).
/// In practice this indicator is a dual-stream consumer; the factory contract
/// covers the ratio side only.
#[derive(Clone, Debug)]
pub struct RatioVsPriceDivergence {
    period: usize,
    ratio_history: VecDeque<f64>,
    price_history: VecDeque<f64>,
    last_score: f64,
    last_side: f64,
}

impl RatioVsPriceDivergence {
    /// Create a new indicator. `period` is clamped to at least 2.
    pub fn new(period: usize) -> Self {
        let period = period.max(2);
        Self {
            period,
            ratio_history: VecDeque::with_capacity(period),
            price_history: VecDeque::with_capacity(period),
            last_score: 0.0,
            last_side: 0.0,
        }
    }

    /// Divergence strength in `[0, 1]`.
    pub fn score(&self) -> f64 {
        self.last_score
    }

    /// Divergence side: `1.0` (bullish), `-1.0` (bearish), `0.0` (none).
    pub fn side(&self) -> f64 {
        self.last_side
    }

    fn compute(&mut self) {
        if self.ratio_history.len() < 2 || self.price_history.len() < 2 {
            return;
        }

        let r_first = self.ratio_history[0];
        let r_last = self.ratio_history[self.ratio_history.len() - 1];
        let p_first = self.price_history[0];
        let p_last = self.price_history[self.price_history.len() - 1];

        let dr = r_last - r_first;
        let dp = p_last - p_first;

        if dr.signum() != dp.signum() && dr.abs() > 1e-9 && dp.abs() > 1e-9 {
            let r_max = self
                .ratio_history
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            let r_min = self
                .ratio_history
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min);
            let p_max = self
                .price_history
                .iter()
                .cloned()
                .fold(f64::NEG_INFINITY, f64::max);
            let p_min = self
                .price_history
                .iter()
                .cloned()
                .fold(f64::INFINITY, f64::min);

            let r_range = r_max - r_min;
            let p_range = p_max - p_min;

            let r_norm = if r_range > 1e-9 { dr.abs() / r_range } else { 0.0 };
            let p_norm = if p_range > 1e-9 { dp.abs() / p_range } else { 0.0 };

            self.last_score = (r_norm * p_norm).min(1.0);
            // ratio ↑ + price ↓ → crowd overextended long → bullish reversal expected
            // ratio ↓ + price ↑ → crowd missing the move → bearish reversal expected
            self.last_side = if dr > 0.0 { 1.0 } else { -1.0 };
        } else {
            self.last_score = 0.0;
            self.last_side = 0.0;
        }
    }

    /// Update with close price from a bar. This is the second input channel;
    /// the factory does not route bars to this indicator (it is contracted as
    /// `LongShortRatio` flavor). Call this directly when both streams are available.
    pub fn update_price(&mut self, close: f64) {
        self.price_history.push_back(close);
        while self.price_history.len() > self.period {
            self.price_history.pop_front();
        }
        self.compute();
    }
}

/// Typed dual-mode config for [`RatioVsPriceDivergence`].
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct RatioVsPriceDivergenceConfig {
    pub period: Param<usize>,
}

impl Indicator for RatioVsPriceDivergence {
    const ID: IndicatorId = IndicatorId::RatioVsPriceDivergence;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::LongShortRatio];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::RatioVsPriceDivergenceScore),
        Output::discrete(IndicatorOutputId::RatioVsPriceDivergenceSide),
    ];
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[
        Store::window(StoreKind::Deque),
        Store::window(StoreKind::Deque),
    ]);
    type Config = RatioVsPriceDivergenceConfig;
    type Runtime = RatioVsPriceDivergence;

    fn create(cfg: RatioVsPriceDivergenceConfig) -> RatioVsPriceDivergence {
        RatioVsPriceDivergence::new(cfg.period.resolved())
    }
}

impl LongShortRatioConsumer for RatioVsPriceDivergence {
    fn update_long_short_ratio(&mut self, lsr: &LongShortRatio) {
        self.ratio_history.push_back(lsr.long_ratio);
        while self.ratio_history.len() > self.period {
            self.ratio_history.pop_front();
        }
        self.compute();
    }


    fn reset(&mut self) {
        self.ratio_history.clear();
        self.price_history.clear();
        self.last_score = 0.0;
        self.last_side = 0.0;
    }

    fn is_ready(&self) -> bool {
        self.ratio_history.len() >= 2 && self.price_history.len() >= 2
    }
}

impl Default for RatioVsPriceDivergence {
    /// Factory default: period=20.
    fn default() -> Self {
        Self::new(20)
    }
}

impl crate::contract::Config for RatioVsPriceDivergenceConfig {
    fn defaults() -> Self {
        RatioVsPriceDivergenceConfig { period: Param::Solo(20) }
    }
    fn machine_defaults() -> Self {
        // period is Class A (lookback period) — auto provides range(2,4048,1).
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for RatioVsPriceDivergence {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(
                IndicatorOutputId::RatioVsPriceDivergenceScore,
                "Divergence Score",
                Color::hex(0xFF7043),
            )
            .line_output(
                IndicatorOutputId::RatioVsPriceDivergenceSide,
                "Divergence Side",
                Color::hex(0x66BB6A),
            )
            .bounds(-1.0, 1.0)
            .precision(3)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;

    fn make_lsr(long_ratio: f64) -> LongShortRatio {
        LongShortRatio {
            symbol: String::new(),
            ratio_type: "global_account".to_string(),
            long_ratio,
            short_ratio: 1.0 - long_ratio,
            ratio: None,
            timestamp: 0,
            ..Default::default()
        }
    }

    #[test]
    fn divergence_detected_when_opposite_directions() {
        let mut ind = RatioVsPriceDivergence::new(4);

        // ratio rises, price falls → divergence
        for (r, p) in [(0.4f64, 100.0f64), (0.5, 95.0), (0.6, 90.0), (0.7, 85.0)] {
            ind.update_long_short_ratio(&make_lsr(r));
            ind.update_price(p);
        }

        let score = ind.score();
        let side = ind.side();
        assert!(score > 0.0, "expected positive divergence score, got {score}");
        assert_eq!(side, 1.0, "expected bullish side (ratio up, price down)");
    }

    #[test]
    fn no_divergence_same_direction() {
        let mut ind = RatioVsPriceDivergence::new(4);

        // both rise — no divergence
        for (r, p) in [(0.4f64, 85.0f64), (0.5, 90.0), (0.6, 95.0), (0.7, 100.0)] {
            ind.update_long_short_ratio(&make_lsr(r));
            ind.update_price(p);
        }

        let score = ind.score();
        let side = ind.side();
        assert_eq!(score, 0.0, "expected no divergence, score={score}");
        assert_eq!(side, 0.0, "expected neutral side");
    }

    #[test]
    fn factory_feeds_resolved_ratio_vs_price_divergence() {
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::RatioVsPriceDivergence(
            <<RatioVsPriceDivergence as crate::contract::Indicator>::Config as crate::contract::Config>::defaults(),
        )
        .build_solo()
        .unwrap();
        // smoke: build + feed ratio samples; price side is unavailable via factory
        f.feed(0, MarketSample::LongShortRatio(&make_lsr(0.4)));
        f.feed(0, MarketSample::LongShortRatio(&make_lsr(0.6)));
        // f.value() = first output (score); no price history → score=0
        let score = f.primary();
        assert_eq!(score, 0.0, "no price history → score=0");
    }
}
