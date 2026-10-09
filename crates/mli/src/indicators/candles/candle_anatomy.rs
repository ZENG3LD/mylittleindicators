// Candle Anatomy: body, upper wick, lower wick ratios and flags


#[derive(Clone, Copy, Debug, Default)]
pub struct CandleAnatomyValue {
    pub body: f64,
    pub upper_wick: f64,
    pub lower_wick: f64,
    pub long_upper: bool,
    pub long_lower: bool,
}

#[derive(Debug, Clone)]
pub struct CandleAnatomy {
    pub value: CandleAnatomyValue,
    pub lower_thr: f64,
    pub upper_thr: f64,
}

impl CandleAnatomy {
    pub fn new(long_wick_ratio_threshold: f64) -> Self {
        Self {
            value: CandleAnatomyValue::default(),
            lower_thr: long_wick_ratio_threshold,
            upper_thr: long_wick_ratio_threshold,
        }
    }

    #[inline]
    pub fn reset(&mut self) {
        self.value = CandleAnatomyValue::default();
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }

    /// Feed `[open, high, low, close]` lanes.
    pub fn feed(&mut self, lanes: &[f64]) -> CandleAnatomyValue {
        let (open, high, low, close) = (lanes[0], lanes[1], lanes[2], lanes[3]);
        let range = (high - low).abs();
        if range <= 1e-12 {
            self.value = CandleAnatomyValue {
                body: 0.0,
                upper_wick: 0.0,
                lower_wick: 0.0,
                long_upper: false,
                long_lower: false,
            };
            return self.value;
        }
        let body = (close - open).abs() / range;
        let upper = (high - open.max(close)).max(0.0) / range;
        let lower = (open.min(close) - low).max(0.0) / range;
        self.value = CandleAnatomyValue {
            body,
            upper_wick: upper,
            lower_wick: lower,
            long_upper: upper >= self.upper_thr,
            long_lower: lower >= self.lower_thr,
        };
        self.value
    }

    /// Typed anatomy value (legacy accessor).
    #[inline]
    pub fn anatomy_value(&self) -> CandleAnatomyValue {
        self.value
    }

}

impl Default for CandleAnatomy {
    fn default() -> Self {
        Self::new(0.6)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderOutput, RenderSpec, SourceAxis,
    UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Config for CandleAnatomy — the single long-wick ratio threshold.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct CandleAnatomyConfig {
    pub long_wick_ratio_threshold: Param<f64>,
}

impl Indicator for CandleAnatomy {
    const ID: IndicatorId = IndicatorId::Candleanatomy;
    /// Not a pluggable family member — an anatomical decomposer (ratios + flags).
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// O(1): pure scalar state (body/wick ratios + threshold fields), no buffer.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[
        Output::percent(IndicatorOutputId::CandleanatomyBody),
        Output::percent(IndicatorOutputId::CandleanatomyUpperWick),
        Output::percent(IndicatorOutputId::CandleanatomyLowerWick),
        Output::discrete(IndicatorOutputId::CandleanatomyLongUpper),
        Output::discrete(IndicatorOutputId::CandleanatomyLongLower),
    ];
    type Config = CandleAnatomyConfig;
    type Runtime = CandleAnatomy;

    fn create(cfg: CandleAnatomyConfig) -> CandleAnatomy {
        CandleAnatomy::new(cfg.long_wick_ratio_threshold.resolved())
    }
}

impl crate::contract::Config for CandleAnatomyConfig {
    fn defaults() -> Self {
        CandleAnatomyConfig { long_wick_ratio_threshold: Param::Solo(0.6) }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        use crate::contract::sweep_f64;
        let mut s = Self::machine_defaults_auto();
        // long_wick_ratio_threshold: Class K indicator-specific → sweep 0.1..=0.9 step 0.05
        s.long_wick_ratio_threshold = Param::many(sweep_f64(0.1, 0.9, 0.05));
        s
    }
}


impl Render for CandleAnatomy {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::CandleanatomyBody,
                "Body",
                Color::hex(0x2196F3),
                2.0,
            ))
            .precision(4)
            .build()
    }
}

impl CandleAnatomy {
    pub fn body(&self) -> f64 { self.value.body }
    pub fn upper_wick(&self) -> f64 { self.value.upper_wick }
    pub fn lower_wick(&self) -> f64 { self.value.lower_wick }
    pub fn long_upper(&self) -> f64 { if self.value.long_upper { 1.0 } else { 0.0 } }
    pub fn long_lower(&self) -> f64 { if self.value.long_lower { 1.0 } else { 0.0 } }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_candle_anatomy_creation() {
        let ind = CandleAnatomy::new(0.3);
        assert!(ind.is_ready());
        assert_eq!(ind.value.body, 0.0);
    }

    #[test]
    fn test_candle_anatomy_update() {
        let mut ind = CandleAnatomy::new(0.3);
        let result = ind.feed(&[100.0, 110.0, 90.0, 105.0]);
        assert!(result.body >= 0.0 && result.body <= 1.0);
        assert!(result.upper_wick >= 0.0 && result.upper_wick <= 1.0);
        assert!(result.lower_wick >= 0.0 && result.lower_wick <= 1.0);
    }

    #[test]
    fn test_candle_anatomy_long_wicks() {
        let mut ind = CandleAnatomy::new(0.3);
        // Long lower wick candle (hammer-like)
        let result = ind.feed(&[100.0, 102.0, 90.0, 101.0]);
        assert!(result.lower_wick > result.upper_wick);
    }

    #[test]
    fn test_candle_anatomy_reset() {
        let mut ind = CandleAnatomy::new(0.3);
        ind.feed(&[100.0, 110.0, 90.0, 105.0]);
        ind.reset();
        assert_eq!(ind.value.body, 0.0);
        assert_eq!(ind.value.upper_wick, 0.0);
        assert_eq!(ind.value.lower_wick, 0.0);
    }

    /// Factory resolves [Open, High, Low, Close] from const SOURCE.
    /// Volume (9999.0) is wild — proves it is NOT consumed.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Candleanatomy(<<CandleAnatomy as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        // Doji-like: open≈close → body ratio near 0
        f.feed(0, MarketSample::Bar {
            open: 100.0, high: 110.0, low: 90.0, close: 100.1, volume: 9999.0,
        });
        assert!(f.is_ready());
        // f.value() returns first output (body ratio)
        let body = f.primary();
        assert!(body >= 0.0 && body <= 1.0, "body={body}");
        // near-doji: body should be small
        assert!(body < 0.1, "expected small body for doji-like candle, got {body}");
    }
}
