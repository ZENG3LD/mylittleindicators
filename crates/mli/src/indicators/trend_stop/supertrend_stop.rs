//! SuperTrend Stop - динамические уровни на основе SuperTrend
//!
//! Вычисляет уровни SuperTrend для создания динамических уровней поддержки/сопротивления.
//! Индикатор НЕ содержит логику стопов — только возвращает уровни SuperTrend.
//! Логика остановки позиций реализуется в стратегиях на основе этих уровней.

use crate::indicators::trend::supertrend::Supertrend;
use crate::engine::contract_engine::SmootherId;

/// SuperTrend Stop индикатор — динамические уровни на основе SuperTrend.
#[derive(Debug, Clone)]
pub struct SuperTrendStop {
    supertrend: Supertrend,
    current_level: f64,
    trend_direction: i8, // 1 = up trend, -1 = down trend
}

impl SuperTrendStop {
    /// Default ctor: period=10, multiplier=3.0, RMA ATR.
    pub fn new() -> Self {
        Self::with_params(10, 3.0)
    }

    /// Build with default RMA ATR smoothing.
    pub fn with_params(period: usize, multiplier: f64) -> Self {
        Self::with_atr_ma_type(period, multiplier, SmootherId::Rma)
    }

    /// Build with configurable ATR smoothing type.
    /// `with_atr_ma_type` is the transitional bridge on the inner `Supertrend` — kept as-is.
    pub fn with_atr_ma_type(period: usize, multiplier: f64, atr_ma_type: SmootherId) -> Self {
        Self {
            supertrend: Supertrend::with_atr_ma_type(period, multiplier, atr_ma_type),
            current_level: 0.0,
            trend_direction: 1,
        }
    }

    /// Feed a bar [high, low, close] (matches `const SOURCE` order: H/L/C).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];

        // Drive the inner Supertrend via its still-present update_bar bridge.
        let supertrend_value = self.supertrend.feed(&[high, low, close]);
        self.current_level = supertrend_value;
        self.trend_direction = self.supertrend.trend_direction();

        self.current_level
    }

    pub fn level(&self) -> f64 { self.current_level }

    /// Contract output: the SuperTrend stop level.
    pub fn value(&self) -> f64 {
        self.current_level
    }

    pub fn trend_direction(&self) -> i8 { self.trend_direction }

    pub fn is_ready(&self) -> bool { self.supertrend.is_ready() }

    pub fn reset(&mut self) {
        self.supertrend.reset();
        self.current_level = 0.0;
        self.trend_direction = 1;
    }
}

impl Default for SuperTrendStop {
    fn default() -> Self { Self::new() }
}

// ── Contract ────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::{IndicatorOutputId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Port, SourceAxis, UpdateComplexity, sweep_f64};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`SuperTrendStop`] — period, ATR multiplier, ATR smoother.
/// ABSORB: builds inner `Supertrend` via `with_atr_ma_type(period, multiplier, atr_order.id())`;
/// no `SupertrendConfig` used.
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct SuperTrendStopConfig {
    pub period: Param<usize>,
    pub multiplier: Param<f64>,
    /// ATR smoother order for the inner Supertrend — default `Rma` at the host `period`.
    #[slot]
    pub atr_smoother: Param<SmootherSlotOrder>,
}

impl Indicator for SuperTrendStop {
    const ID: IndicatorId = IndicatorId::Supts;
    const FAMILY: &'static [Family] = &[Family::Trend];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/C lanes — fed directly into the inner Supertrend ATR.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// O(1) outer wrapper; inner Supertrend cost declared via Port.
    const COST: Cost = Cost {
        update: UpdateComplexity::Constant,
        stores: &[],
        inner: &[Port::new(IndicatorId::Supertrend, &[IndicatorOutputId::Supertrend])],
    };
    const SLOTS: &'static [crate::contract::Slot] = SuperTrendStopConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Supts)];
    type Config = SuperTrendStopConfig;
    type Runtime = SuperTrendStop;

    fn create(cfg: SuperTrendStopConfig) -> SuperTrendStop {
        let period = cfg.period.resolved();
        let multiplier = cfg.multiplier.resolved();
        let atr_order = cfg.atr_smoother.resolved();
        SuperTrendStop::with_atr_ma_type(period, multiplier, atr_order.id())
    }

    fn slot_members(cfg: &SuperTrendStopConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for SuperTrendStopConfig {
    fn defaults() -> Self {
        SuperTrendStopConfig {
            period: Param::Solo(10),
            multiplier: Param::Solo(3.0),
            atr_smoother: Param::Solo(SmootherSlotOrder::Rma(PeriodConfig { period: 10 })),
        }
    }
    fn cube_size(&self) -> u128 {
        self.axes_cube_size()
    }
    fn iter(&self) -> Box<dyn Iterator<Item = Self> + '_> {
        self.axes_iter()
    }
    fn machine_defaults() -> Self {
        let mut s = Self::machine_defaults_auto(); // period→range(2,4048,1)
        s.multiplier = Param::many(sweep_f64(0.1, 10.0, 0.1)); // Class C multiplier
        s
    }
}


impl Render for SuperTrendStop {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Supts, "Support Stop", Color::hex(0x4CAF50))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supertrend_stop_creation() {
        let ind = SuperTrendStop::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_supertrend_stop_warmup() {
        let mut ind = SuperTrendStop::with_params(10, 3.0);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_supertrend_stop_values_finite() {
        let mut ind = SuperTrendStop::with_params(10, 3.0);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let v = ind.feed(&[price + 1.0, price - 1.0, price]);
            assert!(v.is_finite());
        }
    }

    #[test]
    fn test_supertrend_stop_trend_direction() {
        let mut ind = SuperTrendStop::with_params(10, 3.0);
        for i in 0..20 {
            let price = 100.0 + i as f64;
            ind.feed(&[price + 1.0, price - 1.0, price]);
        }
        let dir = ind.trend_direction();
        assert!(dir == 1 || dir == -1);
    }

    #[test]
    fn test_supertrend_stop_reset() {
        let mut ind = SuperTrendStop::with_params(10, 3.0);
        for i in 0..20 {
            ind.feed(&[105.0, 95.0, 100.0 + i as f64]);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_supts() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<SuperTrendStop as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Supts(cfg).build_solo().unwrap();
        for i in 0..20 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 2.0,
                low: price - 2.0,
                close: price,
                volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
