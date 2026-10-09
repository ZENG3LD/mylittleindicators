// High-performance Fuzzy Candlesticks
// (c) 2024


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandleDirection { Bull = 1, None = 0, Bear = -1 }

impl CandleDirection {
    pub fn as_i8(&self) -> i8 {
        match self {
            CandleDirection::Bull => 1,
            CandleDirection::None => 0,
            CandleDirection::Bear => -1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandleSize { None, VerySmall, Small, Medium, Large, VeryLarge, ExtremelyLarge }

impl CandleSize {
    pub fn as_i8(&self) -> i8 {
        match self {
            CandleSize::None => 0,
            CandleSize::VerySmall => 1,
            CandleSize::Small => 2,
            CandleSize::Medium => 3,
            CandleSize::Large => 4,
            CandleSize::VeryLarge => 5,
            CandleSize::ExtremelyLarge => 6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandleBodySize { None, Small, Medium, Large, Trend }

impl CandleBodySize {
    pub fn as_i8(&self) -> i8 {
        match self {
            CandleBodySize::None => 0,
            CandleBodySize::Small => 1,
            CandleBodySize::Medium => 2,
            CandleBodySize::Large => 3,
            CandleBodySize::Trend => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CandleWickSize { None, Small, Medium, Large }

impl CandleWickSize {
    pub fn as_i8(&self) -> i8 {
        match self {
            CandleWickSize::None => 0,
            CandleWickSize::Small => 1,
            CandleWickSize::Medium => 2,
            CandleWickSize::Large => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FuzzyCandle {
    pub direction: CandleDirection,
    pub size: CandleSize,
    pub body_size: CandleBodySize,
    pub upper_wick_size: CandleWickSize,
    pub lower_wick_size: CandleWickSize,
}

#[derive(Clone, Debug)]
pub struct FuzzyCandlesticks {
    period: usize,
    threshold1: f64,
    threshold2: f64,
    threshold3: f64,
    threshold4: f64,
    lengths: Vec<f64>,
    body_percents: Vec<f64>,
    upper_wick_percents: Vec<f64>,
    lower_wick_percents: Vec<f64>,
    idx: usize,
    filled: bool,
    value: FuzzyCandle,
}

impl FuzzyCandlesticks {
    fn fuzzify_size(length: f64, mean_length: f64, sd_lengths: f64, t1: f64, t2: f64, t3: f64, t4: f64) -> CandleSize {
        if length == 0.0 {
            return CandleSize::None;
        }
        let mut x;
        // VerySmall
        x = sd_lengths.mul_add(-t2, mean_length);
        if length <= x {
            return CandleSize::VerySmall;
        }
        // Small
        x = sd_lengths.mul_add(t1, mean_length);
        if length <= x {
            return CandleSize::Small;
        }
        // Medium
        x = sd_lengths * t2;
        if length <= x {
            return CandleSize::Medium;
        }
        // Large
        x = sd_lengths.mul_add(t3, mean_length);
        if length <= x {
            return CandleSize::Large;
        }
        // VeryLarge
        x = sd_lengths.mul_add(t4, mean_length);
        if length <= x {
            return CandleSize::VeryLarge;
        }
        CandleSize::ExtremelyLarge
    }

    fn fuzzify_body_size(body_percent: f64, mean_body_percent: f64, sd_body_percent: f64, t1: f64, t2: f64, t3: f64) -> CandleBodySize {
        if body_percent == 0.0 {
            return CandleBodySize::None;
        }
        let mut x;
        // Small
        x = sd_body_percent.mul_add(-t1, mean_body_percent);
        if body_percent <= x {
            return CandleBodySize::Small;
        }
        // Medium
        x = sd_body_percent.mul_add(t2, mean_body_percent);
        if body_percent <= x {
            return CandleBodySize::Medium;
        }
        // Large
        x = sd_body_percent.mul_add(t3, mean_body_percent);
        if body_percent <= x {
            return CandleBodySize::Large;
        }
        CandleBodySize::Trend
    }

    fn fuzzify_wick_size(wick_percent: f64, mean_wick_percent: f64, sd_wick_percents: f64, t1: f64, t2: f64) -> CandleWickSize {
        if wick_percent == 0.0 {
            return CandleWickSize::None;
        }
        let mut x;
        // Small
        x = sd_wick_percents.mul_add(-t1, mean_wick_percent);
        if wick_percent <= x {
            return CandleWickSize::Small;
        }
        // Medium
        x = sd_wick_percents.mul_add(t2, mean_wick_percent);
        if wick_percent <= x {
            return CandleWickSize::Medium;
        }
        CandleWickSize::Large
    }

    pub fn new(period: usize, t1: f64, t2: f64, t3: f64, t4: f64) -> Self {
        Self {
            period,
            threshold1: t1,
            threshold2: t2,
            threshold3: t3,
            threshold4: t4,
            lengths: Vec::with_capacity(period),
            body_percents: Vec::with_capacity(period),
            upper_wick_percents: Vec::with_capacity(period),
            lower_wick_percents: Vec::with_capacity(period),
            idx: 0,
            filled: false,
            value: FuzzyCandle {
                direction: CandleDirection::None,
                size: CandleSize::None,
                body_size: CandleBodySize::None,
                upper_wick_size: CandleWickSize::None,
                lower_wick_size: CandleWickSize::None,
            },
        }
    }

    /// Feed `[open, high, low, close]` lanes. Returns the fuzzy classification.
    pub fn feed(&mut self, lanes: &[f64]) -> FuzzyCandle {
        let (open, high, low, close) = (lanes[0], lanes[1], lanes[2], lanes[3]);
        let len = (high - low).abs();
        let (body_percent, upper_wick_percent, lower_wick_percent) = if len == 0.0 {
            (0.0, 0.0, 0.0)
        } else {
            (
                (open - low) / len,
                (high - open.max(close)) / len,
                (open.max(close) - low) / len,
            )
        };
        if self.lengths.len() == self.period {
            self.lengths.remove(0);
            self.body_percents.remove(0);
            self.upper_wick_percents.remove(0);
            self.lower_wick_percents.remove(0);
        }
        self.lengths.push(len);
        self.body_percents.push(body_percent);
        self.upper_wick_percents.push(upper_wick_percent);
        self.lower_wick_percents.push(lower_wick_percent);
        self.idx += 1;
        if self.lengths.len() >= self.period {
            self.filled = true;
        }
        if !self.filled {
            self.value = FuzzyCandle {
                direction: CandleDirection::None,
                size: CandleSize::None,
                body_size: CandleBodySize::None,
                upper_wick_size: CandleWickSize::None,
                lower_wick_size: CandleWickSize::None,
            };
            return self.value;
        }
        let mean_len = self.lengths.iter().sum::<f64>() / self.lengths.len() as f64;
        let sd_len = (self.lengths.iter().map(|&v| (v - mean_len).powi(2)).sum::<f64>() / self.lengths.len() as f64).sqrt();
        let mean_body = self.body_percents.iter().sum::<f64>() / self.body_percents.len() as f64;
        let sd_body = (self.body_percents.iter().map(|&v| (v - mean_body).powi(2)).sum::<f64>() / self.body_percents.len() as f64).sqrt();
        let mean_uw = self.upper_wick_percents.iter().sum::<f64>() / self.upper_wick_percents.len() as f64;
        let sd_uw = (self.upper_wick_percents.iter().map(|&v| (v - mean_uw).powi(2)).sum::<f64>() / self.upper_wick_percents.len() as f64).sqrt();
        let mean_lw = self.lower_wick_percents.iter().sum::<f64>() / self.lower_wick_percents.len() as f64;
        let sd_lw = (self.lower_wick_percents.iter().map(|&v| (v - mean_lw).powi(2)).sum::<f64>() / self.lower_wick_percents.len() as f64).sqrt();
        let direction = if close > open {
            CandleDirection::Bull
        } else if close < open {
            CandleDirection::Bear
        } else {
            CandleDirection::None
        };
        let idx = self.lengths.len() - 1;
        let size = Self::fuzzify_size(self.lengths[idx], mean_len, sd_len, self.threshold1, self.threshold2, self.threshold3, self.threshold4);
        let body_size = Self::fuzzify_body_size(self.body_percents[idx], mean_body, sd_body, self.threshold1, self.threshold2, self.threshold3);
        let upper_wick_size = Self::fuzzify_wick_size(self.upper_wick_percents[idx], mean_uw, sd_uw, self.threshold1, self.threshold2);
        let lower_wick_size = Self::fuzzify_wick_size(self.lower_wick_percents[idx], mean_lw, sd_lw, self.threshold1, self.threshold2);
        self.value = FuzzyCandle {
            direction,
            size,
            body_size,
            upper_wick_size,
            lower_wick_size,
        };
        self.value
    }

    /// Typed fuzzy value (legacy accessor).
    pub fn fuzzy_value(&self) -> FuzzyCandle {
        self.value
    }


    pub fn is_ready(&self) -> bool {
        self.filled
    }

    pub fn reset(&mut self) {
        self.lengths.clear();
        self.body_percents.clear();
        self.upper_wick_percents.clear();
        self.lower_wick_percents.clear();
        self.idx = 0;
        self.filled = false;
        self.value = FuzzyCandle {
            direction: CandleDirection::None,
            size: CandleSize::None,
            body_size: CandleBodySize::None,
            upper_wick_size: CandleWickSize::None,
            lower_wick_size: CandleWickSize::None,
        };
    }
}

impl Default for FuzzyCandlesticks {
    fn default() -> Self {
        Self::new(50, 0.5, 1.0, 1.5, 2.0)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Color, Cost, Family, Indicator, Output, Param, Render, RenderOutput, RenderSpec, SourceAxis,
    Store, StoreKind, UpdateComplexity,
};
use crate::engine::stream_kind::StreamKind;

/// Config for FuzzyCandlesticks.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FuzzyCandlesticksConfig {
    pub period: Param<usize>,
    /// Threshold 1 (default 0.5)
    pub t1: Param<f64>,
    /// Threshold 2 (default 1.0)
    pub t2: Param<f64>,
    /// Threshold 3 (default 1.5)
    pub t3: Param<f64>,
    /// Threshold 4 (default 2.0)
    pub t4: Param<f64>,
}

impl Indicator for FuzzyCandlesticks {
    const ID: IndicatorId = IndicatorId::Fuzzy;
    /// Not a pluggable family member — a fuzzy bar classifier.
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open,
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// O(period): each bar recomputes mean+SD over the rolling window (four Vec buffers).
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
            Store::window(StoreKind::Vec),
        ],
    );
    const OUTPUTS: &'static [Output] = &[
        Output::discrete(IndicatorOutputId::FuzzyDirection),
        Output::discrete(IndicatorOutputId::FuzzySize),
        Output::discrete(IndicatorOutputId::FuzzyBodySize),
        Output::discrete(IndicatorOutputId::FuzzyUpperWick),
        Output::discrete(IndicatorOutputId::FuzzyLowerWick),
    ];
    type Config = FuzzyCandlesticksConfig;
    type Runtime = FuzzyCandlesticks;

    fn create(cfg: FuzzyCandlesticksConfig) -> FuzzyCandlesticks {
        FuzzyCandlesticks::new(
            cfg.period.resolved(),
            cfg.t1.resolved(),
            cfg.t2.resolved(),
            cfg.t3.resolved(),
            cfg.t4.resolved(),
        )
    }
}

impl crate::contract::Config for FuzzyCandlesticksConfig {
    fn defaults() -> Self {
        FuzzyCandlesticksConfig {
            period: Param::Solo(50),
            t1: Param::Solo(0.5),
            t2: Param::Solo(1.0),
            t3: Param::Solo(1.5),
            t4: Param::Solo(2.0),
        }
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
        // period: Class A → auto range(2,4048,1) — leave as-is
        // t1..t4: Class K indicator-specific fuzzy thresholds → sweep 0.1..=5.0 step 0.1
        s.t1 = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s.t2 = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s.t3 = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s.t4 = Param::many(sweep_f64(0.1, 5.0, 0.1));
        s
    }
}


impl Render for FuzzyCandlesticks {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::FuzzyDirection,
                "Fuzzy Direction",
                Color::hex(0x00BCD4),
                2.0,
            ))
            .precision(0)
            .build()
    }
}

impl FuzzyCandlesticks {
    pub fn direction(&self) -> f64 { self.value.direction.as_i8() as f64 }
    pub fn size(&self) -> f64 { self.value.size.as_i8() as f64 }
    pub fn body_size(&self) -> f64 { self.value.body_size.as_i8() as f64 }
    pub fn upper_wick(&self) -> f64 { self.value.upper_wick_size.as_i8() as f64 }
    pub fn lower_wick(&self) -> f64 { self.value.lower_wick_size.as_i8() as f64 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_candlesticks_creation() {
        let fc = FuzzyCandlesticks::new(20, 0.5, 1.0, 1.5, 2.0);
        assert!(!fc.is_ready());
        assert_eq!(fc.fuzzy_value().direction, CandleDirection::None);
    }

    #[test]
    fn test_fuzzy_candlesticks_warmup() {
        let mut fc = FuzzyCandlesticks::new(20, 0.5, 1.0, 1.5, 2.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            fc.feed(&[price, price + 1.0, price - 1.0, price + 0.5]);
        }
        assert!(fc.is_ready());
    }

    #[test]
    fn test_fuzzy_candlesticks_bull() {
        let mut fc = FuzzyCandlesticks::new(20, 0.5, 1.0, 1.5, 2.0);
        for i in 0..25 {
            let open = 100.0 + i as f64;
            let close = open + 2.0;
            fc.feed(&[open, close + 0.5, open - 0.5, close]);
        }
        assert_eq!(fc.fuzzy_value().direction, CandleDirection::Bull);
    }

    #[test]
    fn test_fuzzy_candlesticks_bear() {
        let mut fc = FuzzyCandlesticks::new(20, 0.5, 1.0, 1.5, 2.0);
        for i in 0..25 {
            let open = 100.0 + i as f64;
            let close = open - 2.0;
            fc.feed(&[open, open + 0.5, close - 0.5, close]);
        }
        assert_eq!(fc.fuzzy_value().direction, CandleDirection::Bear);
    }

    #[test]
    fn test_fuzzy_candlesticks_reset() {
        let mut fc = FuzzyCandlesticks::new(20, 0.5, 1.0, 1.5, 2.0);
        for i in 0..25 {
            fc.feed(&[100.0 + i as f64, 101.0, 99.0, 100.0 + i as f64]);
        }
        fc.reset();
        assert!(!fc.is_ready());
        assert_eq!(fc.fuzzy_value().direction, CandleDirection::None);
    }

    /// Factory resolves [Open, High, Low, Close] from const SOURCE.
    /// Volume (9999.0) is wild — proves it is NOT consumed.
    /// After warming up with consistent bull bars, direction should be Bull.
    #[test]
    fn factory_feeds_resolved_lanes() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let mut f = IndicatorOrder::Fuzzy(<<FuzzyCandlesticks as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..55 {
            let open = 100.0 + i as f64;
            let close = open + 2.0;
            f.feed(0, MarketSample::Bar {
                open,
                high: close + 0.5,
                low: open - 0.5,
                close,
                volume: 9999.0,
            });
        }
        assert!(f.is_ready());
        // f.value() returns first output (direction as f64)
        let direction = f.primary() as i8;
        assert_eq!(direction, 1i8, "expected Bull(1) direction, got {direction}");
    }
}
