//! Volatility Stop - адаптивные уровни на основе волатильности
//!
//! Вычисляет адаптивные уровни на основе различных мер волатильности:
//! - StandardDeviation — стандартное отклонение цен закрытия
//! - ATR — средний истинный диапазон
//! - Range — простой диапазон High-Low
//!
//! Индикатор НЕ содержит логику стопов — только возвращает адаптивные уровни.
//! Контрактный выход — уровень для лонг позиций (long_stop = MA_price − vol × mult).

use crate::indicators::volatility::atr::Atr;
use crate::engine::contract_engine::{SmootherId, SmootherSlot};

/// Тип меры волатильности для расчёта уровней.
///
/// Это обычный config-enum (не MA-тип) — диспетчер алгоритма расчёта волатильности.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[derive(mli_contract_macros::ParamScalar)]
pub enum VolatilityType {
    /// Стандартное отклонение цен закрытия.
    StandardDeviation,
    /// Average True Range.
    Atr,
    /// Простой диапазон High-Low.
    Range,
}

/// Volatility Stop индикатор — адаптивные уровни на основе волатильности.
#[derive(Debug, Clone)]
pub struct VolatilityStop {
    period: usize,
    multiplier: f64,
    volatility_type: VolatilityType,

    // Smoothers
    atr: Option<Atr>,
    price_ma: SmootherSlot,

    // Buffers for SD / Range modes
    closes: Vec<f64>,
    ranges: Vec<f64>,

    // Current values
    current_price: f64,
    current_volatility: f64,
    long_stop: f64,
    short_stop: f64,

    // State
    bars_count: usize,
    is_ready: bool,
}

impl VolatilityStop {
    /// Default ctor: period=20, multiplier=2.0, StandardDeviation, SMA.
    pub fn new() -> Self {
        Self::from_smoother(SmootherId::Sma, 20, 2.0, VolatilityType::StandardDeviation)
    }

    /// Build from a narrow `SmootherId`.
    /// Legacy bridge; the contract path goes through `VolatilityStopConfig`.
    pub fn from_smoother(
        ma_id: SmootherId,
        period: usize,
        multiplier: f64,
        volatility_type: VolatilityType,
    ) -> Self {
        let p = period.max(1);
        let atr = if volatility_type == VolatilityType::Atr {
            Some(Atr::new_wilder(p))
        } else {
            None
        };
        Self {
            period: p,
            multiplier,
            volatility_type,
            atr,
            price_ma: SmootherSlot::new(ma_id, p),
            closes: Vec::with_capacity(512),
            ranges: Vec::with_capacity(512),
            current_price: 0.0,
            current_volatility: 0.0,
            long_stop: 0.0,
            short_stop: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    /// Feed a bar [high, low, close] (matches `const SOURCE` order: H/L/C).
    pub fn feed(&mut self, lanes: &[f64]) -> f64 {
        let high = lanes[0];
        let low = lanes[1];
        let close = lanes[2];

        self.bars_count += 1;

        // Center price: MA of typical price HLC/3
        let typical_price = (high + low + close) / 3.0;
        self.current_price = self.price_ma.feed(typical_price);

        // Volatility measure
        self.current_volatility = match self.volatility_type {
            VolatilityType::Atr => {
                if let Some(ref mut atr) = self.atr {
                    atr.feed(&[high, low, close])
                } else {
                    0.0
                }
            }
            VolatilityType::StandardDeviation => {
                if self.closes.len() >= self.period {
                    self.closes.remove(0);
                }
                self.closes.push(close);
                if self.closes.len() >= 2 {
                    self.standard_deviation()
                } else {
                    0.0
                }
            }
            VolatilityType::Range => {
                let range = high - low;
                if self.ranges.len() >= self.period {
                    self.ranges.remove(0);
                }
                self.ranges.push(range);
                if !self.ranges.is_empty() {
                    self.ranges.iter().sum::<f64>() / self.ranges.len() as f64
                } else {
                    0.0
                }
            }
        };

        // Adaptive levels
        let offset = self.current_volatility * self.multiplier;
        self.long_stop = self.current_price - offset;
        self.short_stop = self.current_price + offset;

        self.is_ready = self.bars_count >= self.period && self.current_volatility > 0.0;

        self.long_stop
    }

    fn standard_deviation(&self) -> f64 {
        if self.closes.len() < 2 {
            return 0.0;
        }
        let mean = self.closes.iter().sum::<f64>() / self.closes.len() as f64;
        let variance = self.closes.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>()
            / self.closes.len() as f64;
        variance.sqrt()
    }

    pub fn long_stop(&self) -> f64 { self.long_stop }
    pub fn short_stop(&self) -> f64 { self.short_stop }

    /// Contract output: the long stop level.
    pub fn value(&self) -> f64 {
        self.long_stop
    }

    pub fn is_ready(&self) -> bool { self.is_ready }

    pub fn reset(&mut self) {
        if let Some(ref mut atr) = self.atr {
            atr.reset();
        }
        self.price_ma.reset();
        self.closes.clear();
        self.ranges.clear();
        self.current_price = 0.0;
        self.current_volatility = 0.0;
        self.long_stop = 0.0;
        self.short_stop = 0.0;
        self.bars_count = 0;
        self.is_ready = false;
    }
}

impl Default for VolatilityStop {
    fn default() -> Self { Self::new() }
}

// ── Contract ────────────────────────────────────────────────────────────────

use crate::engine::contract_engine::{IndicatorOutputId, SmootherSlotOrder};
use crate::indicators::average::moving_average::PeriodConfig;
use crate::engine::indicator_id::IndicatorId;
use crate::engine::ohlcv_field::OhlcvField;
use crate::contract::{Cost, Family, Indicator, Output, Param, Slot, SourceAxis, Store, StoreKind, UpdateComplexity, sweep_f64};
use crate::contract::{Color, Render, RenderSpec};
use crate::engine::stream_kind::StreamKind;

/// Typed dual-mode config for [`VolatilityStop`] — period, multiplier, volatility algorithm,
/// center-MA slot, and ATR smoother slot (active only in `VolatilityType::Atr` mode).
#[derive(Debug, Clone, mli_contract_macros::Slots, mli_contract_macros::ConfigAxes)]
pub struct VolatilityStopConfig {
    pub period: Param<usize>,
    pub multiplier: Param<f64>,
    /// Algorithm used to measure volatility (SD / ATR / Range). NOT an MA type.
    pub volatility_type: Param<VolatilityType>,
    /// Center-price MA smoother order — default `Sma` at the host `period`.
    #[slot]
    pub ma_smoother: Param<SmootherSlotOrder>,
    /// ATR smoother order — used only when `volatility_type == Atr`, default `Rma` at the host `period`.
    #[slot]
    pub atr_smoother: Param<SmootherSlotOrder>,
}

impl Indicator for VolatilityStop {
    const ID: IndicatorId = IndicatorId::Volts;
    const FAMILY: &'static [Family] = &[];
    const INPUT: &'static [StreamKind] = &[StreamKind::Bar];
    /// Fixed H/L/C lanes — needed for ATR and Range modes.
    const SOURCE: Option<SourceAxis> = Some(SourceAxis::KlineSlice(&[
        OhlcvField::High,
        OhlcvField::Low,
        OhlcvField::Close,
    ]));
    /// SD / Range modes keep a rolling window of closes or ranges.
    const COST: Cost = Cost::new(
        UpdateComplexity::Linear,
        &[Store::window(StoreKind::Vec)],
    );
    const SLOTS: &'static [Slot] = VolatilityStopConfig::SLOTS;
    const OUTPUTS: &'static [Output] = &[Output::price(IndicatorOutputId::Volts)];
    type Config = VolatilityStopConfig;
    type Runtime = VolatilityStop;

    fn create(cfg: VolatilityStopConfig) -> VolatilityStop {
        let p = cfg.period.resolved().max(1);
        let vol_type = cfg.volatility_type.resolved();
        let ma_order = cfg.ma_smoother.resolved();
        let atr_order = cfg.atr_smoother.resolved();
        let atr = if vol_type == VolatilityType::Atr {
            Some(Atr::from_smoother(atr_order.period(), atr_order.id()))
        } else {
            None
        };
        VolatilityStop {
            period: p,
            multiplier: cfg.multiplier.resolved(),
            volatility_type: vol_type,
            atr,
            price_ma: ma_order.into_slot(),
            closes: Vec::with_capacity(512),
            ranges: Vec::with_capacity(512),
            current_price: 0.0,
            current_volatility: 0.0,
            long_stop: 0.0,
            short_stop: 0.0,
            bars_count: 0,
            is_ready: false,
        }
    }

    fn slot_members(cfg: &VolatilityStopConfig) -> Vec<IndicatorId> {
        cfg.slot_members()
    }
}

impl crate::contract::Config for VolatilityStopConfig {
    fn defaults() -> Self {
        VolatilityStopConfig {
            period: Param::Solo(20),
            multiplier: Param::Solo(2.0),
            volatility_type: Param::Solo(VolatilityType::StandardDeviation),
            ma_smoother: Param::Solo(SmootherSlotOrder::Sma(PeriodConfig { period: 20 })),
            atr_smoother: Param::Solo(SmootherSlotOrder::Rma(PeriodConfig { period: 20 })),
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
        s.volatility_type = Param::many(vec![
            VolatilityType::StandardDeviation,
            VolatilityType::Atr,
            VolatilityType::Range,
        ]); // Class Q enum — all variants
        s
    }
}


impl Render for VolatilityStop {

    fn rendering() -> RenderSpec {
        RenderSpec::builder(Self::ID)
            .overlay()
            .line_output(IndicatorOutputId::Volts, "Vol Trail Stop", Color::hex(0xFF9800))
            .precision(4)
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volatility_stop_creation() {
        let ind = VolatilityStop::new();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn test_volatility_stop_warmup() {
        let mut ind = VolatilityStop::from_smoother(SmootherId::Sma, 10, 2.0, VolatilityType::StandardDeviation);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.1).sin() * 5.0;
            ind.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_volatility_stop_atr_type() {
        let mut ind = VolatilityStop::from_smoother(SmootherId::Sma, 10, 2.0, VolatilityType::Atr);
        for i in 0..15 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            ind.feed(&[price + 1.0, price - 1.0, price]);
        }
        assert!(ind.is_ready());
    }

    #[test]
    fn test_volatility_stop_values_finite() {
        let mut ind = VolatilityStop::from_smoother(SmootherId::Sma, 10, 2.0, VolatilityType::StandardDeviation);
        for i in 0..25 {
            let price = 100.0 + (i as f64 * 0.2).sin() * 10.0;
            let v = ind.feed(&[price + 1.0, price - 1.0, price]);
            assert!(v.is_finite());
        }
    }

    #[test]
    fn test_volatility_stop_reset() {
        let mut ind = VolatilityStop::from_smoother(SmootherId::Sma, 10, 2.0, VolatilityType::StandardDeviation);
        for i in 0..20 {
            ind.feed(&[105.0, 95.0, 100.0 + i as f64]);
        }
        ind.reset();
        assert!(!ind.is_ready());
        assert_eq!(ind.value(), 0.0);
    }

    #[test]
    fn factory_feeds_resolved_volts() {
        use crate::engine::contract_engine::IndicatorOrder;
        use crate::contract::MarketSample;
        let cfg = <<VolatilityStop as crate::contract::Indicator>::Config as crate::contract::Config>::defaults();
        let mut f = IndicatorOrder::Volts(cfg).build_solo().unwrap();
        for i in 0..30 {
            let price = 100.0 + i as f64;
            f.feed(0, MarketSample::Bar {
                open: 9999.0,
                high: price + 1.0,
                low: price - 1.0,
                close: price,
                volume: 1000.0,
            });
        }
        assert!(f.primary().is_finite());
    }
}
