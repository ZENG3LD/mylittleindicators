//! High-performance Squeeze Momentum
//! Combines Bollinger Bands and Keltner Channels to detect volatility squeeze
//! (c) 2024

use crate::engine::contract_engine::{SmootherId, SmootherSlot};
use crate::indicators::volatility::atr::Atr;

/// Squeeze Momentum - определяет периоды низкой волатильности (сжатие) и последующий взрыв
///
/// Индикатор использует Bollinger Bands и Keltner Channels для определения:
/// 1. Squeeze (сжатие) - когда BB находятся внутри KC
/// 2. Momentum - направление движения после сжатия
///
/// Source is locked to Close for the MA/BB/momentum path; ATR uses High/Low/Close.
/// Feed lanes: [High, Low, Close].
#[derive(Debug, Clone)]
pub struct SqueezeMomentum {
    bb_period: usize,
    kc_period: usize,
    momentum_period: usize,

    // Bollinger Bands компоненты
    bb_sma: SmootherSlot,
    close_prices: Vec<f64>,
    bb_multiplier: f64,

    // Keltner Channel компоненты
    kc_sma: SmootherSlot,
    atr: Atr,
    kc_multiplier: f64,

    // Momentum компоненты
    momentum_values: Vec<f64>,

    // Текущие значения
    bb_upper: f64,
    bb_lower: f64,
    kc_upper: f64,
    kc_lower: f64,
    momentum: f64,
    is_squeezed: bool,

    // Состояние
    count: usize,
    is_ready: bool,
}

impl SqueezeMomentum {
    /// Default ctor — SMA for both BB and KC smoothers.
    pub fn new(bb_period: usize, kc_period: usize, momentum_period: usize) -> Self {
        Self::from_smoothers(
            SmootherId::Sma, bb_period,
            SmootherId::Sma, kc_period,
            momentum_period,
        )
    }

    /// Build from narrow `SmootherId`s for the BB and KC MA slots.
    /// Legacy bridge; the contract path goes through `SqmomConfig`.
    pub fn from_smoothers(
        bb_id: SmootherId, bb_period: usize,
        kc_id: SmootherId, kc_period: usize,
        momentum_period: usize,
    ) -> Self {
        assert!(bb_period > 0, "BB period must be > 0");
        assert!(kc_period > 0, "KC period must be > 0");
        assert!(momentum_period > 0, "Momentum period must be > 0");

        Self {
            bb_period,
            kc_period,
            momentum_period,
            bb_sma: SmootherSlot::new(bb_id, bb_period),
            close_prices: Vec::with_capacity(bb_period),
            bb_multiplier: 2.0,
            kc_sma: SmootherSlot::new(kc_id, kc_period),
            atr: Atr::new_sma(kc_period),
            kc_multiplier: 1.5,
            momentum_values: Vec::with_capacity(bb_period),
            bb_upper: 0.0,
            bb_lower: 0.0,
            kc_upper: 0.0,
            kc_lower: 0.0,
            momentum: 0.0,
            is_squeezed: false,
            count: 0,
            is_ready: false,
        }
    }

    /// Feed a bar via lanes: `[high, low, close]`.
    pub fn feed(&mut self, lanes: &[f64]) -> (f64, bool) {
        let high  = lanes[0];
        let low   = lanes[1];
        let close = lanes[2];

        // 1. Update Bollinger Bands (Close as source)
        let bb_middle = self.bb_sma.feed(close);

        if self.close_prices.len() >= self.bb_period {
            self.close_prices.remove(0);
        }
        self.close_prices.push(close);

        if self.close_prices.len() >= self.bb_period {
            let std_dev = self.calculate_standard_deviation(&self.close_prices, bb_middle);
            self.bb_upper = bb_middle + self.bb_multiplier * std_dev;
            self.bb_lower = bb_middle - self.bb_multiplier * std_dev;
        }

        // 2. Update Keltner Channel
        let kc_middle = self.kc_sma.feed(close);

        if self.count > 0 {
            let atr_value = self.atr.feed(&[high, low, close]);
            if self.atr.is_ready() {
                self.kc_upper = kc_middle + self.kc_multiplier * atr_value;
                self.kc_lower = kc_middle - self.kc_multiplier * atr_value;
            }
        }

        // 3. Determine squeeze state
        if self.close_prices.len() >= self.bb_period && self.atr.is_ready() {
            self.is_squeezed = self.bb_upper < self.kc_upper && self.bb_lower > self.kc_lower;
        }

        // 4. Calculate Momentum
        if self.close_prices.len() >= self.bb_period {
            let momentum_value = close - bb_middle;

            if self.momentum_values.len() >= self.momentum_period {
                self.momentum_values.remove(0);
            }
            self.momentum_values.push(momentum_value);

            if self.momentum_values.len() >= self.momentum_period {
                self.momentum = self.calculate_linear_regression(&self.momentum_values);
            }
        }

        if self.count >= self.bb_period.max(self.kc_period).max(self.momentum_period) {
            self.is_ready = true;
        }

        self.count += 1;

        (self.momentum, self.is_squeezed)
    }


    fn calculate_standard_deviation(&self, values: &[f64], mean: f64) -> f64 {
        if values.is_empty() {
            return 0.0;
        }
        let variance = values.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / values.len() as f64;
        variance.sqrt()
    }

    fn calculate_linear_regression(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }
        let n = values.len() as f64;
        let sum_x = (0..values.len()).map(|i| i as f64).sum::<f64>();
        let sum_y = values.iter().sum::<f64>();
        let sum_xy = values.iter().enumerate()
            .map(|(i, &y)| i as f64 * y)
            .sum::<f64>();
        let sum_x2 = (0..values.len()).map(|i| (i as f64).powi(2)).sum::<f64>();
        let denominator = n * sum_x2 - sum_x.powi(2);
        if denominator.abs() < 1e-12 {
            return 0.0;
        }
        (n * sum_xy - sum_x * sum_y) / denominator
    }


    /// Named getter for the `squeeze` brace output (squeeze state: 1.0 = squeezed, 0.0 = not).
    pub fn squeeze(&self) -> f64 {
        if self.is_squeezed { 1.0 } else { 0.0 }
    }

    pub fn momentum(&self) -> f64 {
        self.momentum
    }

    pub fn is_squeezed(&self) -> bool {
        self.is_squeezed
    }

    pub fn bollinger_bands(&self) -> (f64, f64) {
        (self.bb_upper, self.bb_lower)
    }

    pub fn keltner_channel(&self) -> (f64, f64) {
        (self.kc_upper, self.kc_lower)
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    pub fn bb_period(&self) -> usize {
        self.bb_period
    }

    pub fn reset(&mut self) {
        self.bb_sma.reset();
        self.close_prices.clear();
        self.kc_sma.reset();
        self.atr.reset();
        self.momentum_values.clear();
        self.bb_upper = 0.0;
        self.bb_lower = 0.0;
        self.kc_upper = 0.0;
        self.kc_lower = 0.0;
        self.momentum = 0.0;
        self.is_squeezed = false;
        self.count = 0;
        self.is_ready = false;
    }

    pub fn trading_signal(&self, prev_squeezed: bool) -> i8 {
        if !self.is_ready {
            return 0;
        }
        if prev_squeezed && !self.is_squeezed {
            if self.momentum > 0.0 {
                return 1;
            } else if self.momentum < 0.0 {
                return -1;
            }
        }
        0
    }

    pub fn market_condition(&self) -> &'static str {
        if !self.is_ready {
            return "Initializing";
        }
        if self.is_squeezed {
            "Squeeze (Low Volatility)"
        } else if self.momentum > 0.0 {
            "Bullish Momentum"
        } else if self.momentum < 0.0 {
            "Bearish Momentum"
        } else {
            "Neutral"
        }
    }

    pub fn momentum_strength(&self) -> f64 {
        if !self.is_ready {
            return 0.0;
        }
        let bb_middle = self.bb_sma.value();
        if bb_middle.abs() < 1e-12 {
            return 0.0;
        }
        (self.momentum.abs() / bb_middle).min(1.0)
    }

    pub fn breakout_potential(&self) -> f64 {
        if !self.is_ready {
            return 0.0;
        }
        if !self.is_squeezed {
            return 0.0;
        }
        self.momentum_strength()
    }

    pub fn info(&self) -> String {
        format!(
            "Squeeze: {}, Momentum: {:.4}, Condition: {}, Strength: {:.3}",
            if self.is_squeezed { "ON" } else { "OFF" },
            self.momentum,
            self.market_condition(),
            self.momentum_strength()
        )
    }
}

impl Default for SqueezeMomentum {
    fn default() -> Self {
        Self::new(20, 20, 20)
    }
}

// -- contract ------------------------------------------------------------------

use crate::engine::indicator_id::IndicatorId;
use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, Slot, SourceAxis, Store, StoreKind, UpdateComplexity};
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, HistogramStyle, RenderOutput, RenderSpec};

/// Typed dual-mode config for [`SqueezeMomentum`].
///
/// BB and KC each carry an independent smoother `#[slot]` (kind + follow/own period)
/// plus their own `period` field. The ATR within KC uses the same smoother kind at
/// `kc_period` (ABSORB: no `AtrConfig`, built via `Atr::new_sma`-equivalent via the
/// resolved `kc_smoother` kind). `momentum_period` drives the linear-regression window.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct SqmomConfig {
    pub bb_period: Param<usize>,
    #[slot]
    pub bb_smoother: Param<SmootherChoice>,
    pub kc_period: Param<usize>,
    #[slot]
    pub kc_smoother: Param<SmootherChoice>,
    pub momentum_period: Param<usize>,
}

impl Indicator for SqueezeMomentum {
    const ID: IndicatorId = IndicatorId::Sqmom;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Reads High, Low, Close: ATR needs H/L/C; BB/KC MA and momentum use Close.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High, OhlcvField::Low, OhlcvField::Close,
    ]));
    const COST: Cost = Cost {
        update: UpdateComplexity::Linear,
        stores: &[Store::window(StoreKind::Vec), Store::window(StoreKind::Vec)],
        inner: &[Port::new(IndicatorId::Atr, &[IndicatorOutputId::Atr])],
    };
    const SLOTS: &'static [Slot] = SqmomConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[
        Output::centered(IndicatorOutputId::SqmomMomentum),
        Output::discrete(IndicatorOutputId::SqmomSqueeze),
    ];
    type Config = SqmomConfig;
    type Runtime = SqueezeMomentum;

    fn create(cfg: SqmomConfig) -> SqueezeMomentum {
        let bb_p = cfg.bb_period.resolved();
        let bb_choice = cfg.bb_smoother.resolved();
        let bb_period = bb_choice.period.resolve(bb_p);
        let kc_p = cfg.kc_period.resolved();
        let kc_choice = cfg.kc_smoother.resolved();
        let kc_period = kc_choice.period.resolve(kc_p);
        SqueezeMomentum::from_smoothers(
            bb_choice.kind, bb_period,
            kc_choice.kind, kc_period,
            cfg.momentum_period.resolved(),
        )
    }

    fn slot_members(cfg: &SqmomConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for SqmomConfig {
    fn defaults() -> Self {
        SqmomConfig {
            bb_period: Param::Solo(20),
            bb_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
            kc_period: Param::Solo(20),
            kc_smoother: Param::Solo(SmootherChoice::follow(SmootherId::Sma)),
            momentum_period: Param::Solo(20),
        }
    }
    fn machine_defaults() -> Self {
        // bb_period, kc_period, momentum_period: Class A usize — auto range(2,4048,1)
        // #[slot] bb_smoother, kc_smoother: leave Solo (deferred sweep wave)
        Self::machine_defaults_auto()
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
}


impl Render for SqueezeMomentum {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .output(RenderOutput::histogram(
                IndicatorOutputId::SqmomMomentum,
                "Squeeze Mom",
                Color::hex(0x4CAF50),
            ))
            .zero_baseline()
            .histogram_style(HistogramStyle::Centered)
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::contract_engine::IndicatorOrder;
    use crate::contract::MarketSample;

    #[test]
    fn test_squeeze_momentum_new() {
        let squeeze = SqueezeMomentum::new(20, 20, 20);
        assert_eq!(squeeze.bb_period(), 20);
        assert!(!squeeze.is_ready());
        assert!(!squeeze.is_squeezed());
    }

    #[test]
    fn test_squeeze_momentum_default() {
        let squeeze = SqueezeMomentum::default();
        assert_eq!(squeeze.bb_period(), 20);
    }

    #[test]
    fn test_squeeze_momentum_calculation() {
        let mut squeeze = SqueezeMomentum::new(10, 10, 10);

        let test_data = vec![
            (100.0, 101.0, 99.0, 100.0),
            (100.0, 101.0, 99.0, 100.5),
            (100.5, 101.5, 99.5, 100.2),
            (100.2, 101.2, 99.2, 100.8),
            (100.8, 101.8, 99.8, 100.3),
            (100.3, 101.3, 99.3, 100.7),
            (100.7, 101.7, 99.7, 100.1),
            (100.1, 101.1, 99.1, 100.9),
            (100.9, 101.9, 99.9, 100.4),
            (100.4, 101.4, 99.4, 100.6),
            (100.6, 103.0, 100.0, 102.5),
            (102.5, 104.0, 101.5, 103.8),
            (103.8, 105.5, 103.0, 105.0),
        ];

        let mut prev_squeezed = false;
        for (open, high, low, close) in test_data {
            let (_momentum, is_squeezed) = squeeze.feed(&[high, low, close]);
            let signal = squeeze.trading_signal(prev_squeezed);
            let _ = (open, signal);
            prev_squeezed = is_squeezed;
        }

        assert!(squeeze.momentum().is_finite());
    }

    #[test]
    fn test_squeeze_momentum_reset() {
        let mut squeeze = SqueezeMomentum::new(10, 10, 10);

        squeeze.feed(&[101.0, 99.0, 100.0]);
        squeeze.feed(&[101.0, 99.0, 100.5]);

        squeeze.reset();

        assert!(!squeeze.is_ready());
        assert!(!squeeze.is_squeezed());
        assert_eq!(squeeze.momentum(), 0.0);
    }

    #[test]
    fn test_squeeze_momentum_from_smoothers() {
        let mut squeeze = SqueezeMomentum::from_smoothers(
            SmootherId::Ema, 10,
            SmootherId::Sma, 10,
            10,
        );

        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            squeeze.feed(&[price + 1.0, price - 1.0, price]);
        }

        assert!(squeeze.is_ready());
    }

    #[test]
    fn factory_feeds_resolved_sqmom() {
        let cfg = <<SqueezeMomentum as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Sqmom(cfg).build_solo().unwrap();
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            // Wild open/volume to prove those fields are NOT used
            f.feed(0, MarketSample::Bar {
                open: 9999.0, high: price + 1.0, low: price - 1.0, close: price, volume: 9999.0,
            });
        }
        assert!(f.read(IndicatorOutputId::SqmomMomentum).is_finite());
    }
}
