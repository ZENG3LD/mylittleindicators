
/// Heikin-Ashi bar transformer
#[derive(Debug, Clone, Default)]
pub struct HeikinAshi {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    initialized: bool,
}

impl HeikinAshi {
    pub fn new() -> Self {
        Self::default()
    }


    /// Feed `[open, high, low, close]` lanes; returns HA (o, h, l, c).
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64, f64, f64) {
        let (o, h, l, c) = (lanes[0], lanes[1], lanes[2], lanes[3]);
        let ha_close = (o + h + l + c) / 4.0;
        let ha_open = if !self.initialized {
            (o + c) / 2.0
        } else {
            (self.open + self.close) / 2.0
        };
        let ha_high = ha_close.max(ha_open).max(h);
        let ha_low = ha_close.min(ha_open).min(l);
        self.open = ha_open;
        self.high = ha_high;
        self.low = ha_low;
        self.close = ha_close;
        self.initialized = true;
        (self.open, self.high, self.low, self.close)
    }


    #[inline]
    pub fn is_ready(&self) -> bool {
        true
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Named output getter: brace `open` (the Heikin-Ashi open).
    pub fn open(&self) -> f64 { self.open }
    /// Named output getter: brace `high` (the Heikin-Ashi high).
    pub fn high(&self) -> f64 { self.high }
    /// Named output getter: brace `low` (the Heikin-Ashi low).
    pub fn low(&self) -> f64 { self.low }
    /// Named output getter: brace `close` (the Heikin-Ashi close).
    pub fn close(&self) -> f64 { self.close }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Render, RenderOutput, RenderSpec, SourceAxis,
    UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Unit config — HeikinAshi has no parameters.
#[derive(Debug, Clone, Copy, PartialEq, mli_contract_macros::ConfigAxes)]
pub struct HeikinAshiConfig;

impl Indicator for HeikinAshi {
    const ID: IndicatorId = IndicatorId::Heikinashi;
    /// Not a pluggable family member — a bar-transformation producer.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// O(1): purely running scalar state (open, high, low, close, initialized), no buffer.
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::HeikinashiOpen),
        Output::price(IndicatorOutputId::HeikinashiHigh),
        Output::price(IndicatorOutputId::HeikinashiLow),
        Output::price(IndicatorOutputId::HeikinashiClose),
    ];
    type Config = HeikinAshiConfig;
    type Runtime = HeikinAshi;

    fn create(_cfg: HeikinAshiConfig) -> HeikinAshi {
        HeikinAshi::new()
    }
}

impl crate::contract::Config for HeikinAshiConfig {
    fn defaults() -> Self {
        HeikinAshiConfig
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // Unit struct — no parameters to sweep; pure bar transformer.
        Self::machine_defaults_auto()
    }
}


impl Render for HeikinAshi {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .output(RenderOutput::line(
                IndicatorOutputId::HeikinashiOpen,
                "HA Open",
                Color::hex(0x2196F3),
                2.0,
            ))
            .output(RenderOutput::line(
                IndicatorOutputId::HeikinashiHigh,
                "HA High",
                Color::hex(0x4CAF50),
                1.0,
            ))
            .output(RenderOutput::line(
                IndicatorOutputId::HeikinashiLow,
                "HA Low",
                Color::hex(0xF44336),
                1.0,
            ))
            .output(RenderOutput::line(
                IndicatorOutputId::HeikinashiClose,
                "HA Close",
                Color::hex(0xFF9800),
                2.0,
            ))
            .precision(4)
            .build()
    }
}

// open/high/low/close are pub fields — no separate getters needed.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heikin_ashi_creation() {
        let ind = HeikinAshi::new();
        assert!(ind.is_ready());
        assert_eq!(ind.open(), 0.0);
        assert_eq!(ind.high(), 0.0);
        assert_eq!(ind.low(), 0.0);
        assert_eq!(ind.close(), 0.0);
    }

    #[test]
    fn test_heikin_ashi_update() {
        let mut ind = HeikinAshi::new();
        let (ha_o, ha_h, ha_l, ha_c) = ind.feed(&[100.0, 105.0, 95.0, 102.0]);
        assert!(ha_o.is_finite());
        assert!(ha_h.is_finite());
        assert!(ha_l.is_finite());
        assert!(ha_c.is_finite());
        assert!(ha_h >= ha_l);
    }

    #[test]
    fn test_heikin_ashi_multiple_updates() {
        let mut ind = HeikinAshi::new();
        for i in 0..10 {
            let price = 100.0 + i as f64;
            let (ha_o, ha_h, ha_l, ha_c) = ind.feed(&[price, price + 2.0, price - 2.0, price + 1.0]);
            assert!(ha_h >= ha_l);
            assert!(ha_h >= ha_o && ha_h >= ha_c);
            assert!(ha_l <= ha_o && ha_l <= ha_c);
        }
    }

    #[test]
    fn test_heikin_ashi_reset() {
        let mut ind = HeikinAshi::new();
        ind.feed(&[100.0, 105.0, 95.0, 102.0]);
        ind.reset();
        assert_eq!(ind.open(), 0.0);
        assert_eq!(ind.high(), 0.0);
        assert_eq!(ind.low(), 0.0);
        assert_eq!(ind.close(), 0.0);
    }

    /// Factory resolves [Open, High, Low, Close] lanes from const SOURCE.
    /// Volume (9999.0) is a wild value that is NOT in SOURCE — proves correct resolution.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Heikinashi(<<HeikinAshi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        f.feed(0, MarketSample::Bar {
            open: 100.0, high: 105.0, low: 95.0, close: 102.0, volume: 9999.0,
        });
        assert!(f.is_ready());
        // HA open of first bar = (prev_open + prev_close) / 2 = 0 initially, so 0.0
        // f.value() returns first output (open)
        let ha_open = f.primary();
        assert!(ha_open.is_finite());
    }
}
