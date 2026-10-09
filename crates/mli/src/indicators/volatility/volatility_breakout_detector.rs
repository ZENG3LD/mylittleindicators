//! Volatility Breakout Detector
//!
//! Detects sudden volatility expansion that often precedes large price moves.
//! Uses multi-level volatility analysis and momentum.

use crate::indicators::volatility::atr::Atr;
use crate::engine::contract_engine::{SmootherId, SmootherSlot};

/// Breakout classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakoutType {
    None,
    Mild,
    Strong,
    Extreme,
    Squeeze,
}

impl BreakoutType {
    pub fn as_str(&self) -> &'static str {
        match self {
            BreakoutType::None    => "Нет пробоя",
            BreakoutType::Mild    => "Умеренный пробой",
            BreakoutType::Strong  => "Сильный пробой",
            BreakoutType::Extreme => "Экстремальный пробой",
            BreakoutType::Squeeze => "Сжатие волатильности",
        }
    }

    pub fn as_number(&self) -> i8 {
        match self {
            BreakoutType::None    => 0,
            BreakoutType::Mild    => 1,
            BreakoutType::Strong  => 2,
            BreakoutType::Extreme => 3,
            BreakoutType::Squeeze => -1,
        }
    }
}

/// Per-bar snapshot from [`VolatilityBreakoutDetector`].
#[derive(Debug, Clone, Copy)]
pub struct VolatilityBreakoutResult {
    pub breakout_type: BreakoutType,
    pub volatility_ratio: f64,
    pub breakout_strength: f64,
    pub momentum_factor: f64,
    pub squeeze_duration: usize,
    pub breakout_probability: f64,
    pub direction_bias: i8,
    pub persistence_score: f64,
}

impl VolatilityBreakoutResult {
    pub fn empty() -> Self {
        Self {
            breakout_type: BreakoutType::None,
            volatility_ratio: 1.0,
            breakout_strength: 0.0,
            momentum_factor: 0.0,
            squeeze_duration: 0,
            breakout_probability: 0.0,
            direction_bias: 0,
            persistence_score: 0.0,
        }
    }

    pub fn strength_description(&self) -> &'static str {
        match self.breakout_strength {
            x if x < 0.5 => "Очень слабый",
            x if x < 1.0 => "Слабый",
            x if x < 1.5 => "Умеренный",
            x if x < 2.0 => "Сильный",
            x if x < 2.5 => "Очень сильный",
            _ => "Экстремальный",
        }
    }

    pub fn direction_description(&self) -> &'static str {
        match self.direction_bias {
            1  => "Восходящий пробой",
            -1 => "Нисходящий пробой",
            _  => "Неопределенное направление",
        }
    }

    pub fn trading_recommendation(&self) -> &'static str {
        match (self.breakout_type, self.persistence_score) {
            (BreakoutType::Extreme, score) if score > 0.7 => "Агрессивная торговля на пробое",
            (BreakoutType::Strong, score) if score > 0.6  => "Торговля на пробое",
            (BreakoutType::Mild, score) if score > 0.5    => "Осторожная торговля",
            (BreakoutType::Squeeze, _)                     => "Подготовка к пробою",
            _ => "Ожидание",
        }
    }
}

/// Volatility Breakout Detector.
#[derive(Debug, Clone)]
pub struct VolatilityBreakoutDetector {
    atr: Atr,
    atr_short: Atr,
    volatility_ma: SmootherSlot,
    momentum_ma: SmootherSlot,
    squeeze_ma: SmootherSlot,

    prices: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    ranges: Vec<f64>,
    volatility_ratios: Vec<f64>,
    breakout_history: Vec<BreakoutType>,

    mild_threshold: f64,
    strong_threshold: f64,
    extreme_threshold: f64,
    squeeze_threshold: f64,

    squeeze_start: Option<usize>,
    last_breakout_bar: Option<usize>,

    current_result: VolatilityBreakoutResult,
    is_ready: bool,
    update_count: usize,
}

impl VolatilityBreakoutDetector {
    pub fn new() -> Self {
        Self::from_smoothers(1.2, 1.8, 2.5, 0.7, SmootherId::Rma)
    }

    pub fn with_parameters(
        mild_threshold: f64,
        strong_threshold: f64,
        extreme_threshold: f64,
        squeeze_threshold: f64,
    ) -> Self {
        Self::from_smoothers(mild_threshold, strong_threshold, extreme_threshold, squeeze_threshold, SmootherId::Rma)
    }

    pub fn from_smoothers(
        mild_threshold: f64,
        strong_threshold: f64,
        extreme_threshold: f64,
        squeeze_threshold: f64,
        atr_smoother: SmootherId,
    ) -> Self {
        assert!(mild_threshold > 1.0,    "Mild threshold must be > 1.0");
        assert!(strong_threshold > mild_threshold,   "Strong threshold must be > mild threshold");
        assert!(extreme_threshold > strong_threshold, "Extreme threshold must be > strong threshold");
        assert!(squeeze_threshold > 0.0 && squeeze_threshold < 1.0, "Squeeze threshold must be 0-1");

        Self {
            atr:       Atr::from_smoother(14, atr_smoother),
            atr_short: Atr::from_smoother(5,  atr_smoother),
            volatility_ma: SmootherSlot::new(SmootherId::Ema, 20),
            momentum_ma:   SmootherSlot::new(SmootherId::Ema, 8),
            squeeze_ma:    SmootherSlot::new(SmootherId::Sma, 10),
            prices: Vec::with_capacity(64),
            highs: Vec::with_capacity(32),
            lows: Vec::with_capacity(32),
            ranges: Vec::with_capacity(32),
            volatility_ratios: Vec::with_capacity(32),
            breakout_history: Vec::with_capacity(16),
            mild_threshold,
            strong_threshold,
            extreme_threshold,
            squeeze_threshold,
            squeeze_start: None,
            last_breakout_bar: None,
            current_result: VolatilityBreakoutResult::empty(),
            is_ready: false,
            update_count: 0,
        }
    }

    pub fn feed(&mut self, lanes: &[f64]) -> VolatilityBreakoutResult {
        let high  = lanes[1];
        let low   = lanes[2];
        let close = lanes[3];

        if self.prices.len() >= 64 { self.prices.remove(0); }
        self.prices.push(close);
        if self.highs.len() >= 32 { self.highs.remove(0); }
        self.highs.push(high);
        if self.lows.len() >= 32 { self.lows.remove(0); }
        self.lows.push(low);

        let atr_long  = self.atr.feed(&[high, low, close]);
        let atr_short = self.atr_short.feed(&[high, low, close]);

        if self.ranges.len() >= 32 { self.ranges.remove(0); }
        self.ranges.push(atr_long);

        let volatility_analysis = self.analyze_volatility(atr_long, atr_short);
        let breakout_type = self.determine_breakout_type(volatility_analysis.0);
        self.analyze_momentum_and_direction(close);
        self.calculate_breakout_probability(volatility_analysis.0, breakout_type);
        self.analyze_persistence(breakout_type);
        self.update_squeeze_state(volatility_analysis.0, breakout_type);

        self.current_result.breakout_type = breakout_type;
        self.current_result.volatility_ratio = volatility_analysis.0;
        self.current_result.breakout_strength = volatility_analysis.1;

        if self.atr.is_ready() && self.atr_short.is_ready() && self.ranges.len() >= 10 {
            self.is_ready = true;
        }
        self.update_count += 1;
        self.current_result
    }

    fn analyze_volatility(&mut self, atr_long: f64, atr_short: f64) -> (f64, f64) {
        let smoothed_volatility = self.volatility_ma.feed(atr_long);
        let volatility_ratio = if smoothed_volatility > 0.0 {
            atr_short / smoothed_volatility
        } else {
            1.0
        };
        if self.volatility_ratios.len() >= 32 { self.volatility_ratios.remove(0); }
        self.volatility_ratios.push(volatility_ratio);
        let breakout_strength = if volatility_ratio > 1.0 {
            (volatility_ratio - 1.0) * 2.0
        } else {
            0.0
        };
        (volatility_ratio, breakout_strength)
    }

    fn determine_breakout_type(&self, volatility_ratio: f64) -> BreakoutType {
        if volatility_ratio >= self.extreme_threshold {
            BreakoutType::Extreme
        } else if volatility_ratio >= self.strong_threshold {
            BreakoutType::Strong
        } else if volatility_ratio >= self.mild_threshold {
            BreakoutType::Mild
        } else if volatility_ratio <= self.squeeze_threshold {
            BreakoutType::Squeeze
        } else {
            BreakoutType::None
        }
    }

    fn analyze_momentum_and_direction(&mut self, current_price: f64) {
        if self.prices.len() < 5 { return; }
        let len = self.prices.len();
        let momentum = current_price - self.prices[len - 5];
        let smoothed_momentum = self.momentum_ma.feed(momentum);
        let momentum_factor = if current_price > 0.0 {
            (smoothed_momentum / current_price).clamp(-1.0, 1.0)
        } else {
            0.0
        };
        self.current_result.momentum_factor = momentum_factor;
        self.current_result.direction_bias = if momentum_factor > 0.1 {
            1
        } else if momentum_factor < -0.1 {
            -1
        } else {
            0
        };
    }

    fn calculate_breakout_probability(&mut self, _volatility_ratio: f64, breakout_type: BreakoutType) {
        let base_probability = match breakout_type {
            BreakoutType::Extreme => 0.9,
            BreakoutType::Strong  => 0.7,
            BreakoutType::Mild    => 0.4,
            BreakoutType::Squeeze => 0.8,
            BreakoutType::None    => 0.1,
        };
        let volatility_trend = if self.volatility_ratios.len() >= 5 {
            let recent = &self.volatility_ratios[self.volatility_ratios.len() - 5..];
            let trend: f64 = recent.windows(2)
                .map(|w| if w[1] > w[0] { 1.0 } else { -1.0 })
                .sum();
            trend / 4.0
        } else {
            0.0
        };
        let squeeze_factor = if let Some(start) = self.squeeze_start {
            let duration = self.update_count - start;
            (duration as f64 / 20.0).min(1.0)
        } else {
            0.0
        };
        let probability = (base_probability +
                          volatility_trend.abs() * 0.2 +
                          squeeze_factor * 0.3).min(1.0);
        self.current_result.breakout_probability = probability;
    }

    fn analyze_persistence(&mut self, breakout_type: BreakoutType) {
        if self.breakout_history.len() >= 16 { self.breakout_history.remove(0); }
        self.breakout_history.push(breakout_type);
        if self.breakout_history.len() < 3 {
            self.current_result.persistence_score = 0.5;
            return;
        }
        let recent_breakouts = &self.breakout_history[self.breakout_history.len() - 3..];
        let strength_sum: i8 = recent_breakouts.iter()
            .map(|bt| bt.as_number().max(0))
            .sum();
        let consistency = if self.volatility_ratios.len() >= 3 {
            let recent_ratios = &self.volatility_ratios[self.volatility_ratios.len() - 3..];
            let increasing = recent_ratios.windows(2).all(|w| w[1] >= w[0]);
            let decreasing = recent_ratios.windows(2).all(|w| w[1] <= w[0]);
            if increasing || decreasing { 1.0 } else { 0.5 }
        } else {
            0.5
        };
        let persistence_score = ((strength_sum as f64 / 9.0) * 0.6 + consistency * 0.4).min(1.0);
        self.current_result.persistence_score = persistence_score;
    }

    fn update_squeeze_state(&mut self, _volatility_ratio: f64, breakout_type: BreakoutType) {
        match breakout_type {
            BreakoutType::Squeeze => {
                if self.squeeze_start.is_none() {
                    self.squeeze_start = Some(self.update_count);
                }
            }
            BreakoutType::Mild | BreakoutType::Strong | BreakoutType::Extreme => {
                if self.squeeze_start.is_some() {
                    self.squeeze_start = None;
                }
                self.last_breakout_bar = Some(self.update_count);
            }
            _ => {}
        }
        self.current_result.squeeze_duration = if let Some(start) = self.squeeze_start {
            self.update_count - start
        } else {
            0
        };
    }

    pub fn breakout_type(&self) -> BreakoutType { self.current_result.breakout_type }

    pub fn value(&self) -> f64 {
        (self.current_result.breakout_type.as_number()) as f64
    }

    pub fn result(&self) -> VolatilityBreakoutResult { self.current_result }
    pub fn is_ready(&self) -> bool { self.is_ready }

    pub fn reset(&mut self) {
        self.atr.reset();
        self.atr_short.reset();
        self.volatility_ma.reset();
        self.momentum_ma.reset();
        self.squeeze_ma.reset();
        self.prices.clear();
        self.highs.clear();
        self.lows.clear();
        self.ranges.clear();
        self.volatility_ratios.clear();
        self.breakout_history.clear();
        self.squeeze_start = None;
        self.last_breakout_bar = None;
        self.current_result = VolatilityBreakoutResult::empty();
        self.is_ready = false;
        self.update_count = 0;
    }

    pub fn period(&self) -> usize { self.atr.period() }

    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready { return 0; }
        let result = self.current_result;
        if result.breakout_probability < 0.6 || result.persistence_score < 0.5 { return 0; }
        match result.breakout_type {
            BreakoutType::Strong | BreakoutType::Extreme => result.direction_bias,
            BreakoutType::Mild => {
                if result.persistence_score > 0.7 { result.direction_bias } else { 0 }
            }
            _ => 0,
        }
    }

    pub fn pre_breakout_signal(&self) -> i8 {
        if !self.is_ready { return 0; }
        let result = self.current_result;
        if matches!(result.breakout_type, BreakoutType::Squeeze) &&
           result.squeeze_duration > 10 &&
           result.breakout_probability > 0.7 {
            return result.direction_bias;
        }
        0
    }

    pub fn strength_signal(&self) -> i8 {
        if !self.is_ready { return 0; }
        let strength = self.current_result.breakout_strength;
        if strength > 2.0 { 3 } else if strength > 1.5 { 2 } else if strength > 1.0 { 1 } else { 0 }
    }

    pub fn info(&self) -> String {
        let result = self.current_result;
        let signal = match self.trading_signal() {
            1 => "Покупка на пробое", -1 => "Продажа на пробое", _ => "Нет сигнала",
        };
        format!(
            "Volatility Breakout: {}, Сила: {} ({:.1}), Вероятность: {:.1}%, {}, Сжатие: {} баров, Сигнал: {}",
            result.breakout_type.as_str(),
            result.strength_description(),
            result.breakout_strength,
            result.breakout_probability * 100.0,
            result.direction_description(),
            result.squeeze_duration,
            signal
        )
    }

    pub fn additional_values(&self) -> std::collections::HashMap<String, f64> {
        let mut values = std::collections::HashMap::new();
        values.insert("breakout_type".to_string(),      self.current_result.breakout_type.as_number() as f64);
        values.insert("volatility_ratio".to_string(),   self.current_result.volatility_ratio);
        values.insert("breakout_strength".to_string(),  self.current_result.breakout_strength);
        values.insert("momentum_factor".to_string(),    self.current_result.momentum_factor);
        values.insert("squeeze_duration".to_string(),   self.current_result.squeeze_duration as f64);
        values.insert("breakout_probability".to_string(), self.current_result.breakout_probability);
        values.insert("direction_bias".to_string(),     self.current_result.direction_bias as f64);
        values.insert("persistence_score".to_string(),  self.current_result.persistence_score);
        values
    }

    pub fn update_count(&self) -> usize { self.update_count }

    pub fn parameters(&self) -> (f64, f64, f64, f64) {
        (self.mild_threshold, self.strong_threshold, self.extreme_threshold, self.squeeze_threshold)
    }
}

impl Default for VolatilityBreakoutDetector {
    fn default() -> Self {
        Self::new()
    }
}

// -- contract ------------------------------------------------------------------

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed dual-mode config for [`VolatilityBreakoutDetector`].
///
/// ABSORB: both long-ATR (at `atr_period`) and short-ATR (fixed at 5) are built via
/// `Atr::from_smoother(period, choice.kind)` — no `AtrConfig`. The `atr_smoother` slot
/// drives the smoother kind for both; the short ATR period (5) is architectural.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct VbdConfig {
    pub atr_period: Param<usize>,
    #[slot]
    pub atr_smoother: Param<SmootherChoice>,
    pub mild_threshold: Param<f64>,
    pub strong_threshold: Param<f64>,
    pub extreme_threshold: Param<f64>,
    pub squeeze_threshold: Param<f64>,
}

impl Indicator for VolatilityBreakoutDetector {
    const ID: IndicatorId = IndicatorId::Vbd;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Full OHLCV — ATR needs H/L/C; the momentum uses Close; volume is passed through.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::Open, OhlcvField::High, OhlcvField::Low, OhlcvField::Close, OhlcvField::Volume,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[
            Store::fixed(StoreKind::Vec, 64), // prices
            Store::fixed(StoreKind::Vec, 32), // highs/lows/ranges/volatility_ratios/breakout_history
        ],
        inner: &[
            Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr]),
            Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr]),
        ],
    };
    const SLOTS: &'static [Slot] = VbdConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::ordinal(IndicatorOutputId::Vbd)];
    type Config = VbdConfig;
    type Runtime = VolatilityBreakoutDetector;

    fn create(cfg: VbdConfig) -> VolatilityBreakoutDetector {
        let choice = cfg.atr_smoother.resolved();
        let atr_p = choice.period.resolve(cfg.atr_period.resolved());
        let smoother = choice.kind;
        let mild    = cfg.mild_threshold.resolved();
        let strong  = cfg.strong_threshold.resolved();
        let extreme = cfg.extreme_threshold.resolved();
        let squeeze = cfg.squeeze_threshold.resolved();
        // Build with resolved ATR period; short ATR stays at 5 (architectural constant).
        VolatilityBreakoutDetector {
            atr:           Atr::from_smoother(atr_p, smoother),
            atr_short:     Atr::from_smoother(5, smoother),
            volatility_ma: SmootherSlot::new(SmootherId::Ema, 20),
            momentum_ma:   SmootherSlot::new(SmootherId::Ema, 8),
            squeeze_ma:    SmootherSlot::new(SmootherId::Sma, 10),
            prices:               Vec::with_capacity(64),
            highs:                Vec::with_capacity(32),
            lows:                 Vec::with_capacity(32),
            ranges:               Vec::with_capacity(32),
            volatility_ratios:    Vec::with_capacity(32),
            breakout_history:     Vec::with_capacity(16),
            mild_threshold:    mild,
            strong_threshold:  strong,
            extreme_threshold: extreme,
            squeeze_threshold: squeeze,
            squeeze_start:        None,
            last_breakout_bar:    None,
            current_result:       VolatilityBreakoutResult::empty(),
            is_ready:             false,
            update_count:         0,
        }
    }

    fn slot_members(cfg: &VbdConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for VbdConfig {
    fn defaults() -> Self {
        VbdConfig {
            atr_period: Param::Solo(14),
            atr_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Rma)),
            mild_threshold: Param::Solo(1.2),
            strong_threshold: Param::Solo(1.8),
            extreme_threshold: Param::Solo(2.5),
            squeeze_threshold: Param::Solo(0.7),
        }
    }
    fn machine_defaults() -> Self {
        // atr_period: Class A usize — auto range(2,4048,1)
        // #[slot] atr_smoother: leave Solo (deferred sweep wave)
        // mild_threshold, strong_threshold, extreme_threshold: Class F — sweep_f64(0.1,5.0,0.1)
        // squeeze_threshold: runtime asserts 0..1; treated as Class D — sweep_f64(0.1,0.9,0.05)
        let mut s = Self::machine_defaults_auto();
        s.mild_threshold      = Param::many(crate::contract::sweep_f64(0.1, 5.0, 0.1));
        s.strong_threshold    = Param::many(crate::contract::sweep_f64(0.1, 5.0, 0.1));
        s.extreme_threshold   = Param::many(crate::contract::sweep_f64(0.1, 5.0, 0.1));
        s.squeeze_threshold   = Param::many(crate::contract::sweep_f64(0.1, 0.9, 0.05));
        s
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for VolatilityBreakoutDetector {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::Vbd, "Vol Breakout", Color::hex(0xFF9800))
            .precision(0)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volatility_breakout_detector_creation() {
        let vbd = VolatilityBreakoutDetector::new();
        assert!(!vbd.is_ready());
        assert_eq!(vbd.breakout_type(), BreakoutType::None);
    }

    #[test]
    fn test_volatility_breakout_detector_with_parameters() {
        let vbd = VolatilityBreakoutDetector::with_parameters(1.3, 2.0, 3.0, 0.6);
        assert_eq!(vbd.parameters(), (1.3, 2.0, 3.0, 0.6));
    }

    #[test]
    fn test_vbd_from_smoothers_ema() {
        let mut vbd = VolatilityBreakoutDetector::from_smoothers(1.2, 1.8, 2.5, 0.7, SmootherId::Ema);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 3.0;
            let result = vbd.feed(&[price, price + 1.0, price - 1.0, price, 1000.0]);
            assert!(result.volatility_ratio.is_finite());
        }
        assert!(vbd.is_ready());
    }

    #[test]
    fn test_breakout_detection() {
        let mut vbd = VolatilityBreakoutDetector::new();
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.01);
            vbd.feed(&[price, price + 0.01, price - 0.01, price, 1000.0]);
        }
        for i in 15..25 {
            let base_price = 100.0;
            let high_vol = 5.0;
            let price = base_price + (i as f64 * 0.5);
            let result = vbd.feed(&[price, price + high_vol, price - high_vol, price, 1000.0]);
            if i > 20 && vbd.is_ready() {
                assert!(result.volatility_ratio > 1.0);
                assert!(result.breakout_strength >= 0.0);
                assert!(result.breakout_probability >= 0.0 && result.breakout_probability <= 1.0);
            }
        }
    }

    #[test]
    fn test_trading_signals() {
        let mut vbd = VolatilityBreakoutDetector::new();
        for i in 0..20 {
            let base_price = 100.0;
            let volatility = if i > 15 { 3.0 } else { 0.5 };
            let price = base_price + i as f64 * 0.2;
            vbd.feed(&[price, price + volatility, price - volatility, price, 1000.0]);
            if i > 18 && vbd.is_ready() {
                let signal = vbd.trading_signal();
                let pre_signal = vbd.pre_breakout_signal();
                let strength_signal = vbd.strength_signal();
                assert!(signal >= -1 && signal <= 1);
                assert!(pre_signal >= -1 && pre_signal <= 1);
                assert!(strength_signal >= 0 && strength_signal <= 3);
            }
        }
    }

    #[test]
    fn factory_feeds_resolved_vbd() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;

        let mut f = IndicatorOrder::Vbd(<<VolatilityBreakoutDetector as crate::contract::Indicator>::Config as crate::contract::Config>::defaults()).build_solo().unwrap();
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 3.0;
            f.feed(0, MarketSample::Bar {
                open: price,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 1000.0,
            });
        }
        assert!(f.is_ready());
    }
}
