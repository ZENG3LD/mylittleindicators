// High-performance Donchian Channel (DC)
// (c) 2024

#[derive(Debug, Clone)]
pub struct Dc {
    period: usize,
    highs: Vec<f64>,
    lows: Vec<f64>,
    index: usize,
    filled: bool,
    upper: f64,
    middle: f64,
    lower: f64,
}

impl Dc {
    pub fn new(period: usize) -> Self {
        Self {
            period,
            highs: Vec::with_capacity(period),
            lows: Vec::with_capacity(period),
            index: 0,
            filled: false,
            upper: 0.0,
            middle: 0.0,
            lower: 0.0,
        }
    }
    /// Feed the resolved input lanes — `[high, low]` (the factory resolves the fixed
    /// High/Low slice). Knows no transport.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        self.feed_hl(high, low)
    }

    fn feed_hl(&mut self, high: f64, low: f64) -> (f64, f64, f64) {
        if self.highs.len() < self.period {
            self.highs.push(high);
            self.lows.push(low);
        } else {
            self.highs[self.index] = high;
            self.lows[self.index] = low;
            self.filled = true;
        }
        self.index = (self.index + 1) % self.period;

        if self.highs.len() < self.period {
            self.upper = 0.0;
            self.middle = 0.0;
            self.lower = 0.0;
            return (self.upper, self.middle, self.lower);
        }
        let (lower, upper) = self.highs.iter()
            .zip(self.lows.iter())
            .fold((f64::INFINITY, f64::NEG_INFINITY),
                  |(min, max), (&h, &l)| (min.min(l), max.max(h)));
        self.upper = upper;
        self.lower = lower;
        self.middle = 0.5 * (self.upper + self.lower);
        (self.upper, self.middle, self.lower)
    }
    /// Named output getter: brace `upper` (the highest high over the window).
    pub fn upper(&self) -> f64 { self.upper }
    /// Named output getter: brace `middle` (the channel midline).
    pub fn middle(&self) -> f64 { self.middle }
    /// Named output getter: brace `lower` (the lowest low over the window).
    pub fn lower(&self) -> f64 { self.lower }
    pub fn is_ready(&self) -> bool {
        self.filled
    }
    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.index = 0;
        self.filled = false;
        self.upper = 0.0;
        self.middle = 0.0;
        self.lower = 0.0;
    }

}

impl Default for Dc {
    fn default() -> Self {
        Self::new(14)
    }
}

// -- contract -----------------------------------------------------------------

use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::contract_engine::IndicatorOutputId;
use crate::contract::{Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct DcConfig {
    pub period: Param<usize>,
}

impl Indicator for Dc {
    const ID: IndicatorId = IndicatorId::VoDc;
    const FAMILY: &'static [Family] = &[Family::Volatility];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> =
        Some(SourceAxis::KlineSlice(&[OhlcvField::High, OhlcvField::Low]));
    const COST: Cost = Cost::new(UpdateComplexity::Linear, &[Store::window(StoreKind::Vec)]);
    const OUTPUTS: &'static [Output] = &[
        Output::price(IndicatorOutputId::VoDcUpper),
        Output::price(IndicatorOutputId::VoDcMiddle),
        Output::price(IndicatorOutputId::VoDcLower),
    ];
    type Config = DcConfig;
    type Runtime = Dc;

    fn create(cfg: DcConfig) -> Dc {
        Dc::new(cfg.period.resolved())
    }
}

impl crate::contract::Config for DcConfig {
    fn defaults() -> Self {
        DcConfig { period: Param::Solo(14) }
    }
    fn machine_defaults() -> Self {
        // period: Class A usize — auto range(2,4048,1)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 { self.axes_cube_size() }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> { self.axes_iter() }
}


impl Render for Dc {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::VoDcUpper, "DC Upper", Color::hex(0x2196F3))
            .line_output(IndicatorOutputId::VoDcMiddle, "DC Mid", Color::hex(0x9E9E9E))
            .line_output(IndicatorOutputId::VoDcLower, "DC Lower", Color::hex(0x2196F3))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dc_creation() {
        let dc = Dc::new(20);
        assert!(!dc.is_ready());
        assert_eq!(dc.upper(), 0.0);
        assert_eq!(dc.middle(), 0.0);
        assert_eq!(dc.lower(), 0.0);
    }

    #[test]
    fn test_dc_warmup() {
        let mut dc = Dc::new(20);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            dc.feed(&[price + 1.0, price - 1.0]);
        }
        assert!(dc.is_ready());
    }

    #[test]
    fn test_dc_values() {
        let mut dc = Dc::new(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            dc.feed(&[price + 1.0, price - 1.0]);
        }
        let (upper, middle, lower) = (dc.upper(), dc.middle(), dc.lower());
        assert!(upper >= middle);
        assert!(middle >= lower);
    }

    #[test]
    fn test_dc_reset() {
        let mut dc = Dc::new(20);
        for i in 0..25 {
            let price = 100.0 + i as f64;
            dc.feed(&[price + 1.0, price - 1.0]);
        }
        dc.reset();
        assert!(!dc.is_ready());
        assert_eq!(dc.upper(), 0.0);
        assert_eq!(dc.middle(), 0.0);
        assert_eq!(dc.lower(), 0.0);
    }
}
