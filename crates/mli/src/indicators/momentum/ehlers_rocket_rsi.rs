//! Ehlers Rocket RSI — momentum-enhanced RSI with velocity tracking.

use crate::engine::contract_engine::{SmootherSlot, SmootherId};

/// Ehlers Rocket RSI — "rocket science" RSI variant.
///
/// Applies price pre-smoothing via EMA(3), then computes Wilder RSI on
/// the smoothed prices. The momentum factor is a smoothed first-derivative
/// of RSI, amplified and clamped to 0-100.
///
/// Output: Single (Rocket RSI 0-100)
#[derive(Debug, Clone)]
pub struct EhlersRocketRsi {
    // Pre-smoothing price slot (hardcoded EMA(3))
    price_smoother: SmootherSlot,
    // Configurable momentum MA slot
    momentum_ma: SmootherSlot,
    // Velocity smoothing slot (hardcoded SMA(5))
    velocity_ma: SmootherSlot,

    // RSI components
    rsi_period: usize,
    gains: Vec<f64>,
    losses: Vec<f64>,
    avg_gain: f64,
    avg_loss: f64,

    // History buffers
    smoothed_prices: Vec<f64>,
    rsi_values: Vec<f64>,

    // Params
    smoothing_factor: f64,

    // Result
    rocket_rsi: f64,

    // State
    is_ready: bool,
    update_count: usize,
}

impl EhlersRocketRsi {
    /// Default: RSI(14), smoothing_factor=0.1, momentum EMA(8).
    pub fn new() -> Self {
        Self::from_smoother(14, 0.1, SmootherId::Ema, 8)
    }

    /// Build with a configurable momentum MA slot.
    pub fn from_smoother(
        rsi_period: usize,
        smoothing_factor: f64,
        momentum_id: SmootherId,
        momentum_period: usize,
    ) -> Self {
        assert!(rsi_period > 0, "RSI period must be greater than 0");
        let sf = smoothing_factor.max(1e-6).min(1.0);
        let mp = momentum_period.max(1);
        Self {
            price_smoother: SmootherSlot::new(SmootherId::Ema, 3),
            momentum_ma: SmootherSlot::new(momentum_id, mp),
            velocity_ma: SmootherSlot::new(SmootherId::Sma, 5),
            rsi_period,
            gains: Vec::with_capacity(64),
            losses: Vec::with_capacity(64),
            avg_gain: 0.0,
            avg_loss: 0.0,
            smoothed_prices: Vec::with_capacity(32),
            rsi_values: Vec::with_capacity(32),
            smoothing_factor: sf,
            rocket_rsi: 50.0,
            is_ready: false,
            update_count: 0,
        }
    }

    /// Feed ONE pre-extracted scalar — the pure core computation.
    pub fn feed(&mut self, value: f64) -> f64 {
        let smoothed_price = self.price_smoother.feed(value);

        if self.smoothed_prices.len() >= 32 {
            self.smoothed_prices.remove(0);
        }
        self.smoothed_prices.push(smoothed_price);

        let regular_rsi = self.calculate_rsi(smoothed_price);
        let rocket_rsi = self.calculate_rocket_rsi(regular_rsi);

        self.rocket_rsi = rocket_rsi;

        if self.gains.len() >= self.rsi_period && self.rsi_values.len() >= 3 {
            self.is_ready = true;
        }

        self.update_count += 1;
        self.rocket_rsi
    }

    fn calculate_rsi(&mut self, smoothed_price: f64) -> f64 {
        if self.smoothed_prices.len() < 2 {
            return 50.0;
        }

        let len = self.smoothed_prices.len();
        let prev_smoothed = self.smoothed_prices[len - 2];
        let change = smoothed_price - prev_smoothed;

        let gain = if change > 0.0 { change } else { 0.0 };
        let loss = if change < 0.0 { -change } else { 0.0 };

        if self.gains.len() >= self.rsi_period {
            self.gains.remove(0);
        }
        self.gains.push(gain);

        if self.losses.len() >= self.rsi_period {
            self.losses.remove(0);
        }
        self.losses.push(loss);

        if self.gains.len() == self.rsi_period {
            if self.avg_gain == 0.0 && self.avg_loss == 0.0 {
                self.avg_gain = self.gains.iter().sum::<f64>() / self.rsi_period as f64;
                self.avg_loss = self.losses.iter().sum::<f64>() / self.rsi_period as f64;
            } else {
                let alpha = 1.0 / self.rsi_period as f64;
                self.avg_gain = alpha * gain + (1.0 - alpha) * self.avg_gain;
                self.avg_loss = alpha * loss + (1.0 - alpha) * self.avg_loss;
            }

            if self.avg_loss == 0.0 {
                return 100.0;
            }

            let rs = self.avg_gain / self.avg_loss;
            return 100.0 - (100.0 / (1.0 + rs));
        }

        50.0
    }

    fn calculate_rocket_rsi(&mut self, regular_rsi: f64) -> f64 {
        if self.rsi_values.len() >= 32 {
            self.rsi_values.remove(0);
        }
        self.rsi_values.push(regular_rsi);

        if self.rsi_values.len() < 3 {
            return regular_rsi;
        }

        let len = self.rsi_values.len();
        let current_rsi = self.rsi_values[len - 1];
        let prev_rsi = self.rsi_values[len - 2];
        let momentum = current_rsi - prev_rsi;

        let smoothed_momentum = self.momentum_ma.feed(momentum);
        let _velocity = self.velocity_ma.feed(momentum);

        let rocket_rsi = regular_rsi + self.smoothing_factor * smoothed_momentum * 10.0;
        rocket_rsi.clamp(0.0, 100.0)
    }

    #[inline]
    pub fn value(&self) -> f64 {
        self.rocket_rsi
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.is_ready
    }

    pub fn reset(&mut self) {
        self.price_smoother.reset();
        self.momentum_ma.reset();
        self.velocity_ma.reset();
        self.gains.clear();
        self.losses.clear();
        self.avg_gain = 0.0;
        self.avg_loss = 0.0;
        self.smoothed_prices.clear();
        self.rsi_values.clear();
        self.rocket_rsi = 50.0;
        self.is_ready = false;
        self.update_count = 0;
    }

    pub fn period(&self) -> usize {
        self.rsi_period
    }
}

use crate::engine::contract_engine::{IndicatorOutputId, SmootherChoice};
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, UpdateComplexity};
use crate::contract::axis::sweep_f64;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::engine::stream_kind::StreamKind;
use crate::contract::Render;
use crate::contract::{Color, RenderSpec};

/// Typed contract config for [`EhlersRocketRsi`].
///
/// Dual-mode: every field is a `Param`. The `momentum` smoother slot follows
/// `momentum_period` by default (`follow(Ema)` = EMA at `momentum_period`).
/// The inline RSI has no smoother slot — it runs its own fixed Wilder math.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct EhlersRocketRsiConfig {
    pub source: Param<OhlcvField>,
    pub rsi_period: Param<usize>,
    pub smoothing_factor: Param<f64>,
    /// Period for the configurable momentum MA slot.
    pub momentum_period: Param<usize>,
    /// Configurable momentum MA slot — default `follow(Ema)` (EMA at `momentum_period`).
    #[slot]
    pub momentum: Param<SmootherChoice>,
}

impl Indicator for EhlersRocketRsi {
    const ID: IndicatorId = IndicatorId::EhlersRocket;
    const FAMILY: &'static [Family] = &[Family::Oscillator];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Configurable single price field (default close).
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::Field { default: OhlcvField::Close });
    const COST: Cost = Cost::new(UpdateComplexity::Constant, &[]);
    const SLOTS: &'static [Slot] = EhlersRocketRsiConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::percent(IndicatorOutputId::EhlersRocket)];
    type Config = EhlersRocketRsiConfig;
    type Runtime = EhlersRocketRsi;

    fn create(cfg: EhlersRocketRsiConfig) -> EhlersRocketRsi {
        let rsi_period = cfg.rsi_period.resolved();
        let smoothing_factor = cfg.smoothing_factor.resolved();
        let momentum_period = cfg.momentum_period.resolved().max(1);
        let choice = cfg.momentum.resolved();
        // Honor the slot's Follow/Own period: Follow -> momentum_period, Own -> its own.
        EhlersRocketRsi::from_smoother(rsi_period, smoothing_factor, choice.kind, choice.period.resolve(momentum_period))
    }

    fn source_fields(cfg: &EhlersRocketRsiConfig) -> arrayvec::ArrayVec<OhlcvField, 8> {
        [cfg.source.resolved()].into_iter().collect()
    }

    fn slot_members(cfg: &EhlersRocketRsiConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for EhlersRocketRsiConfig {
    fn defaults() -> Self {
        EhlersRocketRsiConfig {
            source: Param::Solo(OhlcvField::Close),
            rsi_period: Param::Solo(14),
            smoothing_factor: Param::Solo(0.1),
            momentum_period: Param::Solo(8),
            momentum: Param::Solo(SmootherChoice::follow(SmootherId::Ema)),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        // rsi_period/momentum_period: Class A → auto range(2,4048,1); source: Class O → auto all-8.
        // smoothing_factor: Class E (EMA-style decay) — sweep_f64(0.01,0.99,0.01).
        // momentum: #[slot] SmootherChoice → left Solo (deferred wave).
        let mut s = Self::machine_defaults_auto();
        s.smoothing_factor = Param::many(sweep_f64(0.01, 0.99, 0.01));
        s
    }
}


impl Render for EhlersRocketRsi {
    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .sub_pane()
            .line_output(IndicatorOutputId::EhlersRocket, "Rocket RSI", Color::hex(0xFF9800))
            .bounds(0.0, 100.0)
            .zero_baseline()
            .precision(4)
            .build()
    }
}

impl Default for EhlersRocketRsi {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rocket_rsi_creation() {
        let r = EhlersRocketRsi::new();
        assert!(!r.is_ready());
        assert_eq!(r.period(), 14);
    }

    #[test]
    fn test_rocket_rsi_update() {
        let mut r = EhlersRocketRsi::new();
        for i in 0..25 {
            let price = 100.0 + i as f64 * 0.5;
            let val = r.feed(price);
            if i > 15 {
                assert!(r.is_ready());
                assert!(val >= 0.0 && val <= 100.0);
            }
        }
    }

    #[test]
    fn test_rocket_rsi_reset() {
        let mut r = EhlersRocketRsi::new();
        for i in 0..30 {
            r.feed(100.0 + i as f64);
        }
        assert!(r.is_ready());
        r.reset();
        assert!(!r.is_ready());
        assert!((r.value() - 50.0).abs() < 1e-10);
    }

    #[test]
    fn test_rocket_rsi_finite() {
        let mut r = EhlersRocketRsi::new();
        for i in 1..=50 {
            let price = 100.0 + (i as f64 * 0.3).sin() * 20.0;
            let v = r.feed(price);
            assert!(v.is_finite());
        }
    }

    #[test]
    fn factory_feeds_resolved_ehlers_rocket() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<EhlersRocketRsi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::EhlersRocket(cfg).build_solo().unwrap();
        for i in 1..=30 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: 9999.0,
                low: 9999.0,
                close: price,
                volume: 9999.0,
            });
        }
        assert!(f.primary().is_finite());
    }

    #[test]
    fn config_dual_mode() {
        use crate::contract::{Config};
        let d = <<EhlersRocketRsi as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        assert_eq!(d.cube_size(), 1);
        assert_eq!(d.iter().count(), 1);

        let swept = EhlersRocketRsiConfig {
            source: Param::Solo(OhlcvField::Close),
            rsi_period: Param::range(7, 21, 7),
            smoothing_factor: Param::Solo(0.1),
            momentum_period: Param::range(4, 12, 4),
            momentum: Param::many(vec![
                SmootherChoice::follow(SmootherId::Ema),
                SmootherChoice::follow(SmootherId::Sma),
            ]),
        };
        // 3 rsi_period × 3 momentum_period × 2 smoothers = 18
        assert_eq!(swept.cube_size(), 18);
    }
}
