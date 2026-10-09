//! FundingSentimentAlignment — detects alignment between funding rate sign and long/short positioning.
//!
//! Dual consumer: `FundingRateConsumer` + `LongShortRatioConsumer`.
//!
//! Logic:
//! - `funding > 0` (longs pay shorts) AND `long_ratio > 0.5` (crowd is long) → `+1`
//!   (longs over-positioned — bearish reversal signal)
//! - `funding < 0` (shorts pay longs) AND `long_ratio < 0.5` (crowd is short) → `-1`
//!   (shorts over-positioned — bullish reversal signal)
//! - Otherwise → `0`
//!
//! Output: `Signal(i8)`.

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::streams::funding_rate_consumer::FundingRateConsumer;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::streams::long_short_ratio_consumer::LongShortRatioConsumer;
use crate::contract::{Cost, Family, Indicator, Output, SourceAxis, UpdateComplexity};
use crate::contract::Render;
use crate::contract::{Color, HistogramStyle, RenderOutput, RenderSpec};
use crate::core::types::FundingRate;
use crate::core::types::LongShortRatio;
use crate::engine::stream_kind::StreamKind;

/// Alignment detector between funding rate and market positioning.
///
/// Implements both `FundingRateConsumer` and `LongShortRatioConsumer`.
/// Inherent methods used by `IndicatorInstance` dispatch to avoid UFCS ambiguity.
#[derive(Debug, Clone)]
pub struct FundingSentimentAlignment {
    last_funding: f64,
    last_long_ratio: f64,
    last_signal: i8,
    funding_seen: bool,
    ratio_seen: bool,
}

impl FundingSentimentAlignment {
    /// Create a new indicator.
    pub fn new() -> Self {
        Self {
            last_funding: 0.0,
            last_long_ratio: 0.5,
            last_signal: 0,
            funding_seen: false,
            ratio_seen: false,
        }
    }

    fn recompute(&mut self) {
        self.last_signal = if self.last_funding > 0.0 && self.last_long_ratio > 0.5 {
            1
        } else if self.last_funding < 0.0 && self.last_long_ratio < 0.5 {
            -1
        } else {
            0
        };
    }

    /// Current value (inherent — avoids UFCS conflict).
    pub fn indicator_value(&self) -> f64 {
        self.last_signal as f64
    }

    /// True when both streams have delivered at least one update.
    pub fn indicator_is_ready(&self) -> bool {
        self.funding_seen && self.ratio_seen
    }

    /// Reset all internal state.
    pub fn indicator_reset(&mut self) {
        self.last_funding = 0.0;
        self.last_long_ratio = 0.5;
        self.last_signal = 0;
        self.funding_seen = false;
        self.ratio_seen = false;
    }
}

impl Default for FundingSentimentAlignment {
    fn default() -> Self {
        Self::new()
    }
}

impl FundingRateConsumer for FundingSentimentAlignment {
    fn update_funding(&mut self, fr: &FundingRate) {
        self.last_funding = fr.rate;
        self.funding_seen = true;
        self.recompute();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

impl LongShortRatioConsumer for FundingSentimentAlignment {
    fn update_long_short_ratio(&mut self, lsr: &LongShortRatio) {
        self.last_long_ratio = lsr.long_ratio;
        self.ratio_seen = true;
        self.recompute();
    }


    fn reset(&mut self) {
        self.indicator_reset();
    }

    fn is_ready(&self) -> bool {
        self.indicator_is_ready()
    }
}

/// Typed configuration for [`FundingSentimentAlignment`].
///
/// No tuneable parameters — the indicator is purely threshold-driven
/// (funding sign × long-ratio vs 0.5).
#[derive(Debug, Clone)]
pub struct FundingSentimentAlignmentConfig;

impl FundingSentimentAlignmentConfig {
    /// Config fingerprint: no params → stable constant.
    pub fn config_hash(&self) -> u64 { 0 }

    /// Paramless config — the sole cube point is `self` (mixed-radix dual of `iter().nth`).
    pub fn axes_decode(&self, _idx: u128) -> Self { self.clone() }
}

impl Indicator for FundingSentimentAlignment {
    const ID: IndicatorId = IndicatorId::FundingSentimentAlignment;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Funding, StreamKind::LongShortRatio];
    const SOURCE: Option<SourceAxis> = None;
    const OUTPUTS: &'static [Output] = &[Output::discrete(IndicatorOutputId::FundingSentimentAlignment)];
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    type Config = FundingSentimentAlignmentConfig;
    type Runtime = FundingSentimentAlignment;

    fn create(_cfg: FundingSentimentAlignmentConfig) -> FundingSentimentAlignment {
        FundingSentimentAlignment::new()
    }
}

impl crate::contract::Config for FundingSentimentAlignmentConfig {
    fn defaults() -> Self {
        FundingSentimentAlignmentConfig
    }
    fn machine_defaults() -> Self {
        // No Param fields — nothing to sweep. machine_defaults == defaults.
        FundingSentimentAlignmentConfig
    }
    fn cube_size(&self) -> u128 {
        1
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        Box::new(std::iter::once(self.clone()))
    }
}


impl Render for FundingSentimentAlignment {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(
                IndicatorOutputId::FundingSentimentAlignment,
                "Funding-Sentiment Alignment",
                Color::hex(0x9C27B0),
            ))
            .bounds(-1.5, 1.5)
            .histogram_style(HistogramStyle::Centered)
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fr(rate: f64) -> FundingRate {
        FundingRate { rate, next_funding_time: None, timestamp: 1000, ..Default::default()}
    }

    fn make_lsr(long_ratio: f64) -> LongShortRatio {
        LongShortRatio {
            symbol: String::new(),
            ratio_type: "global_account".to_string(),
            long_ratio,
            short_ratio: 1.0 - long_ratio,
            ratio: Some(long_ratio / (1.0 - long_ratio + 1e-9)),
            timestamp: 1000,
            ..Default::default()
        }
    }

    #[test]
    fn positive_funding_majority_long_gives_plus_one() {
        let mut ind = FundingSentimentAlignment::new();
        ind.update_funding(&make_fr(0.001));
        ind.update_long_short_ratio(&make_lsr(0.65));
        let s = ind.indicator_value().round() as i8;
        assert_eq!(s, 1);
    }

    #[test]
    fn negative_funding_majority_short_gives_minus_one() {
        let mut ind = FundingSentimentAlignment::new();
        ind.update_funding(&make_fr(-0.001));
        ind.update_long_short_ratio(&make_lsr(0.35));
        let s = ind.indicator_value().round() as i8;
        assert_eq!(s, -1);
    }

    #[test]
    fn misaligned_gives_zero() {
        let mut ind = FundingSentimentAlignment::new();
        ind.update_funding(&make_fr(0.001));
        ind.update_long_short_ratio(&make_lsr(0.45)); // positive funding, fewer longs
        let s = ind.indicator_value().round() as i8;
        assert_eq!(s, 0);
    }

    #[test]
    fn not_ready_before_both_streams() {
        let mut ind = FundingSentimentAlignment::new();
        ind.update_funding(&make_fr(0.001));
        assert!(!ind.indicator_is_ready());
        ind.update_long_short_ratio(&make_lsr(0.65));
        assert!(ind.indicator_is_ready());
    }

    #[test]
    fn factory_feeds_resolved_funding_sentiment_alignment() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::FundingSentimentAlignment(FundingSentimentAlignmentConfig)
            .build_solo()
            .unwrap();
        let fr = make_fr(0.001);
        let lsr = make_lsr(0.65);
        f.feed(0, MarketSample::Funding(&fr));
        f.feed(0, MarketSample::LongShortRatio(&lsr));
        let s = f.primary().round() as i8;
        assert_eq!(s, 1, "expected +1 for positive funding + majority long");
    }
}
