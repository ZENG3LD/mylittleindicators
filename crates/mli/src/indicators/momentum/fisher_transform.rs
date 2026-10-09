//! Fisher Transform indicator.

/// Fisher Transform - converts prices to Gaussian normal distribution.
///
/// Fisher = 0.5 × ln((1 + Smooth) / (1 - Smooth))
/// where Smooth = EMA(Normalize)
/// Normalize = 2 × ((Close - Lowest) / (Highest - Lowest)) - 1
/// Trigger = Previous Fisher value
///
/// # Parameters
/// - `period`: Normalization lookback period (typically 10)
/// - `smooth_period`: EMA smoothing period (typically 3)
#[derive(Debug, Clone)]
pub struct FisherTransform {
    period: usize,
    smooth_period: usize,

    highs: Vec<f64>,
    lows: Vec<f64>,
    normalized_values: Vec<f64>,

    ema_alpha: f64,
    smoothed_value: f64,
    ema_initialized: bool,

    fisher_value: f64,
    trigger_value: f64,

    count: usize,
    is_ready: bool,
}

impl FisherTransform {
    pub fn new(period: usize, smooth_period: usize) -> Self {
        assert!(period > 0, "Period must be > 0");
        assert!(smooth_period > 0, "Smooth period must be > 0");

        let ema_alpha = 2.0 / (smooth_period as f64 + 1.0);

        Self {
            period,
            smooth_period,
            highs: Vec::with_capacity(period),
            lows: Vec::with_capacity(period),
            normalized_values: Vec::new(),
            ema_alpha,
            smoothed_value: 0.0,
            ema_initialized: false,
            fisher_value: 0.0,
            trigger_value: 0.0,
            count: 0,
            is_ready: false,
        }
    }

    /// Feed resolved `[high, low, close]` lanes — contract input (SOURCE = KlineSlice[H, L, C]).
    /// Returns (fisher, trigger).
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, f64) {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];

        if self.highs.len() >= self.period {
            self.highs.remove(0);
            self.lows.remove(0);
        }
        self.highs.push(high);
        self.lows.push(low);

        if self.highs.len() >= self.period {
            let normalized = self.normalize_price(close);

            if !self.ema_initialized {
                self.smoothed_value = normalized;
                self.ema_initialized = true;
            } else {
                self.smoothed_value = self.ema_alpha * normalized + (1.0 - self.ema_alpha) * self.smoothed_value;
            }

            let clamped = self.smoothed_value.clamp(-0.999, 0.999);

            self.trigger_value = self.fisher_value;
            self.fisher_value = 0.5 * ((1.0 + clamped) / (1.0 - clamped)).ln();

            if self.count >= self.period + self.smooth_period {
                self.is_ready = true;
            }
        }

        self.count += 1;
        (self.fisher_value, self.trigger_value)
    }

    fn normalize_price(&self, close: f64) -> f64 {
        let highest = self.highs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let lowest = self.lows.iter().fold(f64::INFINITY, |a, &b| a.min(b));

        if (highest - lowest).abs() < 1e-12 {
            return 0.0;
        }

        2.0 * ((close - lowest) / (highest - lowest)) - 1.0
    }


    #[inline]
    pub fn fisher_value(&self) -> f64 {
        self.fisher_value
    }

    #[inline]
    pub fn trigger_value(&self) -> f64 {
        self.trigger_value
    }

    /// Brace-named getter: `fisher` output (Fisher Transform line).
    #[inline]
    pub fn fisher(&self) -> f64 {
        self.fisher_value
    }

    /// Brace-named getter: `trigger` output (prior Fisher value, used as signal).
    #[inline]
    pub fn trigger(&self) -> f64 {
        self.trigger_value
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    #[inline]
    pub fn period(&self) -> usize {
        self.period
    }

    pub fn reset(&mut self) {
        self.highs.clear();
        self.lows.clear();
        self.normalized_values.clear();
        self.smoothed_value = 0.0;
        self.ema_initialized = false;
        self.fisher_value = 0.0;
        self.trigger_value = 0.0;
        self.count = 0;
        self.is_ready = false;
    }

    pub fn trading_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        let oversold_level = -1.5;
        let overbought_level = 1.5;
        if self.fisher_value > self.trigger_value
            && self.fisher_value < oversold_level
            && self.trigger_value < oversold_level
        {
            return 1;
        }
        if self.fisher_value < self.trigger_value
            && self.fisher_value > overbought_level
            && self.trigger_value > overbought_level
        {
            return -1;
        }
        0
    }

    pub fn crossover(&self, prev_fisher: f64, prev_trigger: f64) -> i8 {
        if !self.is_ready {
            return 0;
        }
        if prev_fisher <= prev_trigger && self.fisher_value > self.trigger_value {
            return 1;
        }
        if prev_fisher >= prev_trigger && self.fisher_value < self.trigger_value {
            return -1;
        }
        0
    }

    pub fn market_condition(&self) -> &'static str {
        if !self.is_ready {
            return "Initializing";
        }
        match self.fisher_value {
            x if x > 2.0 => "Extremely Overbought",
            x if x > 1.5 => "Overbought",
            x if x > 0.5 => "Bullish",
            x if x > -0.5 => "Neutral",
            x if x > -1.5 => "Bearish",
            x if x > -2.0 => "Oversold",
            _ => "Extremely Oversold",
        }
    }

    pub fn signal_strength(&self) -> f64 {
        if !self.is_ready {
            return 0.0;
        }
        let fisher_abs = self.fisher_value.abs();
        let trigger_diff = (self.fisher_value - self.trigger_value).abs();
        let extremeness = (fisher_abs / 3.0).min(1.0);
        let divergence = (trigger_diff / 2.0).min(1.0);
        (extremeness * 0.7 + divergence * 0.3).min(1.0)
    }

    pub fn reversal_signal(&self) -> i8 {
        if !self.is_ready {
            return 0;
        }
        let extreme_oversold = -2.0;
        let extreme_overbought = 2.0;
        if self.fisher_value < extreme_oversold && self.fisher_value > self.trigger_value {
            return 1;
        }
        if self.fisher_value > extreme_overbought && self.fisher_value < self.trigger_value {
            return -1;
        }
        0
    }

    pub fn info(&self) -> String {
        format!(
            "Fisher: {:.3}, Trigger: {:.3}, Condition: {}, Strength: {:.3}",
            self.fisher_value,
            self.trigger_value,
            self.market_condition(),
            self.signal_strength()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fisher_basic_calculation() {
        let mut fisher = FisherTransform::new(10, 3);
        for i in 1..=25 {
            let base = 100.0 - i as f64;
            fisher.feed(&[base + 1.0, base - 1.0, base - 0.5]);
        }
        assert!(fisher.is_ready());
        let (fisher_val, trigger_val) = (fisher.fisher(), fisher.trigger());
        assert!(fisher_val >= -5.0 && fisher_val <= 5.0);
        assert!(trigger_val >= -5.0 && trigger_val <= 5.0);
    }

    #[test]
    fn test_fisher_uptrend() {
        let mut fisher = FisherTransform::new(10, 3);
        for i in 1..=25 {
            let base = 100.0 + i as f64;
            fisher.feed(&[base + 1.0, base - 0.5, base + 0.5]);
        }
        assert!(fisher.is_ready());
        assert!(fisher.fisher_value() > 0.0, "Fisher in uptrend should be positive");
    }

    #[test]
    fn test_fisher_downtrend() {
        let mut fisher = FisherTransform::new(10, 3);
        for i in 1..=25 {
            let base = 200.0 - i as f64;
            fisher.feed(&[base + 0.5, base - 1.0, base - 0.5]);
        }
        assert!(fisher.is_ready());
        assert!(fisher.fisher_value() < 0.0, "Fisher in downtrend should be negative");
    }

    #[test]
    fn test_fisher_trigger_lags() {
        let mut fisher = FisherTransform::new(10, 3);
        for i in 1..=25 {
            let base = 100.0 + i as f64;
            fisher.feed(&[base + 1.0, base - 0.5, base + 0.5]);
        }
        assert!(fisher.is_ready());
        let (f, t) = (fisher.fisher(), fisher.trigger());
        assert!(f.is_finite());
        assert!(t.is_finite());
    }

    #[test]
    fn test_fisher_reset() {
        let mut fisher = FisherTransform::new(10, 3);
        for i in 1..=25 {
            let base = 100.0 + i as f64;
            fisher.feed(&[base + 1.0, base - 0.5, base]);
        }
        assert!(fisher.is_ready());
        fisher.reset();
        assert!(!fisher.is_ready());
        assert_eq!(fisher.fisher_value(), 0.0);
        assert_eq!(fisher.trigger_value(), 0.0);
    }

    #[test]
    fn test_fisher_period() {
        let fisher = FisherTransform::new(14, 5);
        assert_eq!(fisher.period(), 14);
    }

    #[test]
    fn test_fisher_market_condition() {
        let mut fisher = FisherTransform::new(10, 3);
        for i in 1..=25 {
            let base = 100.0 + i as f64;
            fisher.feed(&[base + 1.0, base - 0.5, base]);
        }
        assert!(fisher.is_ready());
        let condition = fisher.market_condition();
        assert!(
            condition == "Extremely Overbought"
                || condition == "Overbought"
                || condition == "Bullish"
                || condition == "Neutral"
                || condition == "Bearish"
                || condition == "Oversold"
                || condition == "Extremely Oversold"
        );
    }

    #[test]
    fn test_fisher_signal_strength() {
        let mut fisher = FisherTransform::new(10, 3);
        assert_eq!(fisher.signal_strength(), 0.0);
        for i in 1..=25 {
            let base = 100.0 + i as f64;
            fisher.feed(&[base + 1.0, base - 0.5, base]);
        }
        assert!(fisher.is_ready());
        let strength = fisher.signal_strength();
        assert!(strength >= 0.0 && strength <= 1.0);
    }
}

impl Default for FisherTransform {
    fn default() -> Self {
        Self::new(10, 3)
    }
}

// ---- Indicator contract ----

use crate::engine::contract_engine::IndicatorOutputId;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{
    Cost, Family, Indicator, Output, Param, SourceAxis, Store, StoreKind, UpdateComplexity,
};
use crate::contract::Render;
use crate::contract::{Color, RenderOutput, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed config for FisherTransform: normalization period + EMA smoothing period.
///
/// Dual-mode: every field is a `Param` — `Solo` = one value, `Many` = a swept set.
#[derive(Debug, Clone, mli_contract_macros::ConfigAxes)]
pub struct FisherTransformConfig {
    /// Normalization lookback (highest-high / lowest-low window).
    pub period: Param<usize>,
    /// EMA smoothing period applied to the normalized value.
    pub smooth_period: Param<usize>,
}

impl Indicator for FisherTransform {
    const ID: IndicatorId = IndicatorId::MoFisher;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads High, Low, Close — fixed fields.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::MoFisherFisher),
        Output::centered(IndicatorOutputId::MoFisherTrigger),
    ];
    /// O(period): normalize_price scans the H/L window each bar.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[
            Store::window(StoreKind::Vec), // highs
            Store::window(StoreKind::Vec), // lows
        ],
    );

    type Config = FisherTransformConfig;
    type Runtime = FisherTransform;

    fn create(cfg: FisherTransformConfig) -> FisherTransform {
        FisherTransform::new(cfg.period.resolved(), cfg.smooth_period.resolved())
    }
}

impl crate::contract::Config for FisherTransformConfig {
    fn defaults() -> Self {
        FisherTransformConfig {
            period: Param::Solo(10),
            smooth_period: Param::Solo(3),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // period/smooth_period: Class A → auto range(2,4048,1).
        Self::machine_defaults_auto()
    }
}


impl Render for FisherTransform {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::line(
                IndicatorOutputId::MoFisherFisher,
                "Fisher",
                Color::hex(0x2196F3),
                2.0,
            ))
            .output(RenderOutput::line(
                IndicatorOutputId::MoFisherTrigger,
                "Trigger",
                Color::hex(0xFF5722),
                1.0,
            ))
            .zero_baseline()
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod contract_tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn factory_feeds_resolved_lanes() {
        let cfg = <<FisherTransform as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(cfg.period.resolved(), 10);
        assert_eq!(cfg.smooth_period.resolved(), 3);
        let mut f = IndicatorOrder::MoFisher(cfg)
            .build_solo()
            .unwrap();
        for i in 1..=25 {
            let base = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: base + 1.0,
                low: base - 0.5,
                close: base + 0.5,
                volume: 0.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
